//! Explicitly selected Context services; ownership stays in Rust topology.

#[cfg(feature = "context-proxy")]
use pwright_backend_cdp_context_creation::CdpBrowserContextCreationOptions;
use pwright_backend_cdp_topology::CdpContextOwnership;
#[cfg(not(feature = "context-proxy"))]
use pwright_cap_context_creation::BrowserNewContext;
use pwright_cap_context_lifecycle::BrowserContextClose;
use pwright_composition_chromium::{
    AttachedBrowserNewContextService, AttachedChromiumComposition,
    ChromiumNetworkPageRegistrationHandle,
};
#[cfg(not(feature = "context-proxy"))]
use pwright_core::Service;
use pwright_core::{CallId, CallMetadata, Command, ObjectId};
use pwright_runtime_tokio::TokioCleanupExecutor;

use crate::lifecycle::{DriverRequest, ordinary_close, outcome};
use crate::{CommandRuntime, ContextDescriptor, HostError, HostSession};

#[cfg(any(
    feature = "device-emulation",
    feature = "context-geolocation",
    feature = "context-geolocation-permission",
    feature = "context-http-credentials",
    feature = "context-locale",
    feature = "context-offline",
    feature = "context-forced-colors",
    feature = "context-proxy",
    feature = "ignore-https-errors",
    feature = "bypass-csp",
    feature = "javascript-disabled",
    feature = "context-timezone",
    feature = "init-scripts",
    feature = "page-bindings"
))]
fn portable_context_id(value: &str) -> Result<ObjectId, HostError> {
    value
        .parse::<u64>()
        .ok()
        .filter(|value| *value != 0)
        .map(ObjectId::new)
        .ok_or_else(|| HostError::InvalidArgument("invalid portable Context ID".into()))
}

pub(crate) struct ContextLifecycleServices {
    create: AttachedBrowserNewContextService<TokioCleanupExecutor>,
    close: crate::close_services::ContextCloseService,
    metadata: crate::context_metadata::ContextMetadata,
    trees: std::sync::Arc<crate::frame_trees::FrameTrees>,
}

#[derive(Default)]
struct ContextCreationOptions<'a> {
    base_url: Option<&'a str>,
    #[cfg(feature = "extra-headers")]
    headers: Option<&'a [crate::extra_headers::HeaderRef<'a>]>,
    #[cfg(feature = "device-emulation")]
    device: Option<crate::DeviceContextOptions>,
    #[cfg(feature = "context-geolocation")]
    geolocation: Option<crate::ContextGeolocation>,
    #[cfg(feature = "context-geolocation-permission")]
    geolocation_permission: bool,
    #[cfg(feature = "context-http-credentials")]
    http_credentials: Option<crate::ContextHttpCredentials>,
    #[cfg(feature = "context-locale")]
    locale: Option<crate::ContextLocale>,
    #[cfg(feature = "context-timezone")]
    timezone: Option<crate::ContextTimezone>,
    #[cfg(feature = "context-offline")]
    offline: bool,
    #[cfg(feature = "context-forced-colors")]
    forced_colors: Option<crate::ContextForcedColors>,
    #[cfg(feature = "context-proxy")]
    proxy: Option<crate::ContextProxy>,
    #[cfg(feature = "ignore-https-errors")]
    ignore_https_errors: bool,
    #[cfg(feature = "bypass-csp")]
    bypass_csp: bool,
    #[cfg(feature = "javascript-disabled")]
    javascript_disabled: bool,
}

impl<'a> ContextCreationOptions<'a> {
    fn with_base_url(base_url: Option<&'a str>) -> Self {
        Self {
            base_url,
            #[cfg(feature = "extra-headers")]
            headers: None,
            #[cfg(feature = "device-emulation")]
            device: None,
            #[cfg(feature = "context-geolocation")]
            geolocation: None,
            #[cfg(feature = "context-geolocation-permission")]
            geolocation_permission: false,
            #[cfg(feature = "context-http-credentials")]
            http_credentials: None,
            #[cfg(feature = "context-locale")]
            locale: None,
            #[cfg(feature = "context-timezone")]
            timezone: None,
            #[cfg(feature = "context-offline")]
            offline: false,
            #[cfg(feature = "context-forced-colors")]
            forced_colors: None,
            #[cfg(feature = "context-proxy")]
            proxy: None,
            #[cfg(feature = "ignore-https-errors")]
            ignore_https_errors: false,
            #[cfg(feature = "bypass-csp")]
            bypass_csp: false,
            #[cfg(feature = "javascript-disabled")]
            javascript_disabled: false,
        }
    }
}

/// Independently selected policies for one newly owned BrowserContext.
#[derive(Default)]
pub struct ContextPolicyOptions {
    #[cfg(feature = "context-geolocation")]
    geolocation: Option<crate::ContextGeolocation>,
    #[cfg(feature = "context-geolocation-permission")]
    geolocation_permission: bool,
    #[cfg(feature = "context-http-credentials")]
    http_credentials: Option<crate::ContextHttpCredentials>,
    #[cfg(feature = "context-locale")]
    locale: Option<crate::ContextLocale>,
    #[cfg(feature = "context-timezone")]
    timezone: Option<crate::ContextTimezone>,
    #[cfg(feature = "context-offline")]
    offline: bool,
    #[cfg(feature = "context-forced-colors")]
    forced_colors: Option<crate::ContextForcedColors>,
    #[cfg(feature = "context-proxy")]
    proxy: Option<crate::ContextProxy>,
    #[cfg(feature = "ignore-https-errors")]
    ignore_https_errors: bool,
    #[cfg(feature = "bypass-csp")]
    bypass_csp: bool,
    #[cfg(feature = "javascript-disabled")]
    javascript_disabled: bool,
}

