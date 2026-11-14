//! A browser-neutral, failure-only startup leaf for an explicitly owned process.
//!
//! This crate does not discover executables, establish a browser protocol, or
//! return a usable browser. A child still running after the bounded probe is
//! killed and reaped before the caller learns that full launch is unsupported.

use std::io;
use std::path::Path;
use std::process::{ExitStatus, Stdio};
use std::time::Duration;

use tokio::process::Command;

pub const MAX_STARTUP_PROBE: Duration = Duration::from_secs(2);
pub const MAX_STARTUP_OUTPUT_BYTES: usize = 3 * 1024;

#[derive(Debug)]
pub enum StartupObservation {
    ExitedBeforeReady(ExitStatus),
    RunningWithoutProtocol,
}

/// A bounded diagnostic from an explicitly owned, browser-neutral launch.
///
/// The combined size of `stdout` and `stderr` never exceeds
/// [`MAX_STARTUP_OUTPUT_BYTES`]. The caller owns any user-facing log formatting.
#[derive(Debug)]
pub struct StartupReport {
    pub outcome: StartupObservation,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Spawn only the caller's explicit executable and observe whether it exits.
///
/// `requested_wait == Duration::ZERO` uses the two-second safety cap. The
/// direct child is killed on cancellation and explicitly reaped on timeout.
pub async fn observe_explicit_exit(
    executable: &Path,
    arguments: &[String],
    requested_wait: Duration,
) -> io::Result<StartupObservation> {
    if !executable.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "startup executable path must be absolute",
        ));
    }
    if !executable.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "startup executable is not a regular file",
        ));
    }

    let mut command = Command::new(executable);
    command
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let mut child = command.spawn()?;
    let wait = if requested_wait.is_zero() {
        MAX_STARTUP_PROBE
    } else {
        requested_wait.min(MAX_STARTUP_PROBE)
    };
    match tokio::time::timeout(wait, child.wait()).await {
        Ok(Ok(status)) => Ok(StartupObservation::ExitedBeforeReady(status)),
        Ok(Err(error)) => Err(error),
        Err(_) => {
            child.kill().await?;
            Ok(StartupObservation::RunningWithoutProtocol)
        }
    }
}

