use super::*;
use pwright_cdp::connection::CdpError;
use std::fmt::{self, Write};

pub type NativeContextCloseFuture =
    Pin<Box<dyn Future<Output = Result<NativeContextCloseReceipt, Error>> + Send>>;

pub struct PreparedNativeContextClose {
    future: NativeContextCloseFuture,
}
impl PreparedNativeContextClose {
    /// Retain started work until reply/transport termination even on caller loss.
    pub fn start(self) -> NativeContextCloseFuture {
        self.future
    }
}

pub struct NativeContextCloseReceipt {
    ticket: Ticket,
    epoch: ConnectionEpoch,
    context: ObjectId,
    native_context: Box<str>,
    through: u64,
}

/// Borrowed native evidence only. The caller must serialize certification and
/// retirement commit without await; this is not a public event/commit receipt.
pub struct NativeAncestorRetirement<'a> {
    page: &'a NativePageCloseBinding,
    _context: &'a NativeContextCloseReceipt,
}
impl NativeAncestorRetirement<'_> {
    pub fn page(&self) -> ObjectId {
        self.page.page
    }
    pub fn context(&self) -> ObjectId {
        self.page.context
    }
}

impl NativeCloseAuthority {
    pub fn prepare_context(&self, context: ObjectId) -> Result<PreparedNativeContextClose, Error> {
        self.0.ensure_live()?;
        let topology = self.0.topology.lock().map_err(|_| poisoned())?;
        if topology.topology().epoch() != self.0.epoch {
            return Err(stale());
        }
        let route = topology
            .topology()
            .browser_context_route(context)
            .ok_or_else(|| {
                fail(
                    FailureKind::TargetClosed,
                    "native close Context route is absent",
                )
            })?;
        if route.ownership() != CdpContextOwnership::Owned {
            return Err(fail(
                FailureKind::UnsupportedCapability,
                "native close authority cannot dispose an unowned Context",
            ));
        }
        self.0.validate_id(route.browser_context_id())?;
        let ticket = Ticket::new(self.0.clone())?;
        let mut receipt = NativeContextCloseReceipt {
            ticket,
            epoch: topology.topology().epoch(),
            context,
            native_context: route.browser_context_id().into(),
            through: 0,
        };
        drop(topology);
        // Retained ID, parameter/JSON copies, bounded diagnostic and command
        // bookkeeping. Raw request/response admission belongs to the connection.
        let retained = std::mem::size_of::<NativeContextCloseReceipt>()
            + 3 * receipt.native_context.len()
            + 1024
            + 4096;
        let future = async move {
            receipt.validate(true)?;
            // Only an actual successful native response mints the receipt.
            // Error-classified already-closed results remain errors, not proof.
            CdpSession::browser(receipt.ticket.pending.0.connection.clone())
                .target_dispose_browser_context(&receipt.native_context)
                .await
                .map_err(native_error)?;
            receipt.validate(false)?;
            receipt.through = receipt.ticket.pending.0.sequence.load(Ordering::Acquire);
            Ok(receipt)
        };
        if retained + std::mem::size_of_val(&future) > INTENT_BYTES as usize {
            return Err(fail(
                FailureKind::ResourceExhausted,
                "native Context future exceeds admission",
            ));
        }
        Ok(PreparedNativeContextClose {
            future: Box::pin(future),
        })
    }
}

impl NativeContextCloseReceipt {
    pub fn context(&self) -> ObjectId {
        self.context
    }
    pub fn epoch(&self) -> ConnectionEpoch {
        self.epoch
    }

    pub(super) fn validate(&self, require_present: bool) -> Result<(), Error> {
        let state = &self.ticket.pending.0;
        state.ensure_live()?;
        let topology = state.topology.lock().map_err(|_| poisoned())?;
        self.validate_topology(topology.topology(), require_present)
    }