/// Immutable browser-owned defaults used to construct one Context-bound API
/// request owner. Cookie state remains live in Chromium and is not copied into
/// this snapshot.
#[cfg(feature = "context-api-request")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextApiRequestOptions {
    pub base_url: Option<String>,
    pub user_agent: String,
    pub ignore_https_errors: bool,
    pub credentials: Option<ContextApiRequestCredentials>,
    #[cfg(feature = "context-proxy")]
    pub proxy: Option<crate::ContextProxy>,
}

#[cfg(feature = "context-api-request")]
#[derive(Clone, Eq, PartialEq)]
pub struct ContextApiRequestCredentials {
    pub username: String,
    pub password: String,
    pub origin: Option<String>,
    pub send_always: bool,
}

#[cfg(feature = "context-api-request")]
impl std::fmt::Debug for ContextApiRequestCredentials {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ContextApiRequestCredentials")
            .field("has_username", &!self.username.is_empty())
            .field("has_password", &!self.password.is_empty())
            .field("origin", &self.origin)
            .field("send_always", &self.send_always)
            .finish()
    }
}

impl ContextPolicyOptions {
    #[cfg(feature = "context-geolocation")]
    #[must_use]
    pub const fn with_geolocation(
        mut self,
        geolocation: Option<crate::ContextGeolocation>,
    ) -> Self {
        self.geolocation = geolocation;
        self
    }

    #[cfg(feature = "context-geolocation-permission")]
    #[must_use]
    pub const fn with_geolocation_permission(mut self, enabled: bool) -> Self {
        self.geolocation_permission = enabled;
        self
    }

    #[cfg(feature = "context-http-credentials")]
    #[must_use]
    pub fn with_http_credentials(
        mut self,
        credentials: Option<crate::ContextHttpCredentials>,
    ) -> Self {
        self.http_credentials = credentials;
        self
    }

    #[cfg(feature = "context-locale")]
    #[must_use]
    pub fn with_locale(mut self, locale: Option<crate::ContextLocale>) -> Self {
        self.locale = locale;
        self
    }

    #[cfg(feature = "context-timezone")]
    #[must_use]
    pub fn with_timezone(mut self, timezone: Option<crate::ContextTimezone>) -> Self {
        self.timezone = timezone;
        self
    }

    #[cfg(feature = "context-offline")]
    #[must_use]
    pub const fn with_offline(mut self, offline: bool) -> Self {
        self.offline = offline;
        self
    }

    #[cfg(feature = "context-forced-colors")]
    #[must_use]
    pub const fn with_forced_colors(
        mut self,
        forced_colors: Option<crate::ContextForcedColors>,
    ) -> Self {
        self.forced_colors = forced_colors;
        self
    }

    #[cfg(feature = "context-proxy")]
    #[must_use]
    pub fn with_proxy(mut self, proxy: Option<crate::ContextProxy>) -> Self {
        self.proxy = proxy;
        self
    }

    #[cfg(feature = "ignore-https-errors")]
    #[must_use]
    pub const fn with_ignore_https_errors(mut self, ignore_https_errors: bool) -> Self {
        self.ignore_https_errors = ignore_https_errors;
        self
    }

    #[cfg(feature = "bypass-csp")]
    #[must_use]
    pub const fn with_bypass_csp(mut self, bypass_csp: bool) -> Self {
        self.bypass_csp = bypass_csp;
        self
    }

    #[cfg(feature = "javascript-disabled")]
    #[must_use]
    pub const fn with_javascript_enabled(mut self, javascript_enabled: bool) -> Self {
        self.javascript_disabled = !javascript_enabled;
        self
    }
}

impl ContextLifecycleServices {
    pub fn new(
        composition: &AttachedChromiumComposition,
        runtime: &CommandRuntime,
        cleanup: TokioCleanupExecutor,
        metadata: crate::context_metadata::ContextMetadata,
        trees: std::sync::Arc<crate::frame_trees::FrameTrees>,
        network: Option<ChromiumNetworkPageRegistrationHandle>,
    ) -> Result<Self, HostError> {
        let create = composition
            .browser_new_context_service(runtime.lifecycle_handle(), cleanup.clone())
            .map_err(HostError::backend)?
            .ok_or_else(|| {
                HostError::UnsupportedOperation("Context creation profile is not installed".into())
            })?;
        let close = crate::close_services::ContextCloseService::new(
            composition,
            runtime,
            cleanup,
            &trees,
            network,
        )?;
        Ok(Self {
            create,
            close,
            metadata,
            trees,
        })
    }

