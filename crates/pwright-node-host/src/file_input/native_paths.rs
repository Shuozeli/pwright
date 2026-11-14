//! Opt-in native-path continuation for directory uploads. Ordinary remote
//! files remain payload transfers; selecting this leaf does not weaken the
//! separate-browser 50 MiB policy.

use super::{DirectUploadFuture, HostError, profile};
use pwright_backend_cdp_file_input::{
    CdpFileInputPreflightService, FileInputPreflightFlags, RetainedPreflightProfile,
};
use pwright_composition_chromium_file_input::{FileInputPreflightOperationService, UploadProfile};
use pwright_composition_chromium_locator_resolution::OwnedElement;
use pwright_core::{ProductionResourceLedger, ResourceRequirements, ResourceScope};
use pwright_file_input_fs::PreparedPaths;
use pwright_file_input_model::{UploadPlan, UploadRoute};
use pwright_file_input_tokio::CancellableReader;
use pwright_runtime::ReservedCleanupExecutor;
use pwright_runtime_tokio::{TokioCleanupExecutor, TokioCleanupReservation};
use std::{fs::File, sync::Arc};

const CALLS: usize = 1;

fn backend_profile() -> Result<RetainedPreflightProfile, HostError> {
    RetainedPreflightProfile::new(ResourceScope::extension(148, 7), CALLS)
        .map_err(HostError::backend)
}

fn owner_profile() -> Result<UploadProfile, HostError> {
    UploadProfile::new(ResourceScope::extension(148, 8), CALLS, 16 * 1024)
        .map_err(HostError::backend)
}

pub(super) fn requirements() -> Result<ResourceRequirements, HostError> {
    Ok(backend_profile()?
        .requirements()
        .combine_concurrent(owner_profile()?.requirements()))
}

pub(crate) struct Service {
    preflight: FileInputPreflightOperationService<TokioCleanupExecutor>,
    preflight_cleanup: TokioCleanupExecutor,
    operation_cleanup: TokioCleanupExecutor,
}

impl Service {
    pub(super) fn new(resources: &ProductionResourceLedger) -> Result<Self, HostError> {
        let preflight_cleanup = TokioCleanupExecutor::new(CALLS);
        let preflight = FileInputPreflightOperationService::new(
            owner_profile()?,
            resources,
            Arc::new(
                CdpFileInputPreflightService::new(backend_profile()?, resources)
                    .map_err(HostError::backend)?,
            ),
            preflight_cleanup.clone(),
        )
        .map_err(HostError::backend)?;
        Ok(Self {
            preflight,
            preflight_cleanup,
            operation_cleanup: TokioCleanupExecutor::new(CALLS),
        })
    }

    pub(crate) async fn apply<Retained: Send + 'static>(
        &self,
        element: OwnedElement<TokioCleanupReservation>,
        prepared: PreparedPaths<CancellableReader<File>>,
        retained: Retained,
    ) -> Result<(), HostError> {
        let flags = preflight_flags(prepared.plan());
        let mut files = Vec::new();
        files
            .try_reserve_exact(prepared.paths().len())
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        for path in prepared.paths() {
            let value = path.to_str().ok_or_else(|| {
                HostError::InvalidArgument("file-input path must be valid UTF-8".into())
            })?;
            files.push(value.to_owned());
        }

        // Reserve the terminal continuation before starting prevalidation.
        // The preflight owner has its own independent cleanup reservation.
        let reservation = self
            .operation_cleanup
            .try_reserve()
            .map_err(HostError::backend)?;
        let validated = self
            .preflight
            .prepare(element, flags)
            .map_err(HostError::backend)?
            .await
            .map_err(HostError::backend)?;
        let operation = async move {
            let _prepared = prepared;
            let _retained = retained;
            let client = validated.element().client();
            let object_id = validated.object_id().to_owned();
            let primary = client
                .dom_set_file_input_files_by_object_id(&object_id, &files)
                .await
                .map_err(HostError::backend);
            let cleanup = validated.release().await.map_err(HostError::backend);
            match (primary, cleanup) {
                (Ok(()), result) | (result, Ok(())) => result,
                (Err(primary), Err(cleanup)) => Err(HostError::backend(format_args!(
                    "{primary}; native file-input cleanup failed: {cleanup}"
                ))),
            }
        };
        if std::mem::size_of_val(&operation) > profile::CALL_BYTES as usize {
            return Err(HostError::ResourceExhausted(
                "native file-input continuation exceeds admitted storage".into(),
            ));
        }
        DirectUploadFuture {
            inner: Some(Box::pin(operation)),
            reservation: Some(reservation),
            started: false,
        }
        .await
    }

    pub(super) async fn shutdown(&self) -> Result<(), HostError> {
        let preflight = self.preflight_cleanup.shutdown().await;
        let operations = self.operation_cleanup.shutdown().await;
        if preflight.metrics.failed != 0
            || preflight.metrics.timed_out != 0
            || preflight.metrics.cancelled != 0
            || operations.metrics.failed != 0
            || operations.metrics.timed_out != 0
            || operations.metrics.cancelled != 0
            || !self.preflight.is_cleanup_healthy()
        {
            return Err(HostError::backend("native file-input cleanup failed"));
        }
        Ok(())
    }
}

fn preflight_flags(plan: UploadPlan) -> FileInputPreflightFlags {
    FileInputPreflightFlags {
        multiple: plan.items() > 1,
        directory_upload: plan.route() == UploadRoute::NativeDirectory,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwright_file_input_model::{
        BrowserLocation, FileInputLimits, PathKind, ResolvedPathMetadata,
    };

    #[test]
    fn directory_plan_selects_directory_prevalidation() {
        // Arrange
        let limits = FileInputLimits::new(8, 128, 64, 64, 256).unwrap();
        let paths = [ResolvedPathMetadata {
            path: "/fixtures/folder",
            kind: PathKind::Directory,
        }];
        let plan = limits.paths(&paths, BrowserLocation::Separate).unwrap();

        // Act
        let flags = preflight_flags(plan);

        // Assert
        assert!(!flags.multiple);
        assert!(flags.directory_upload);
    }
}
