//! Optional bounded BrowserContext proxy ownership.
//!
//! The host retains public proxy policy for exactly one owned Context. CDP
//! receives only server/bypass creation fields; API-request composition can
//! independently consume the retained credentials and bypass selector.

use std::sync::{Arc, Mutex};

use pwright_backend_cdp_context_creation::CdpBrowserContextCreationOptions;
use pwright_backend_cdp_topology::CdpContextOwnership;
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequirements,
    ResourceScope,
};
use url::{Host, Url};
use zeroize::Zeroizing;

use crate::HostError;

pub(crate) const MAX_CONTEXTS: usize = 16;
pub const MAX_PROXY_SERVER_BYTES: usize = 64 * 1024;
pub const MAX_PROXY_BYPASS_BYTES: usize = 64 * 1024;
pub const MAX_PROXY_CREDENTIAL_BYTES: usize = 4 * 1024;
const SCOPE: ResourceScope = ResourceScope::extension(197, 0);

#[derive(Clone, Eq, PartialEq)]
pub struct ContextProxy {
    server: Box<str>,
    bypass: Option<Box<str>>,
    username: Option<Zeroizing<String>>,
    password: Option<Zeroizing<String>>,
}

impl ContextProxy {
    pub fn new(
        server: String,
        bypass: Option<String>,
        username: Option<String>,
        password: Option<String>,
    ) -> Result<Self, HostError> {
        validate_field("proxy server", &server, 1, MAX_PROXY_SERVER_BYTES)?;
        let server = normalize_server(&server)?;
        validate_field(
            "normalized proxy server",
            &server,
            1,
            MAX_PROXY_SERVER_BYTES,
        )?;
        if matches!(
            Url::parse(&server)
                .map(|url| url.scheme().to_owned())
                .as_deref(),
            Ok("socks4" | "socks5")
        ) && (username.is_some() || password.is_some())
        {
            return Err(HostError::InvalidArgument(
                "Browser proxy authentication is not supported for socks4 or socks5".into(),
            ));
        }
        let bypass = bypass.map(|value| {
            value
                .split(',')
                .map(str::trim)
                .collect::<Vec<_>>()
                .join(",")
        });
        if let Some(value) = bypass.as_deref() {
            validate_field("proxy bypass", value, 0, MAX_PROXY_BYPASS_BYTES)?;
        }
        if let Some(value) = username.as_deref() {
            validate_field("proxy username", value, 0, MAX_PROXY_CREDENTIAL_BYTES)?;
        }
        if let Some(value) = password.as_deref() {
            validate_field("proxy password", value, 0, MAX_PROXY_CREDENTIAL_BYTES)?;
        }
        Ok(Self {
            server: server.into_boxed_str(),
            bypass: bypass.map(String::into_boxed_str),
            username: username.map(Zeroizing::new),
            password: password.map(Zeroizing::new),
        })
    }

    pub fn server(&self) -> &str {
        &self.server
    }

    pub fn bypass(&self) -> Option<&str> {
        self.bypass.as_deref()
    }

    pub fn username(&self) -> Option<&str> {
        self.username.as_deref().map(String::as_str)
    }

    pub fn password(&self) -> Option<&str> {
        self.password.as_deref().map(String::as_str)
    }

    pub(crate) fn cdp_options(&self) -> CdpBrowserContextCreationOptions {
        let bypass = self
            .bypass
            .as_deref()
            .filter(|value| !value.is_empty())
            .map_or_else(
                || "<-loopback>".to_owned(),
                |value| format!("<-loopback>,{value}"),
            );
        CdpBrowserContextCreationOptions::with_proxy(self.server.to_string(), Some(bypass))
    }
}

impl std::fmt::Debug for ContextProxy {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ContextProxy")
            .field("server", &self.server)
            .field("bypass", &self.bypass)
            .field("has_username", &self.username.is_some())
            .field("has_password", &self.password.is_some())
            .finish()
    }
}

struct Entry {
    context: ObjectId,
    proxy: ContextProxy,
}

pub(crate) const fn requirements() -> ResourceRequirements {
    let retained_bytes =
        MAX_PROXY_SERVER_BYTES + MAX_PROXY_BYPASS_BYTES + (2 * MAX_PROXY_CREDENTIAL_BYTES);
    let storage_bytes = (std::mem::size_of::<Installation>()
        + MAX_CONTEXTS * (std::mem::size_of::<Entry>() + retained_bytes))
        as u64;
    ResourceRequirements::none()
        .with_class(
            ResourceClass::BackendStateBytes,
            storage_bytes,
            storage_bytes,
        )
        .with_metadata(1, 1)
}

pub(crate) struct Installation {
    entries: Mutex<Vec<Entry>>,
    _storage: ResourceLease,
}

impl Installation {
    pub(crate) fn new(resources: &ProductionResourceLedger) -> Result<Arc<Self>, HostError> {
        let plan = requirements();
        let storage = resources
            .claim(plan)
            .map_err(HostError::backend)?
            .reserve(
                SCOPE,
                ResourceClass::BackendStateBytes,
                plan.class(ResourceClass::BackendStateBytes).global,
            )
            .map_err(HostError::backend)?;
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(MAX_CONTEXTS)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        Ok(Arc::new(Self {
            entries: Mutex::new(entries),
            _storage: storage,
        }))
    }

    pub(crate) fn install(&self, context: ObjectId, proxy: ContextProxy) -> Result<(), HostError> {
        let mut entries = self.entries.lock().map_err(HostError::backend)?;
        if entries.iter().any(|entry| entry.context == context) {
            return Err(HostError::InvalidArgument(
                "proxy is already installed for this Context".into(),
            ));
        }
        if entries.len() == MAX_CONTEXTS {
            return Err(HostError::ResourceExhausted(format!(
                "proxy policy exceeds {MAX_CONTEXTS} live Contexts"
            )));
        }
        entries.push(Entry { context, proxy });
        Ok(())
    }