    pub async fn create(
        &self,
        call_id: CallId,
        browser: ObjectId,
        metadata: crate::context_metadata::ContextReservation,
        #[cfg(feature = "extra-headers")] headers: Option<crate::extra_headers::CreationHeaders>,
        #[cfg(feature = "context-proxy")] creation_options: CdpBrowserContextCreationOptions,
    ) -> Result<ContextDescriptor, HostError> {
        let collection = self.trees.reserve_creation_collection()?;
        #[cfg(feature = "context-proxy")]
        let creation = self
            .create
            .call_with_cdp_options(call_id, browser, creation_options);
        #[cfg(not(feature = "context-proxy"))]
        let creation = self.create.call(Command::<BrowserNewContext>::new(
            call_id,
            browser,
            CallMetadata::default(),
            (),
        ));
        let context = outcome(creation.await)?;
        #[cfg(feature = "extra-headers")]
        if let Some(headers) = headers
            && let Err(error) = headers.commit(context.object_id)
        {
            self.close(call_id, context.object_id).await.map_err(|cleanup| HostError::Backend(format!(
                "Context header installation failed: {error}; compensation failed: {cleanup}")))?;
            return Err(error);
        }
        let metadata = match metadata.commit(context.object_id).and_then(|metadata| {
            collection.commit()?;
            Ok(metadata)
        }) {
            Ok(metadata) => metadata,
            Err(error) => {
                self.close(call_id, context.object_id).await.map_err(|cleanup| HostError::Backend(format!(
                    "Context metadata commit failed: {error}; compensation failed: {cleanup}")))?;
                return Err(error);
            }
        };
        Ok(ContextDescriptor {
            id: context.object_id.get().to_string(),
            settings: metadata.snapshot,
            settings_binding: Some(metadata.binding),
        })
    }

    pub async fn close(&self, call_id: CallId, context: ObjectId) -> Result<(), HostError> {
        ordinary_close(outcome(
            self.close
                .call(Command::<BrowserContextClose>::new(
                    call_id,
                    context,
                    CallMetadata::default(),
                    (),
                ))
                .await,
        ))?;
        self.metadata.remove(context)
    }
}

