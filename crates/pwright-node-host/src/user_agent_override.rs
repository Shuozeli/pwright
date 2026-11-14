//! Composes the optional device user agent and Context locale into Chromium's
//! single user-agent override command.
//!
//! Chromium couples `userAgent` and `acceptLanguage` in one protocol method.
//! Applying those policies from independent initializers would make the later
//! command erase the earlier field, so this stateless boundary projects both
//! independently retained policies and emits at most one command per session.

use std::error::Error;
use std::sync::Arc;

use pwright_backend_cdp_topology::CdpContextOwnership;
use pwright_cdp::CdpConnection;
use pwright_plugin_chromium_session_initialization::{
    ChromiumSessionInitializationFuture, ChromiumSessionInitializationTarget,
    ChromiumSessionInitializer,
};

#[cfg(feature = "context-locale")]
use crate::context_locale;
#[cfg(feature = "device-emulation")]
use crate::device_emulation;

pub(crate) struct Initializer<Downstream> {
    downstream: Downstream,
    connection: Arc<CdpConnection>,
    #[cfg(feature = "device-emulation")]
    device: Option<Arc<device_emulation::Installation>>,
    #[cfg(feature = "context-locale")]
    locale: Option<Arc<context_locale::Installation>>,
}

impl<Downstream> Initializer<Downstream> {
    pub(crate) fn new(downstream: Downstream, connection: Arc<CdpConnection>) -> Self {
        Self {
            downstream,
            connection,
            #[cfg(feature = "device-emulation")]
            device: None,
            #[cfg(feature = "context-locale")]
            locale: None,
        }
    }

    #[cfg(feature = "device-emulation")]
    pub(crate) fn set_device_installation(
        &mut self,
        installation: Arc<device_emulation::Installation>,
    ) {
        self.device = Some(installation);
    }

    #[cfg(feature = "context-locale")]
    pub(crate) fn set_locale_installation(
        &mut self,
        installation: Arc<context_locale::Installation>,
    ) {
        self.locale = Some(installation);
    }

    fn parameters(
        &self,
        context: pwright_core::ObjectId,
        ownership: CdpContextOwnership,
    ) -> Result<Option<serde_json::Value>, Box<dyn Error + Send + Sync>> {
        #[cfg(feature = "device-emulation")]
        let device = self
            .device
            .as_ref()
            .map(|installation| installation.settings(context, ownership))
            .transpose()?
            .flatten();
        #[cfg(feature = "device-emulation")]
        let user_agent = device.as_ref().and_then(|settings| settings.user_agent());
        #[cfg(not(feature = "device-emulation"))]
        let user_agent = None;

        #[cfg(feature = "context-locale")]
        let locale = self
            .locale
            .as_ref()
            .map(|installation| installation.locale(context, ownership))
            .transpose()?
            .flatten();
        #[cfg(feature = "context-locale")]
        let accept_language = locale.as_ref().map(|locale| locale.as_str());
        #[cfg(not(feature = "context-locale"))]
        let accept_language = None;

        Ok(user_agent_override_params(user_agent, accept_language))
    }
}

fn user_agent_override_params(
    user_agent: Option<&str>,
    accept_language: Option<&str>,
) -> Option<serde_json::Value> {
    match (user_agent, accept_language) {
        (None, None) => None,
        (Some(user_agent), None) => Some(serde_json::json!({ "userAgent": user_agent })),
        (user_agent, Some(accept_language)) => Some(serde_json::json!({
            "userAgent": user_agent.unwrap_or_default(),
            "acceptLanguage": accept_language,
        })),
    }
}

impl<Downstream> ChromiumSessionInitializer for Initializer<Downstream>
where
    Downstream: ChromiumSessionInitializer,
{
    fn initialize<'a>(
        &'a self,
        session_id: &'a str,
        target_id: &'a str,
        ownership: CdpContextOwnership,
        is_page: bool,
    ) -> ChromiumSessionInitializationFuture<'a> {
        self.downstream
            .initialize(session_id, target_id, ownership, is_page)
    }

    fn initialize_scoped<'a>(
        &'a self,
        target: ChromiumSessionInitializationTarget<'a>,
    ) -> ChromiumSessionInitializationFuture<'a> {
        Box::pin(async move {
            let params = self.parameters(target.context, target.ownership)?;
            let queue = params.map(|params| {
                self.connection.enqueue_command_with_session(
                    "Emulation.setUserAgentOverride",
                    params,
                    Some(target.session_id),
                )
            });
            crate::session_initialization_queue::enqueue_before_downstream(
                target.resume_fence,
                queue,
                self.downstream.initialize_scoped(target),
                |command| async move {
                    command.response().await?;
                    Ok::<(), Box<dyn Error + Send + Sync>>(())
                },
            )
            .await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_only_preserves_playwrights_empty_user_agent_sentinel() {
        // Arrange
        let locale = "ru-RU";

        // Act
        let params = user_agent_override_params(None, Some(locale));

        // Assert
        assert_eq!(
            params,
            Some(serde_json::json!({
                "userAgent": "",
                "acceptLanguage": "ru-RU",
            }))
        );
    }

    #[test]
    fn device_only_emits_the_selected_user_agent() {
        // Arrange
        let user_agent = "Pwright Mobile";

        // Act
        let params = user_agent_override_params(Some(user_agent), None);

        // Assert
        assert_eq!(
            params,
            Some(serde_json::json!({ "userAgent": "Pwright Mobile" }))
        );
    }

    #[test]
    fn device_and_locale_are_composed_into_one_override() {
        // Arrange
        let user_agent = "Pwright Mobile";
        let locale = "fr-FR";

        // Act
        let params = user_agent_override_params(Some(user_agent), Some(locale));

        // Assert
        assert_eq!(
            params,
            Some(serde_json::json!({
                "userAgent": "Pwright Mobile",
                "acceptLanguage": "fr-FR",
            }))
        );
    }

    #[test]
    fn absent_policies_emit_no_override() {
        // Arrange
        let user_agent = None;
        let locale = None;

        // Act
        let params = user_agent_override_params(user_agent, locale);

        // Assert
        assert_eq!(params, None);
    }
}