    pub(crate) fn remove(&self, context: ObjectId) -> Result<(), HostError> {
        self.entries
            .lock()
            .map_err(HostError::backend)?
            .retain(|entry| entry.context != context);
        Ok(())
    }

    pub(crate) fn proxy(
        &self,
        context: ObjectId,
        ownership: CdpContextOwnership,
    ) -> Result<Option<ContextProxy>, HostError> {
        if ownership != CdpContextOwnership::Owned {
            return Ok(None);
        }
        Ok(self
            .entries
            .lock()
            .map_err(HostError::backend)?
            .iter()
            .find(|entry| entry.context == context)
            .map(|entry| entry.proxy.clone()))
    }
}

fn validate_field(
    name: &str,
    value: &str,
    minimum: usize,
    maximum: usize,
) -> Result<(), HostError> {
    if value.len() < minimum || value.len() > maximum {
        return Err(HostError::InvalidArgument(format!(
            "{name} must contain {minimum}..={maximum} UTF-8 bytes"
        )));
    }
    if value.contains('\0') {
        return Err(HostError::InvalidArgument(format!(
            "{name} must not contain NUL"
        )));
    }
    Ok(())
}

fn normalize_server(server: &str) -> Result<String, HostError> {
    let server = server.trim();
    let parsed = Url::parse(server)
        .ok()
        .filter(Url::has_host)
        .or_else(|| Url::parse(&format!("http://{server}")).ok())
        .filter(Url::has_host)
        .ok_or_else(|| HostError::InvalidArgument("proxy server must include a host".into()))?;
    let host = match parsed.host() {
        Some(Host::Domain(value)) => value.to_owned(),
        Some(Host::Ipv4(value)) => value.to_string(),
        Some(Host::Ipv6(value)) => format!("[{value}]"),
        None => {
            return Err(HostError::InvalidArgument(
                "proxy server must include a host".into(),
            ));
        }
    };
    let port = parsed
        .port()
        .map_or_else(String::new, |value| format!(":{value}"));
    Ok(format!("{}://{host}{port}", parsed.scheme()))
}

#[cfg(test)]
mod tests {
    use pwright_core::{ProductionResourceLedger, ResourceClass, ResourceLimits};

    use super::*;

    fn resources() -> ProductionResourceLedger {
        let required = requirements();
        let mut limits = ResourceLimits::deny_all(required.scopes, required.leases);
        for class in ResourceClass::ALL {
            let limit = required.class(class);
            limits = limits.with_class(class, limit.global, limit.per_scope);
        }
        ProductionResourceLedger::new(limits, required).expect("valid proxy policy plan")
    }

    #[test]
    fn proxy_is_retained_only_for_its_owned_context() {
        // Arrange
        let installation = Installation::new(&resources()).expect("installation");
        let context = ObjectId::new(7);
        let proxy = ContextProxy::new(
            "proxy.example.test:3128".into(),
            Some("one.example.test".into()),
            Some("user".into()),
            Some("secret".into()),
        )
        .expect("proxy");

        // Act
        installation.install(context, proxy.clone()).unwrap();
        let owned = installation
            .proxy(context, CdpContextOwnership::Owned)
            .unwrap();
        let external = installation
            .proxy(context, CdpContextOwnership::External)
            .unwrap();

        // Assert
        assert_eq!(owned, Some(proxy));
        assert_eq!(external, None);
    }

    #[test]
    fn removal_is_idempotent() {
        // Arrange
        let installation = Installation::new(&resources()).expect("installation");
        let context = ObjectId::new(7);
        let proxy =
            ContextProxy::new("proxy.example.test:3128".into(), None, None, None).expect("proxy");
        installation.install(context, proxy).unwrap();

        // Act
        installation.remove(context).unwrap();
        installation.remove(context).unwrap();

        // Assert
        assert_eq!(
            installation
                .proxy(context, CdpContextOwnership::Owned)
                .unwrap(),
            None
        );
    }

    #[test]
    fn credentials_are_redacted_from_debug_output() {
        // Arrange
        let proxy = ContextProxy::new(
            "proxy.example.test:3128".into(),
            None,
            Some("private-user".into()),
            Some("private-password".into()),
        )
        .expect("proxy");

        // Act
        let output = format!("{proxy:?}");

        // Assert
        assert!(!output.contains("private-user"));
        assert!(!output.contains("private-password"));
    }

    #[test]
    fn server_and_bypass_are_normalized_before_cdp_creation() {
        // Arrange
        let proxy = ContextProxy::new(
            " proxy.example.test:3128/path?ignored=true ".into(),
            Some(" one.example.test, .two.example.test ".into()),
            None,
            None,
        )
        .expect("proxy");

        // Act
        let options = proxy.cdp_options();

        // Assert
        assert_eq!(proxy.server(), "http://proxy.example.test:3128");
        assert_eq!(proxy.bypass(), Some("one.example.test,.two.example.test"));
        assert_eq!(
            options.proxy_server(),
            Some("http://proxy.example.test:3128")
        );
        assert_eq!(
            options.proxy_bypass_list(),
            Some("<-loopback>,one.example.test,.two.example.test")
        );
    }

    #[test]
    fn browser_socks_authentication_is_rejected_at_context_creation() {
        // Arrange
        let input = "socks5://proxy.example.test:1080".to_owned();

        // Act
        let result = ContextProxy::new(input, None, Some("user".into()), None);

        // Assert
        assert!(
            matches!(result, Err(HostError::InvalidArgument(message)) if message.contains("socks5"))
        );
    }
}