impl HostSession {
    #[cfg(feature = "context-api-request")]
    pub fn context_api_request_options(
        &self,
        context_id: &str,
    ) -> Result<ContextApiRequestOptions, HostError> {
        let (context, _call, _operation) = self.admit_operation(context_id)?;
        let (ownership, user_agent, ignore_https_errors, credentials) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            let route = topology
                .topology()
                .browser_context_route(context)
                .ok_or(HostError::TargetClosed)?;
            let ownership = route.ownership();
            let user_agent = live
                .device_emulation
                .settings(context, ownership)?
                .and_then(|settings| settings.user_agent().map(str::to_owned))
                .unwrap_or_else(|| self.browser_user_agent.clone());
            let ignore_https_errors = live.ignore_https_errors.enabled(context, ownership)?;
            let credentials = live
                .context_http_credentials
                .credentials(context, ownership)?
                .map(|credentials| ContextApiRequestCredentials {
                    username: credentials.username().to_owned(),
                    password: credentials.password().to_owned(),
                    origin: credentials.origin().map(str::to_owned),
                    send_always: credentials.send_always(),
                });
            (ownership, user_agent, ignore_https_errors, credentials)
        };
        #[cfg(feature = "context-proxy")]
        let proxy = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            live.context_proxy.proxy(context, ownership)?
        };
        let settings = self
            .context_metadata
            .visible(context, ownership == CdpContextOwnership::Owned)?
            .ok_or(HostError::TargetClosed)?;
        Ok(ContextApiRequestOptions {
            base_url: settings.base_url().map(str::to_owned),
            user_agent,
            ignore_https_errors,
            credentials,
            #[cfg(feature = "context-proxy")]
            proxy,
        })
    }

    pub async fn new_context(&self) -> Result<ContextDescriptor, HostError> {
        self.new_context_with_base_url(None).await
    }

    pub async fn new_context_with_base_url(
        &self,
        base_url: Option<&str>,
    ) -> Result<ContextDescriptor, HostError> {
        self.new_context_with_options(ContextCreationOptions::with_base_url(base_url))
            .await
    }

    #[cfg(feature = "device-emulation")]
    pub async fn new_context_with_device_options(
        &self,
        base_url: Option<&str>,
        device: Option<crate::DeviceContextOptions>,
    ) -> Result<ContextDescriptor, HostError> {
        self.new_context_with_options(ContextCreationOptions {
            base_url,
            device,
            ..ContextCreationOptions::default()
        })
        .await
    }

    #[cfg(feature = "ignore-https-errors")]
    pub async fn new_context_with_ignore_https_errors(
        &self,
        base_url: Option<&str>,
        #[cfg(feature = "device-emulation")] device: Option<crate::DeviceContextOptions>,
        ignore_https_errors: bool,
    ) -> Result<ContextDescriptor, HostError> {
        self.new_context_with_options(ContextCreationOptions {
            base_url,
            #[cfg(feature = "device-emulation")]
            device,
            ignore_https_errors,
            ..ContextCreationOptions::default()
        })
        .await
    }

    #[cfg(feature = "context-locale")]
    pub async fn new_context_with_locale(
        &self,
        base_url: Option<&str>,
        #[cfg(feature = "device-emulation")] device: Option<crate::DeviceContextOptions>,
        locale: Option<crate::ContextLocale>,
        #[cfg(feature = "ignore-https-errors")] ignore_https_errors: bool,
    ) -> Result<ContextDescriptor, HostError> {
        self.new_context_with_options(ContextCreationOptions {
            base_url,
            #[cfg(feature = "device-emulation")]
            device,
            locale,
            #[cfg(feature = "ignore-https-errors")]
            ignore_https_errors,
            ..ContextCreationOptions::default()
        })
        .await
    }

    /// Selected creation port. Desired headers are published atomically with the
    /// Context; HTTP syntax is deferred until a Page installs the merged values.
    #[cfg(feature = "extra-headers")]
    pub async fn new_context_with_extra_http_headers_native(
        &self,
        base_url: Option<&str>,
        headers: &[crate::extra_headers::HeaderRef<'_>],
    ) -> Result<ContextDescriptor, HostError> {
        #[allow(clippy::needless_update)] // Remaining fields vary by selected capabilities.
        self.new_context_with_options(ContextCreationOptions {
            base_url,
            headers: Some(headers),
            ..ContextCreationOptions::default()
        })
        .await
    }

    #[cfg(feature = "bypass-csp")]
    pub async fn new_context_with_bypass_csp(
        &self,
        base_url: Option<&str>,
        #[cfg(feature = "device-emulation")] device: Option<crate::DeviceContextOptions>,
        #[cfg(feature = "context-locale")] locale: Option<crate::ContextLocale>,
        #[cfg(feature = "ignore-https-errors")] ignore_https_errors: bool,
        bypass_csp: bool,
    ) -> Result<ContextDescriptor, HostError> {
        self.new_context_with_options(ContextCreationOptions {
            base_url,
            #[cfg(feature = "device-emulation")]
            device,
            #[cfg(feature = "context-locale")]
            locale,
            #[cfg(feature = "ignore-https-errors")]
            ignore_https_errors,
            bypass_csp,
            ..ContextCreationOptions::default()
        })
        .await
    }

    #[cfg(feature = "javascript-disabled")]
    pub async fn new_context_with_javascript_enabled(
        &self,
        base_url: Option<&str>,
        #[cfg(feature = "device-emulation")] device: Option<crate::DeviceContextOptions>,
        #[cfg(feature = "context-locale")] locale: Option<crate::ContextLocale>,
        #[cfg(feature = "ignore-https-errors")] ignore_https_errors: bool,
        #[cfg(feature = "bypass-csp")] bypass_csp: bool,
        javascript_enabled: bool,
    ) -> Result<ContextDescriptor, HostError> {
        self.new_context_with_options(ContextCreationOptions {
            base_url,
            #[cfg(feature = "device-emulation")]
            device,
            #[cfg(feature = "context-locale")]
            locale,
            #[cfg(feature = "ignore-https-errors")]
            ignore_https_errors,
            #[cfg(feature = "bypass-csp")]
            bypass_csp,
            javascript_disabled: !javascript_enabled,
            ..ContextCreationOptions::default()
        })
        .await
    }

    #[cfg(any(
        feature = "extra-headers",
        feature = "context-geolocation",
        feature = "context-geolocation-permission",
        feature = "context-http-credentials",
        feature = "context-locale",
        feature = "context-timezone",
        feature = "context-offline",
        feature = "context-forced-colors",
        feature = "context-proxy",
        feature = "ignore-https-errors",
        feature = "bypass-csp",
        feature = "javascript-disabled"
    ))]
    pub async fn new_context_with_policies(
        &self,
        base_url: Option<&str>,
        #[cfg(feature = "device-emulation")] device: Option<crate::DeviceContextOptions>,
        policies: ContextPolicyOptions,
    ) -> Result<ContextDescriptor, HostError> {
        self.new_context_with_options(Self::context_creation_options_with_policies(
            base_url,
            #[cfg(feature = "device-emulation")]
            device,
            policies,
            #[cfg(feature = "extra-headers")]
            None,
        ))
        .await
    }

    /// Publish creation headers together with every selected Context policy.
    /// The existing policy entry point remains unchanged for callers without headers.
    #[cfg(feature = "extra-headers")]
    pub async fn new_context_with_policies_and_extra_http_headers(
        &self,
        base_url: Option<&str>,
        #[cfg(feature = "device-emulation")] device: Option<crate::DeviceContextOptions>,
        policies: ContextPolicyOptions,
        headers: &[crate::extra_headers::HeaderRef<'_>],
    ) -> Result<ContextDescriptor, HostError> {
        self.new_context_with_options(Self::context_creation_options_with_policies(
            base_url,
            #[cfg(feature = "device-emulation")]
            device,
            policies,
            Some(headers),
        ))
        .await
    }

    #[cfg(any(
        feature = "extra-headers",
        feature = "context-geolocation",
        feature = "context-geolocation-permission",
        feature = "context-http-credentials",
        feature = "context-locale",
        feature = "context-timezone",
        feature = "context-offline",
        feature = "context-forced-colors",
        feature = "context-proxy",
        feature = "ignore-https-errors",
        feature = "bypass-csp",
        feature = "javascript-disabled"
    ))]
    fn context_creation_options_with_policies<'a>(
        base_url: Option<&'a str>,
        #[cfg(feature = "device-emulation")] device: Option<crate::DeviceContextOptions>,
        policies: ContextPolicyOptions,
        #[cfg(feature = "extra-headers")] headers: Option<
            &'a [crate::extra_headers::HeaderRef<'a>],
        >,
    ) -> ContextCreationOptions<'a> {
        let ContextPolicyOptions {
            #[cfg(feature = "context-geolocation")]
            geolocation,
            #[cfg(feature = "context-geolocation-permission")]
            geolocation_permission,
            #[cfg(feature = "context-http-credentials")]
            http_credentials,
            #[cfg(feature = "context-locale")]
            locale,
            #[cfg(feature = "context-timezone")]
            timezone,
            #[cfg(feature = "context-offline")]
            offline,
            #[cfg(feature = "context-forced-colors")]
            forced_colors,
            #[cfg(feature = "context-proxy")]
            proxy,
            #[cfg(feature = "ignore-https-errors")]
            ignore_https_errors,
            #[cfg(feature = "bypass-csp")]
            bypass_csp,
            #[cfg(feature = "javascript-disabled")]
            javascript_disabled,
        } = policies;
        let mut options = ContextCreationOptions {
            base_url,
            ..ContextCreationOptions::default()
        };
        #[cfg(feature = "extra-headers")]
        {
            options.headers = headers;
        }
        #[cfg(feature = "device-emulation")]
        {
            options.device = device;
        }
        #[cfg(feature = "context-geolocation")]
        {
            options.geolocation = geolocation;
        }
        #[cfg(feature = "context-geolocation-permission")]
        {
            options.geolocation_permission = geolocation_permission;
        }
        #[cfg(feature = "context-http-credentials")]
        {
            options.http_credentials = http_credentials;
        }
        #[cfg(feature = "context-locale")]
        {
            options.locale = locale;
        }
        #[cfg(feature = "context-timezone")]
        {
            options.timezone = timezone;
        }
        #[cfg(feature = "context-offline")]
        {
            options.offline = offline;
        }
        #[cfg(feature = "context-forced-colors")]
        {
            options.forced_colors = forced_colors;
        }
        #[cfg(feature = "context-proxy")]
        {
            options.proxy = proxy;
        }
        #[cfg(feature = "ignore-https-errors")]
        {
            options.ignore_https_errors = ignore_https_errors;
        }
        #[cfg(feature = "bypass-csp")]
        {
            options.bypass_csp = bypass_csp;
        }
        #[cfg(feature = "javascript-disabled")]
        {
            options.javascript_disabled = javascript_disabled;
        }
        options
    }

    async fn new_context_with_options(
        &self,
        options: ContextCreationOptions<'_>,
    ) -> Result<ContextDescriptor, HostError> {
        let ContextCreationOptions {
            base_url,
            #[cfg(feature = "extra-headers")]
            headers,
            #[cfg(feature = "device-emulation")]
            device,
            #[cfg(feature = "context-geolocation")]
            geolocation,
            #[cfg(feature = "context-geolocation-permission")]
            geolocation_permission,
            #[cfg(feature = "context-http-credentials")]
            http_credentials,
            #[cfg(feature = "context-locale")]
            locale,
            #[cfg(feature = "context-timezone")]
            timezone,
            #[cfg(feature = "context-offline")]
            offline,
            #[cfg(feature = "context-forced-colors")]
            forced_colors,
            #[cfg(feature = "context-proxy")]
            proxy,
            #[cfg(feature = "ignore-https-errors")]
            ignore_https_errors,
            #[cfg(feature = "bypass-csp")]
            bypass_csp,
            #[cfg(feature = "javascript-disabled")]
            javascript_disabled,
        } = options;
        let (browser, call_id, _operation) = self.admit_operation(&self.browser_id)?;
        prune_metadata(&self.state, &self.context_metadata)?;
        let metadata = self.context_metadata.reserve_creation(base_url)?;
        #[cfg(feature = "extra-headers")]
        let headers = headers
            .map(|input| {
                let state = self.state.lock().map_err(HostError::backend)?;
                state
                    .as_ref()
                    .ok_or(HostError::TargetClosed)?
                    .extra_headers
                    .creation(input)
            })
            .transpose()?;
        let (reply, completion) = tokio::sync::oneshot::channel();
        self.send_driver_request(DriverRequest::NewContext {
            call_id,
            browser,
            // The reservation's body is already byte-admitted before boxing.
            metadata: Box::new(metadata),
            #[cfg(feature = "extra-headers")]
            headers,
            #[cfg(feature = "context-proxy")]
            creation_options: proxy.as_ref().map_or_else(
                CdpBrowserContextCreationOptions::default,
                crate::ContextProxy::cdp_options,
            ),
            reply,
        })?;
        let context = self.await_driver_reply(completion).await?;
        #[cfg(feature = "context-forced-colors")]
        if let Some(forced_colors) = forced_colors {
            let context_id = portable_context_id(&context.id)?;
            let installed = (|| {
                let state = self.state.lock().map_err(HostError::backend)?;
                let live = state.as_ref().ok_or(HostError::TargetClosed)?;
                let topology = live.composition.topology().map_err(HostError::backend)?;
                // A native close can finish after its caller is canceled. The
                // close future then never reaches its policy cleanup, so prune
                // retired native Contexts before enforcing the fixed slot cap.
                live.emulate_media.reconcile_contexts(|id| {
                    topology.topology().browser_context_route(id).is_some()
                })?;
                live.emulate_media
                    .install_context(context_id, forced_colors)
            })();
            if let Err(primary) = installed {
                if let Err(cleanup) = self.close_owned_context(&context.id).await {
                    return Err(HostError::Backend(format!(
                        "forced colors policy installation failed: {primary}; Context cleanup failed: {cleanup}"
                    )));
                }
                return Err(primary);
            }
        }
        #[cfg(feature = "context-proxy")]
        if let Some(proxy) = proxy {
            let context_id = portable_context_id(&context.id)?;
            let installation = {
                let state = self.state.lock().map_err(HostError::backend)?;
                state
                    .as_ref()
                    .ok_or(HostError::TargetClosed)?
                    .context_proxy
                    .clone()
            };
            if let Err(primary) = installation.install(context_id, proxy) {
                if let Err(cleanup) = self.close_owned_context(&context.id).await {
                    return Err(HostError::Backend(format!(
                        "proxy policy installation failed: {primary}; Context cleanup failed: {cleanup}"
                    )));
                }
                return Err(primary);
            }
        }
        #[cfg(feature = "context-geolocation")]
        if let Some(geolocation) = geolocation {
            let context_id = portable_context_id(&context.id)?;
            let installation = {
                let state = self.state.lock().map_err(HostError::backend)?;
                state
                    .as_ref()
                    .ok_or(HostError::TargetClosed)?
                    .context_geolocation
                    .clone()
            };
            if let Err(primary) = installation.install(context_id, geolocation) {
                if let Err(cleanup) = self.close_owned_context(&context.id).await {
                    return Err(HostError::Backend(format!(
                        "geolocation policy installation failed: {primary}; Context cleanup failed: {cleanup}"
                    )));
                }
                return Err(primary);
            }
        }
        #[cfg(feature = "context-geolocation-permission")]
        if geolocation_permission {
            let context_id = portable_context_id(&context.id)?;
            let (installation, native_id) = {
                let state = self.state.lock().map_err(HostError::backend)?;
                let live = state.as_ref().ok_or(HostError::TargetClosed)?;
                let topology = live.composition.topology().map_err(HostError::backend)?;
                let route = topology
                    .topology()
                    .browser_context_route(context_id)
                    .ok_or_else(|| {
                        HostError::Backend("created Context lacks a native topology route".into())
                    })?;
                (
                    live.context_geolocation_permission.clone(),
                    route.browser_context_id().to_owned(),
                )
            };
            if let Err(primary) = installation.install(context_id, &native_id).await {
                if let Err(cleanup) = self.close_owned_context(&context.id).await {
                    return Err(HostError::Backend(format!(
                        "geolocation permission installation failed: {primary}; Context cleanup failed: {cleanup}"
                    )));
                }
                return Err(primary);
            }
        }
        #[cfg(feature = "context-http-credentials")]
        if let Some(http_credentials) = http_credentials {
            let context_id = portable_context_id(&context.id)?;
            let installation = {
                let state = self.state.lock().map_err(HostError::backend)?;
                state
                    .as_ref()
                    .ok_or(HostError::TargetClosed)?
                    .context_http_credentials
                    .clone()
            };
            if let Err(primary) = installation.install(context_id, http_credentials) {
                if let Err(cleanup) = self.close_owned_context(&context.id).await {
                    return Err(HostError::Backend(format!(
                        "HTTP credentials installation failed: {primary}; Context cleanup failed: {cleanup}"
                    )));
                }
                return Err(primary);
            }
        }
        #[cfg(feature = "context-locale")]
        if let Some(locale) = locale {
            let context_id = portable_context_id(&context.id)?;
            let installation = {
                let state = self.state.lock().map_err(HostError::backend)?;
                state
                    .as_ref()
                    .ok_or(HostError::TargetClosed)?
                    .context_locale
                    .clone()
            };
            if let Err(primary) = installation.install(context_id, locale) {
                if let Err(cleanup) = self.close_owned_context(&context.id).await {
                    return Err(HostError::Backend(format!(
                        "locale policy installation failed: {primary}; Context cleanup failed: {cleanup}"
                    )));
                }
                return Err(primary);
            }
        }
        #[cfg(feature = "context-timezone")]
        if let Some(timezone) = timezone {
            let context_id = portable_context_id(&context.id)?;
            let installation = {
                let state = self.state.lock().map_err(HostError::backend)?;
                state
                    .as_ref()
                    .ok_or(HostError::TargetClosed)?
                    .context_timezone
                    .clone()
            };
            if let Err(primary) = installation.install(context_id, timezone) {
                if let Err(cleanup) = self.close_owned_context(&context.id).await {
                    return Err(HostError::Backend(format!(
                        "timezone policy installation failed: {primary}; Context cleanup failed: {cleanup}"
                    )));
                }
                return Err(primary);
            }
        }
        #[cfg(feature = "context-offline")]
        if offline {
            let context_id = portable_context_id(&context.id)?;
            let installation = {
                let state = self.state.lock().map_err(HostError::backend)?;
                state
                    .as_ref()
                    .ok_or(HostError::TargetClosed)?
                    .context_offline
                    .clone()
            };
            if let Err(primary) = installation.install(context_id) {
                if let Err(cleanup) = self.close_owned_context(&context.id).await {
                    return Err(HostError::Backend(format!(
                        "offline policy installation failed: {primary}; Context cleanup failed: {cleanup}"
                    )));
                }
                return Err(primary);
            }
        }
        #[cfg(feature = "ignore-https-errors")]
        if ignore_https_errors {
            let context_id = portable_context_id(&context.id)?;
            let installation = {
                let state = self.state.lock().map_err(HostError::backend)?;
                state
                    .as_ref()
                    .ok_or(HostError::TargetClosed)?
                    .ignore_https_errors
                    .clone()
            };
            if let Err(primary) = installation.install(context_id) {
                if let Err(cleanup) = self.close_owned_context(&context.id).await {
                    return Err(HostError::Backend(format!(
                        "ignore-HTTPS-errors policy installation failed: {primary}; Context cleanup failed: {cleanup}"
                    )));
                }
                return Err(primary);
            }
        }
        #[cfg(feature = "bypass-csp")]
        if bypass_csp {
            let context_id = portable_context_id(&context.id)?;
            let installation = {
                let state = self.state.lock().map_err(HostError::backend)?;
                state
                    .as_ref()
                    .ok_or(HostError::TargetClosed)?
                    .bypass_csp
                    .clone()
            };
            if let Err(primary) = installation.install(context_id) {
                if let Err(cleanup) = self.close_owned_context(&context.id).await {
                    return Err(HostError::Backend(format!(
                        "bypass-CSP policy installation failed: {primary}; Context cleanup failed: {cleanup}"
                    )));
                }
                return Err(primary);
            }
        }
        #[cfg(feature = "javascript-disabled")]
        if javascript_disabled {
            let context_id = portable_context_id(&context.id)?;
            let installation = {
                let state = self.state.lock().map_err(HostError::backend)?;
                state
                    .as_ref()
                    .ok_or(HostError::TargetClosed)?
                    .javascript_disabled
                    .clone()
            };
            if let Err(primary) = installation.install(context_id) {
                if let Err(cleanup) = self.close_owned_context(&context.id).await {
                    return Err(HostError::Backend(format!(
                        "JavaScript-disabled policy installation failed: {primary}; Context cleanup failed: {cleanup}"
                    )));
                }
                return Err(primary);
            }
        }
        #[cfg(feature = "device-emulation")]
        if let Some(device) = device {
            let context_id = portable_context_id(&context.id)?;
            let installation = {
                let state = self.state.lock().map_err(HostError::backend)?;
                state
                    .as_ref()
                    .ok_or(HostError::TargetClosed)?
                    .device_emulation
                    .clone()
            };
            if let Err(primary) = installation.install(context_id, device) {
                if let Err(cleanup) = self.close_owned_context(&context.id).await {
                    return Err(HostError::Backend(format!(
                        "device settings installation failed: {primary}; Context cleanup failed: {cleanup}"
                    )));
                }
                return Err(primary);
            }
        }
        #[cfg(feature = "request-history")]
        if let Err(primary) = self.enable_request_history_context(&context.id).await {
            if let Err(cleanup) = self.close_owned_context(&context.id).await {
                return Err(HostError::Backend(format!(
                    "request history activation failed: {primary}; Context cleanup failed: {cleanup}"
                )));
            }
            return Err(primary);
        }
        #[cfg(feature = "request-events")]
        if let Err(primary) = self.enable_request_event_context(&context.id).await {
            if let Err(cleanup) = self.close_owned_context(&context.id).await {
                return Err(HostError::Backend(format!(
                    "request event activation failed: {primary}; Context cleanup failed: {cleanup}"
                )));
            }
            return Err(primary);
        }
        #[cfg(feature = "response-events")]
        if let Err(primary) = self.enable_response_event_context(&context.id).await {
            if let Err(cleanup) = self.close_owned_context(&context.id).await {
                return Err(HostError::Backend(format!(
                    "response event activation failed: {primary}; Context cleanup failed: {cleanup}"
                )));
            }
            return Err(primary);
        }
        Ok(context)
    }

    pub async fn close_context(&self, context_id: &str) -> Result<(), HostError> {
        // Playwright's CDP default Context closes the attachment, not Chrome.
        // Do not keep an operation token while awaiting the host's own drain.
        let result = match self.context_ownership(context_id)? {
            Some(CdpContextOwnership::Default) => self.close().await,
            None => Ok(()),
            Some(CdpContextOwnership::Owned | CdpContextOwnership::External) => {
                let result = self.close_owned_context(context_id).await;
                crate::lifecycle::settle_context_close(result, &self.stop, &self.disconnected).await
            }
        };
        if result.is_ok() {
            self.browser_pages.remove_context(context_id)?;
            #[cfg(feature = "page-bindings")]
            self.retire_context_binding_registrations(portable_context_id(context_id)?)?;
        }
        result
    }

    async fn close_owned_context(&self, context_id: &str) -> Result<(), HostError> {
        let (context, call_id, _operation) = self.admit_operation(context_id)?;
        let (reply, completion) = tokio::sync::oneshot::channel();
        self.send_driver_request(DriverRequest::CloseContext {
            call_id,
            context,
            reply,
        })?;
        let result = self.await_driver_reply(completion).await;
        #[cfg(feature = "context-forced-colors")]
        if result.is_ok() {
            let context = portable_context_id(context_id)?;
            let installation = {
                let state = self.state.lock().map_err(HostError::backend)?;
                state
                    .as_ref()
                    .ok_or(HostError::TargetClosed)?
                    .emulate_media
                    .clone()
            };
            installation.remove_context(context)?;
        }
        #[cfg(feature = "context-offline")]
        if result.is_ok() {
            let context = portable_context_id(context_id)?;
            let installation = {
                let state = self.state.lock().map_err(HostError::backend)?;
                state
                    .as_ref()
                    .ok_or(HostError::TargetClosed)?
                    .context_offline
                    .clone()
            };
            installation.remove_serialized(context).await?;
        }
        #[cfg(feature = "download-events")]
        if result.is_ok() {
            self.disable_download_context(context_id).await?;
        }
        #[cfg(feature = "context-geolocation")]
        if result.is_ok() {
            let context = portable_context_id(context_id)?;
            let installation = {
                let state = self.state.lock().map_err(HostError::backend)?;
                state
                    .as_ref()
                    .ok_or(HostError::TargetClosed)?
                    .context_geolocation
                    .clone()
            };
            installation.remove(context)?;
        }
        #[cfg(feature = "context-geolocation-permission")]
        if result.is_ok() {
            let context = portable_context_id(context_id)?;
            let installation = {
                let state = self.state.lock().map_err(HostError::backend)?;
                state
                    .as_ref()
                    .ok_or(HostError::TargetClosed)?
                    .context_geolocation_permission
                    .clone()
            };
            installation.remove(context)?;
        }
        #[cfg(feature = "context-http-credentials")]
        if result.is_ok() {
            let context = portable_context_id(context_id)?;
            let installation = {
                let state = self.state.lock().map_err(HostError::backend)?;
                state
                    .as_ref()
                    .ok_or(HostError::TargetClosed)?
                    .context_http_credentials
                    .clone()
            };
            installation.remove(context)?;
        }
        #[cfg(feature = "init-scripts")]
        if result.is_ok() {
            let context = portable_context_id(context_id)?;
            let installation = {
                let state = self.state.lock().map_err(HostError::backend)?;
                state
                    .as_ref()
                    .ok_or(HostError::TargetClosed)?
                    .init_scripts
                    .clone()
            };
            installation.remove_context(context)?;
        }
        #[cfg(feature = "device-emulation")]
        if result.is_ok() {
            let context = portable_context_id(context_id)?;
            let installation = {
                let state = self.state.lock().map_err(HostError::backend)?;
                state
                    .as_ref()
                    .ok_or(HostError::TargetClosed)?
                    .device_emulation
                    .clone()
            };
            installation.remove(context)?;
        }
        #[cfg(feature = "context-locale")]
        if result.is_ok() {
            let context = portable_context_id(context_id)?;
            let installation = {
                let state = self.state.lock().map_err(HostError::backend)?;
                state
                    .as_ref()
                    .ok_or(HostError::TargetClosed)?
                    .context_locale
                    .clone()
            };
            installation.remove(context)?;
        }
        #[cfg(feature = "context-timezone")]
        if result.is_ok() {
            let context = portable_context_id(context_id)?;
            let installation = {
                let state = self.state.lock().map_err(HostError::backend)?;
                state
                    .as_ref()
                    .ok_or(HostError::TargetClosed)?
                    .context_timezone
                    .clone()
            };
            installation.remove(context)?;
        }
        #[cfg(feature = "context-proxy")]
        if result.is_ok() {
            let context = portable_context_id(context_id)?;
            let installation = {
                let state = self.state.lock().map_err(HostError::backend)?;
                state
                    .as_ref()
                    .ok_or(HostError::TargetClosed)?
                    .context_proxy
                    .clone()
            };
            installation.remove(context)?;
        }
        #[cfg(feature = "ignore-https-errors")]
        if result.is_ok() {
            let context = portable_context_id(context_id)?;
            let installation = {
                let state = self.state.lock().map_err(HostError::backend)?;
                state
                    .as_ref()
                    .ok_or(HostError::TargetClosed)?
                    .ignore_https_errors
                    .clone()
            };
            installation.remove(context)?;
        }
        #[cfg(feature = "bypass-csp")]
        if result.is_ok() {
            let context = portable_context_id(context_id)?;
            let installation = {
                let state = self.state.lock().map_err(HostError::backend)?;
                state
                    .as_ref()
                    .ok_or(HostError::TargetClosed)?
                    .bypass_csp
                    .clone()
            };
            installation.remove(context)?;
        }
        #[cfg(feature = "javascript-disabled")]
        if result.is_ok() {
            let context = portable_context_id(context_id)?;
            let installation = {
                let state = self.state.lock().map_err(HostError::backend)?;
                state
                    .as_ref()
                    .ok_or(HostError::TargetClosed)?
                    .javascript_disabled
                    .clone()
            };
            installation.remove(context)?;
        }
        result
    }

    pub(crate) fn context_ownership(
        &self,
        context_id: &str,
    ) -> Result<Option<CdpContextOwnership>, HostError> {
        let id = context_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable object ID".into()))?;
        let state = self.state.lock().map_err(HostError::backend)?;
        let Some(live) = state.as_ref() else {
            return Ok(None);
        };
        let topology = live.composition.topology().map_err(HostError::backend)?;
        let route = topology
            .topology()
            .browser_context_routes()
            .find(|context| context.object_id().get() == id);
        if self.context_metadata.owned_contexts_only()
            && let Some(route) = route
        {
            self.context_metadata
                .visible(
                    route.object_id(),
                    route.ownership() == CdpContextOwnership::Owned,
                )?
                .ok_or(HostError::TargetClosed)?;
        }
        Ok(route.map(|context| context.ownership()))
    }
}