    pub(super) fn validate_topology(
        &self,
        topology: &pwright_backend_cdp::CdpTopology,
        require_present: bool,
    ) -> Result<(), Error> {
        if topology.epoch() != self.epoch || self.epoch != self.ticket.pending.0.epoch {
            return Err(stale());
        }
        match topology.browser_context_route(self.context) {
            Some(route)
                if route.browser_context_id() == self.native_context.as_ref()
                    && route.ownership() == CdpContextOwnership::Owned => {}
            None if !require_present => {}
            _ => return Err(stale()),
        }
        if topology
            .context_object_id(&self.native_context)
            .is_some_and(|id| id != self.context)
        {
            return Err(stale());
        }
        Ok(())
    }

    pub fn certify_page<'a>(
        &'a self,
        page: &'a NativePageCloseBinding,
    ) -> Result<NativeAncestorRetirement<'a>, Error> {
        self.validate_page_binding(page)?;
        let state = &self.ticket.pending.0;
        state.ensure_live()?;
        let topology = state.topology.lock().map_err(|_| poisoned())?;
        self.certify_page_in_topology(page, topology.topology())?;
        Ok(NativeAncestorRetirement {
            page,
            _context: self,
        })
    }

    #[cfg(feature = "native-close-scheduler")]
    pub(super) fn state(&self) -> &Arc<State> {
        &self.ticket.pending.0
    }

    /// The selecting caller holds this exact authority's topology lock. This
    /// form permits proof validation and local commit under one lock, without
    /// recursively locking during ancestor validation.
    pub(super) fn certify_page_in_topology(
        &self,
        page: &NativePageCloseBinding,
        topology: &pwright_backend_cdp::CdpTopology,
    ) -> Result<(), Error> {
        self.validate_page_binding(page)?;
        self.validate_topology(topology, false)?;
        if topology
            .page_object_id(&page.target)
            .is_some_and(|id| id != page.page)
            || topology
                .target_session_route(&page.session)
                .is_some_and(|route| route.page_target_id() != page.target.as_ref())
        {
            return Err(stale());
        }
        if let Some(route) = topology
            .page_session_routes()
            .find(|route| route.object_id() == page.page)
            && (route.target_id() != page.target.as_ref()
                || route.session_id() != page.session.as_ref()
                || route.browser_context_object_id() != page.context)
        {
            return Err(stale());
        }
        Ok(())
    }

    fn validate_page_binding(&self, page: &NativePageCloseBinding) -> Result<(), Error> {
        if !Arc::ptr_eq(&self.ticket.pending.0, &page.ticket.pending.0)
            || self.epoch != page.epoch
            || self.context != page.context
            || self.native_context != page.native_context
            || page.ordinal > self.through
        {
            return Err(stale());
        }
        Ok(())
    }

    #[cfg(feature = "native-close-scheduler")]
    pub(super) fn native_context(&self) -> &str {
        &self.native_context
    }
}
fn stale() -> Error {
    fail(
        FailureKind::TargetClosed,
        "native ancestor receipt does not cover this exact Page binding",
    )
}

struct Bounded(String);
impl Write for Bounded {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        let mut end = value.len().min(1024 - self.0.len());
        while !value.is_char_boundary(end) {
            end -= 1;
        }
        self.0.push_str(&value[..end]);
        Ok(())
    }
}
fn native_error(error: CdpError) -> Error {
    let kind = pwright_backend_cdp_common::classify_cdp_failure(&error);
    let mut message = Bounded(String::with_capacity(1024));
    let _ = write!(message, "CDP Target.disposeBrowserContext failed: {error}");
    OperationError::new(kind, message.0, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_error_formatting_is_utf8_bounded() {
        // Arrange
        let error = CdpError::Other("关闭".repeat(2048));
        // Act
        let error = native_error(error);
        // Assert
        assert!(error.message().len() <= 1024);
        assert!(
            error
                .message()
                .starts_with("CDP Target.disposeBrowserContext failed:")
        );
        assert_eq!(error.kind(), FailureKind::Backend);
    }

    #[test]
    fn context_error_retains_native_timeout_classification() {
        // Arrange
        let error = CdpError::Timeout;
        // Act
        let error = native_error(error);
        // Assert
        assert_eq!(error.kind(), FailureKind::Timeout);
    }
}
