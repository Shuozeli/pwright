//! Optional BrowserContext service-worker policy over the init-script capability.

use crate::{HostError, HostSession};

const BLOCK_REGISTRATION_SOURCE: &str = "if (navigator.serviceWorker) navigator.serviceWorker.register = async () => { console.warn('Service Worker registration blocked by Playwright'); };";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ServiceWorkerPolicy {
    #[default]
    Allow,
    Block,
}

impl ServiceWorkerPolicy {
    const fn init_script(self) -> Option<&'static str> {
        match self {
            Self::Allow => None,
            Self::Block => Some(BLOCK_REGISTRATION_SOURCE),
        }
    }
}

impl HostSession {
    /// Install the selected policy before the newly created Context is returned
    /// to a language binding. A failed registration closes that exact Context.
    pub async fn install_service_worker_policy(
        &self,
        context_id: &str,
        policy: ServiceWorkerPolicy,
    ) -> Result<(), HostError> {
        if self.context_ownership(context_id)?.is_none() {
            return Err(HostError::TargetClosed);
        }
        let Some(source) = policy.init_script() else {
            return Ok(());
        };
        if let Err(primary) = self
            .context_add_init_script(context_id, source.to_owned())
            .await
        {
            if let Err(cleanup) = self.close_context(context_id).await {
                return Err(HostError::Backend(format!(
                    "service-worker policy installation failed: {primary}; Context cleanup failed: {cleanup}"
                )));
            }
            return Err(primary);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_policy_uses_the_exact_playwright_registration_override() {
        // Arrange
        let policy = ServiceWorkerPolicy::Block;

        // Act
        let source = policy.init_script();

        // Assert
        assert_eq!(source, Some(BLOCK_REGISTRATION_SOURCE));
        assert!(
            source
                .unwrap()
                .contains("Service Worker registration blocked by Playwright")
        );
    }

    #[test]
    fn allow_policy_installs_no_script() {
        // Arrange
        let policy = ServiceWorkerPolicy::Allow;

        // Act
        let source = policy.init_script();

        // Assert
        assert_eq!(source, None);
    }
}