pub(crate) fn prune_metadata(
    state: &std::sync::Mutex<Option<crate::LiveSession>>,
    metadata: &crate::context_metadata::ContextMetadata,
) -> Result<(), HostError> {
    let state = state.lock().map_err(HostError::backend)?;
    let Some(live) = state.as_ref() else {
        return Ok(());
    };
    #[cfg(feature = "extra-headers")]
    live.extra_headers.prune(live)?;
    let topology = live.composition.topology().map_err(HostError::backend)?;
    metadata.retain_live(|id| {
        topology
            .topology()
            .browser_context_routes()
            .any(|context| context.object_id() == id)
    })
}

#[cfg(all(test, feature = "extra-headers", feature = "context-offline"))]
mod creation_headers_tests {
    use super::{ContextPolicyOptions, HostSession};
    use crate::extra_headers::HeaderRef;

    #[test]
    fn creation_headers_keep_other_context_policies() {
        // Arrange
        let headers = [HeaderRef {
            name: "X-From-Context",
            value: "yes",
        }];
        let policies = ContextPolicyOptions::default().with_offline(true);

        // Act
        let options = HostSession::context_creation_options_with_policies(
            Some("https://example.test/"),
            #[cfg(feature = "device-emulation")]
            None,
            policies,
            Some(&headers),
        );

        // Assert
        assert_eq!(options.base_url, Some("https://example.test/"));
        assert!(options.offline);
        assert_eq!(
            options.headers.expect("headers must be composed")[0].name,
            "X-From-Context"
        );
    }

    #[test]
    fn no_headers_preserves_the_existing_context_policy_path() {
        // Arrange
        let policies = ContextPolicyOptions::default().with_offline(true);

        // Act
        let options = HostSession::context_creation_options_with_policies(
            None,
            #[cfg(feature = "device-emulation")]
            None,
            policies,
            None,
        );

        // Assert
        assert!(options.headers.is_none());
        assert!(options.offline);
    }
}