/// Observe an explicitly selected executable without establishing a protocol.
///
/// The child starts in its own process group on macOS and Linux. The group is
/// killed before the direct child is reaped on exit, timeout, and cancellation;
/// dropping this future waits for the supervisor to finish that cleanup.
/// A zero wait uses [`MAX_STARTUP_PROBE`], and every wait is capped by it.
/// Other platforms return `Unsupported` before any process is started.
pub async fn observe_explicit_startup(
    executable: &Path,
    arguments: &[String],
    requested_wait: Duration,
) -> io::Result<StartupReport> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        observed_launch::observe(executable, arguments, requested_wait).await
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = (executable, arguments, requested_wait);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "process-group startup observation is only supported on macOS and Linux",
        ))
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod observed_launch {
    use super::{MAX_STARTUP_OUTPUT_BYTES, MAX_STARTUP_PROBE, StartupObservation, StartupReport};
    use std::io::{self, Read};
    use std::os::fd::AsRawFd;
    use std::os::unix::process::CommandExt;
    use std::path::Path;
    use std::process::{Child, ChildStderr, ChildStdout, Command, ExitStatus, Stdio};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    const POLL_INTERVAL: Duration = Duration::from_millis(5);
    const OUTPUT_DRAIN_GRACE: Duration = Duration::from_millis(100);

    struct CancellationGuard {
        cancellation: Option<Arc<AtomicBool>>,
        worker: Option<std::thread::JoinHandle<()>>,
    }

    impl CancellationGuard {
        fn disarm(&mut self) {
            self.cancellation = None;
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
        }
    }

    impl Drop for CancellationGuard {
        fn drop(&mut self) {
            if let Some(flag) = self.cancellation.take() {
                flag.store(true, Ordering::Release);
            }
            // Cancellation is a cleanup barrier: callers may now drop an
            // owned temporary profile without racing a live child process.
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
        }
    }

    struct OwnedProcessGroup {
        child: Child,
        group_id: libc::pid_t,
        reaped: bool,
    }

    impl OwnedProcessGroup {
        fn new(mut child: Child) -> io::Result<Self> {
            let group_id = match libc::pid_t::try_from(child.id()) {
                Ok(group_id) => group_id,
                Err(_) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "spawned process ID is out of range",
                    ));
                }
            };
            Ok(Self {
                child,
                group_id,
                reaped: false,
            })
        }

        fn kill_group(&self) -> io::Result<()> {
            // SAFETY: process_group(0) made this positive PID the unique group ID.
            let result = unsafe { libc::kill(-self.group_id, libc::SIGKILL) };
            if result == 0 {
                return Ok(());
            }
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::ESRCH) {
                Ok(())
            } else {
                Err(error)
            }
        }

        fn kill_and_reap(&mut self) -> io::Result<ExitStatus> {
            // A waitid(WNOWAIT) observation leaves the leader PID reserved here.
            let kill_result = self.kill_group();
            // Also signal the direct child if it changed groups after spawn.
            let _ = self.child.kill();
            let wait_result = self.child.wait();
            if wait_result.is_ok() {
                self.reaped = true;
            }
            if let Err(error) = kill_result {
                // macOS reports EPERM when only the already-exited group
                // leader remained as a zombie. Accept that case only after
                // reaping the leader and verifying the group disappeared.
                if error.raw_os_error() != Some(libc::EPERM) || !self.group_is_gone()? {
                    return Err(error);
                }
            }
            wait_result
        }

        fn group_is_gone(&self) -> io::Result<bool> {
            // SAFETY: a negative positive PGID probes the owned group.
            let result = unsafe { libc::kill(-self.group_id, 0) };
            if result == 0 {
                return Ok(false);
            }
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::ESRCH) {
                Ok(true)
            } else if error.raw_os_error() == Some(libc::EPERM) {
                Ok(false)
            } else {
                Err(error)
            }
        }

        fn exited_without_reaping(&self) -> io::Result<bool> {
            // SAFETY: `group_id` is the live or zombie direct child. WNOWAIT
            // leaves its PID reserved until kill_and_reap has killed the group.
            let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
            let result = unsafe {
                libc::waitid(
                    libc::P_PID,
                    self.group_id as libc::id_t,
                    &mut info,
                    libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
                )
            };
            if result == 0 {
                // SAFETY: waitid initialized the siginfo for a reported event.
                let reported_pid = unsafe { info.si_pid() };
                return Ok(reported_pid == self.group_id);
            }
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::EINTR) {
                return Ok(false);
            }
            Err(error)
        }
    }

    impl Drop for OwnedProcessGroup {
        fn drop(&mut self) {
            if !self.reaped {
                let _ = self.kill_group();
                let _ = self.child.kill();
                let _ = self.child.wait();
            }
        }
    }

    struct OutputPipes {
        stdout: Option<ChildStdout>,
        stderr: Option<ChildStderr>,
        stdout_bytes: Vec<u8>,
        stderr_bytes: Vec<u8>,
    }

    impl OutputPipes {
        fn new(child: &mut Child) -> io::Result<Self> {
            let stdout = child.stdout.take().ok_or_else(|| {
                io::Error::new(io::ErrorKind::BrokenPipe, "spawned stdout pipe is absent")
            })?;
            let stderr = child.stderr.take().ok_or_else(|| {
                io::Error::new(io::ErrorKind::BrokenPipe, "spawned stderr pipe is absent")
            })?;
            set_nonblocking(&stdout)?;
            set_nonblocking(&stderr)?;
            Ok(Self {
                stdout: Some(stdout),
                stderr: Some(stderr),
                stdout_bytes: Vec::new(),
                stderr_bytes: Vec::new(),
            })
        }

        fn poll(&mut self) -> io::Result<bool> {
            let mut remaining = MAX_STARTUP_OUTPUT_BYTES
                .saturating_sub(self.stdout_bytes.len() + self.stderr_bytes.len());
            let stdout_progress =
                poll_pipe(&mut self.stdout, &mut self.stdout_bytes, &mut remaining)?;
            let stderr_progress =
                poll_pipe(&mut self.stderr, &mut self.stderr_bytes, &mut remaining)?;
            Ok(stdout_progress || stderr_progress)
        }

        fn drain_after_cleanup(&mut self) -> io::Result<()> {
            let deadline = Instant::now() + OUTPUT_DRAIN_GRACE;
            while (self.stdout.is_some() || self.stderr.is_some()) && Instant::now() < deadline {
                if !self.poll()? {
                    std::thread::sleep(POLL_INTERVAL);
                }
            }
            Ok(())
        }

        fn report(self, outcome: StartupObservation) -> StartupReport {
            StartupReport {
                outcome,
                stdout: self.stdout_bytes,
                stderr: self.stderr_bytes,
            }
        }
    }

    fn set_nonblocking(pipe: &impl AsRawFd) -> io::Result<()> {
        let fd = pipe.as_raw_fd();
        // SAFETY: this FD is an owned pipe end and remains open for this call.
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: the pipe remains owned and `flags` came from F_GETFL.
        if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    fn poll_pipe<P: Read>(
        pipe: &mut Option<P>,
        output: &mut Vec<u8>,
        remaining: &mut usize,
    ) -> io::Result<bool> {
        let Some(reader) = pipe.as_mut() else {
            return Ok(false);
        };
        let mut chunk = [0_u8; 4096];
        match reader.read(&mut chunk) {
            Ok(0) => {
                *pipe = None;
                Ok(true)
            }
            Ok(count) => {
                let saved = count.min(*remaining);
                output.extend_from_slice(&chunk[..saved]);
                *remaining -= saved;
                Ok(true)
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(false),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => Ok(false),
            Err(error) => Err(error),
        }
    }

    fn supervise(
        mut owned: OwnedProcessGroup,
        cancellation: Arc<AtomicBool>,
        wait: Duration,
    ) -> io::Result<StartupReport> {
        let mut pipes = OutputPipes::new(&mut owned.child)?;
        let deadline = Instant::now() + wait;
        loop {
            if cancellation.load(Ordering::Acquire) {
                owned.kill_and_reap()?;
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "startup observation cancelled",
                ));
            }
            pipes.poll()?;
            if owned.exited_without_reaping()? {
                let status = owned.kill_and_reap()?;
                pipes.drain_after_cleanup()?;
                return Ok(pipes.report(StartupObservation::ExitedBeforeReady(status)));
            }
            if Instant::now() >= deadline {
                owned.kill_and_reap()?;
                pipes.drain_after_cleanup()?;
                return Ok(pipes.report(StartupObservation::RunningWithoutProtocol));
            }
            std::thread::sleep(
                POLL_INTERVAL.min(deadline.saturating_duration_since(Instant::now())),
            );
        }
    }

    pub(super) async fn observe(
        executable: &Path,
        arguments: &[String],
        requested_wait: Duration,
    ) -> io::Result<StartupReport> {
        if !executable.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "startup executable path must be absolute",
            ));
        }
        if !executable.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "startup executable is not a regular file",
            ));
        }
        let wait = if requested_wait.is_zero() {
            MAX_STARTUP_PROBE
        } else {
            requested_wait.min(MAX_STARTUP_PROBE)
        };
        let mut command = Command::new(executable);
        command
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0);
        let child = command.spawn()?;
        let owned = OwnedProcessGroup::new(child)?;
        let cancellation = Arc::new(AtomicBool::new(false));
        let worker_cancellation = Arc::clone(&cancellation);
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let worker = std::thread::Builder::new()
            .name("pwright-owned-startup".to_owned())
            .spawn(move || {
                let result = supervise(owned, worker_cancellation, wait);
                let _ = sender.send(result);
            })?;
        let mut guard = CancellationGuard {
            cancellation: Some(cancellation),
            worker: Some(worker),
        };
        let result = receiver.await.map_err(|_| {
            io::Error::new(
                io::ErrorKind::BrokenPipe,
                "startup supervisor exited without a result",
            )
        });
        guard.disarm();
        result?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    async fn group_disappears(group_id: libc::pid_t) -> bool {
        for _ in 0..100 {
            // SAFETY: a negative positive PGID probes the entire owned group.
            let result = unsafe { libc::kill(-group_id, 0) };
            if result < 0 && io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        false
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[tokio::test]
    async fn captured_failure_reports_exit_and_bounded_logs() {
        // Arrange
        let executable = Path::new("/bin/sh");
        let arguments = [
            "-c".to_owned(),
            "printf 'stdout text'; printf 'stderr text' >&2; exit 7".to_owned(),
        ];

        // Act
        let report = observe_explicit_startup(executable, &arguments, Duration::from_millis(200))
            .await
            .expect("the owned shell must launch");

        // Assert
        assert!(matches!(
            report.outcome,
            StartupObservation::ExitedBeforeReady(status) if status.code() == Some(7)
        ));
        assert_eq!(report.stdout, b"stdout text");
        assert_eq!(report.stderr, b"stderr text");
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[tokio::test]
    async fn output_from_both_pipes_stays_within_combined_cap() {
        // Arrange
        let executable = Path::new("/bin/sh");
        let payload = "x".repeat(MAX_STARTUP_OUTPUT_BYTES + 4096);
        let arguments = [
            "-c".to_owned(),
            "printf %s \"$1\"; printf %s \"$1\" >&2; exit 1".to_owned(),
            "sh".to_owned(),
            payload,
        ];

        // Act
        let report = observe_explicit_startup(executable, &arguments, Duration::from_secs(1))
            .await
            .expect("the owned shell must launch");

        // Assert
        assert!(matches!(
            report.outcome,
            StartupObservation::ExitedBeforeReady(status) if status.code() == Some(1)
        ));
        assert_eq!(
            report.stdout.len() + report.stderr.len(),
            MAX_STARTUP_OUTPUT_BYTES
        );
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[tokio::test]
    async fn timeout_kills_owned_group_and_reaps_direct_child() {
        // Arrange
        let executable = Path::new("/bin/sh");
        let arguments = [
            "-c".to_owned(),
            "printf '%s' \"$$\"; sleep 30 & wait".to_owned(),
        ];

        // Act
        let report = observe_explicit_startup(executable, &arguments, Duration::from_millis(100))
            .await
            .expect("the owned shell must launch");

        // Assert
        assert!(matches!(
            report.outcome,
            StartupObservation::RunningWithoutProtocol
        ));
        let group_id: libc::pid_t = String::from_utf8(report.stdout)
            .expect("the shell must print its PID")
            .parse()
            .expect("the shell must print a numeric PID");
        assert!(group_disappears(group_id).await);
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[tokio::test]
    async fn early_exit_kills_remaining_owned_group_members() {
        // Arrange
        let executable = Path::new("/bin/sh");
        let arguments = [
            "-c".to_owned(),
            "sleep 30 & printf '%s' \"$$\"; exit 9".to_owned(),
        ];

        // Act
        let report = observe_explicit_startup(executable, &arguments, Duration::from_millis(200))
            .await
            .expect("the owned shell must launch");

        // Assert
        assert!(matches!(
            report.outcome,
            StartupObservation::ExitedBeforeReady(status) if status.code() == Some(9)
        ));
        let group_id: libc::pid_t = String::from_utf8(report.stdout)
            .expect("the shell must print its PID")
            .parse()
            .expect("the shell must print a numeric PID");
        assert!(group_disappears(group_id).await);
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[tokio::test]
    async fn cancellation_kills_owned_group_after_future_is_dropped() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_TEST_FILE: AtomicU64 = AtomicU64::new(1);

        // Arrange
        let path = std::env::temp_dir().join(format!(
            "pwright-owned-startup-{}-{}.pid",
            std::process::id(),
            NEXT_TEST_FILE.fetch_add(1, Ordering::Relaxed)
        ));
        let _owned_file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .expect("the test PID file must be uniquely created");
        let arguments = [
            "-c".to_owned(),
            "printf '%s' \"$$\" > \"$1\"; sleep 30 & wait".to_owned(),
            "sh".to_owned(),
            path.to_string_lossy().into_owned(),
        ];
        let task = tokio::spawn(async move {
            observe_explicit_startup(Path::new("/bin/sh"), &arguments, Duration::from_secs(2)).await
        });
        let published_pid = tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                let contents =
                    std::fs::read_to_string(&path).expect("the test PID file must exist");
                if let Ok(pid) = contents.parse() {
                    break pid;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await;
        let group_id: libc::pid_t = match published_pid {
            Ok(pid) => pid,
            Err(_) => {
                task.abort();
                let _ = task.await;
                std::fs::remove_file(&path).expect("only the test-created PID file is removed");
                panic!("the shell must publish its PID promptly");
            }
        };

        // Act
        task.abort();
        let cancelled = task.await;

        // Assert
        assert!(
            cancelled
                .expect_err("task must be cancelled")
                .is_cancelled()
        );
        let gone = group_disappears(group_id).await;
        std::fs::remove_file(&path).expect("only the test-created PID file is removed");
        assert!(gone);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn reports_nonzero_exit_from_owned_process() {
        // Arrange
        let executable = Path::new("/usr/bin/false");

        // Act
        let observed = observe_explicit_exit(executable, &[], Duration::from_millis(200))
            .await
            .expect("the fixture executable must launch");

        // Assert
        assert!(matches!(
            observed,
            StartupObservation::ExitedBeforeReady(status) if status.code() == Some(1)
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn reaps_process_that_outlives_bounded_probe() {
        // Arrange
        let executable = Path::new("/bin/sleep");

        // Act
        let observed =
            observe_explicit_exit(executable, &["5".to_owned()], Duration::from_millis(20))
                .await
                .expect("the fixture executable must launch");

        // Assert
        assert!(matches!(
            observed,
            StartupObservation::RunningWithoutProtocol
        ));
    }

    #[tokio::test]
    async fn rejects_non_absolute_path_without_launching() {
        // Arrange
        let executable = Path::new("dummy-browser");

        // Act
        let error = observe_explicit_exit(executable, &[], Duration::from_millis(20))
            .await
            .expect_err("relative path must be rejected");

        // Assert
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }
}
