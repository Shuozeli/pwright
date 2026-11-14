//! A one-shot foreign-result lease for newly attached CDP children.

use pwright_node_host::HostError;

pub struct ClaimLease {
    token: u64,
    cleanup: Option<Box<dyn FnOnce() + Send + Sync>>,
}

impl ClaimLease {
    pub fn new(token: u64, cleanup: impl FnOnce() + Send + Sync + 'static) -> Self {
        Self {
            token,
            cleanup: Some(Box::new(cleanup)),
        }
    }

    pub fn token(&self) -> u64 {
        self.token
    }

    pub fn claim(&mut self) -> Result<u64, HostError> {
        if self.cleanup.take().is_none() {
            return Err(HostError::InvalidArgument(
                "CDP session lease was already claimed or aborted".into(),
            ));
        }
        Ok(self.token)
    }

    /// Schedule compensation immediately; Drop remains an idempotent fallback.
    pub fn abort(&mut self) {
        if let Some(cleanup) = self.cleanup.take() {
            cleanup();
        }
    }
}

impl Drop for ClaimLease {
    fn drop(&mut self) {
        self.abort();
    }
}
