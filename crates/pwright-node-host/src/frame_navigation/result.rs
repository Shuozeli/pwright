//! Bounded native-response projection and independently retained typed results.
use crate::HostError;
use pwright_backend_cdp_navigation::frame_lifecycle::navigation::{
    FrameNavigationWait, NavigationPoll, NavigationWaitError,
};
use pwright_cap_navigation::v1_2::{
    FrameGotoResult, Header, NavigationErrorDetails, NavigationResponse,
};
use pwright_core::{
    FailureKind, OperationError, ProductionResourceLedger, ResourceClass, ResourceLease,
    ResourceRequirements, ResourceScope,
};
use serde::{
    Deserialize, Deserializer,
    de::{self, MapAccess, Visitor},
};
use std::{
    borrow::Cow,
    fmt,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

const RESULTS: usize = 32;
const SOURCE_BYTES: usize = 64 * 1024;
const HEADERS: usize = 256;
const URL_BYTES: usize = 16 * 1024;
const STATUS_BYTES: usize = 1024;
// Includes parser scratch, escaped strings, final owned strings, both bounded
// header arrays, error formatting and owner metadata. Native JSON has a separate
// arena lease. Allocator bookkeeping and the caller's task stack are external.
const RESULT_BYTES: u64 = 384 * 1024;
// At most twice the bounded source for geometrically grown parser scratch,
// once for escaped owned strings and once for final string copies, plus both
// header arrays and a fixed allowance for structs/error metadata.
const _: () = assert!(
    RESULT_BYTES as usize >= 4 * SOURCE_BYTES + 2 * HEADERS * std::mem::size_of::<Header>() + 4096
);
const RESULT_SCOPE: ResourceScope = ResourceScope::extension(126, 3);
const SLOTS_SCOPE: ResourceScope = ResourceScope::extension(126, 4);

pub struct FrameGotoOutput {
    outcome: Result<FrameGotoResult, OperationError<NavigationErrorDetails>>,
    timeout_ms: u32,
    _permit: ResultPermit,
}
impl FrameGotoOutput {
    /// Borrow the result while its independent allocation lease is retained.
    /// A foreign copy must acquire its own budget before cloning these values.
    pub fn outcome(&self) -> &Result<FrameGotoResult, OperationError<NavigationErrorDetails>> {
        &self.outcome
    }

    /// Host error projection for an admitted foreign conversion. Returned
    /// error strings are a separate bounded copy owned by that conversion.
    pub fn value(&self) -> Result<&FrameGotoResult, HostError> {
        self.outcome
            .as_ref()
            .map_err(|error| HostError::operation(error.clone(), self.timeout_ms))
    }
}

pub(super) struct ResultSlots {
    active: AtomicUsize,
    _lease: ResourceLease,
}
pub(super) struct ResultPermit {
    slots: Arc<ResultSlots>,
    _lease: ResourceLease,
}
impl Drop for ResultPermit {
    fn drop(&mut self) {
        self.slots.active.fetch_sub(1, Ordering::AcqRel);
    }
}
fn slots_requirements() -> ResourceRequirements {
    let bytes =
        (std::mem::size_of::<ResultSlots>() + 2 * std::mem::size_of::<usize>() + 256) as u64;
    ResourceRequirements::none()
        .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
        .with_metadata(1, 1)
}
pub(super) fn requirements() -> ResourceRequirements {
    ResourceRequirements::none()
        .with_class(
            ResourceClass::BackendStateBytes,
            RESULT_BYTES * RESULTS as u64,
            RESULT_BYTES * RESULTS as u64,
        )
        .with_metadata(1, RESULTS)
        .combine_concurrent(slots_requirements())
}
impl ResultSlots {
    pub(super) fn new(resources: &ProductionResourceLedger) -> Result<Arc<Self>, HostError> {
        let profile = slots_requirements();
        let lease = resources
            .claim(profile)
            .map_err(HostError::backend)?
            .reserve(
                SLOTS_SCOPE,
                ResourceClass::BackendStateBytes,
                profile.class(ResourceClass::BackendStateBytes).global,
            )
            .map_err(HostError::backend)?;
        Ok(Arc::new(Self {
            active: AtomicUsize::new(0),
            _lease: lease,
        }))
    }
    pub(super) fn admit(
        self: &Arc<Self>,
        resources: &ProductionResourceLedger,
    ) -> Result<ResultPermit, HostError> {
        self.active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < RESULTS).then_some(n + 1)
            })
            .map_err(|_| {
                HostError::ResourceExhausted(
                    "Frame navigation retained-result capacity exhausted".into(),
                )
            })?;
        let reserved = resources.claim(requirements()).and_then(|ledger| {
            Ok(ledger.reserve(RESULT_SCOPE, ResourceClass::BackendStateBytes, RESULT_BYTES)?)
        });
        match reserved {
            Ok(lease) => Ok(ResultPermit {
                slots: self.clone(),
                _lease: lease,
            }),
            Err(error) => {
                self.active.fetch_sub(1, Ordering::AcqRel);
                Err(HostError::ResourceExhausted(error.to_string()))
            }
        }
    }
}

pub(super) fn project(
    owner: FrameNavigationWait,
    permit: ResultPermit,
    timeout_ms: u32,
) -> Result<FrameGotoOutput, HostError> {
    let outcome = match owner.outcome() {
        Some(Ok(NavigationPoll::Complete {
            frame,
            response_json,
        })) => {
            if frame.get() == 0 {
                return Err(HostError::Backend(
                    "navigation returned zero Frame identity".into(),
                ));
            }
            response_json
                .map(parse_response)
                .transpose()
                .map(|response| FrameGotoResult {
                    frame_id: frame.get(),
                    response,
                })
        }
        Some(Err(error)) => Err(wait_error(error, owner.failure_text(), timeout_ms)),
        Some(Ok(NavigationPoll::Pending)) | None => {
            return Err(HostError::Backend(
                "navigation result is not settled".into(),
            ));
        }
    };
    // All borrowed input fields have been moved/copied under the permit before
    // releasing the native response arena. Neither owner retains HostSession.
    drop(owner);
    Ok(FrameGotoOutput {
        outcome,
        timeout_ms,
        _permit: permit,
    })
}

fn invalid(message: impl Into<String>) -> OperationError<NavigationErrorDetails> {
    OperationError::new(
        FailureKind::Backend,
        "Invalid native navigation response",
        Some(NavigationErrorDetails::InvalidBackendResponse(
            message.into(),
        )),
    )
}

fn parse_response(
    bytes: &[u8],
) -> Result<NavigationResponse, OperationError<NavigationErrorDetails>> {
    if bytes.len() > SOURCE_BYTES {
        return Err(OperationError::new(
            FailureKind::ResourceExhausted,
            "navigation response exceeds 64 KiB",
            None,
        ));
    }
    let response: NativeResponse<'_> = serde_json::from_slice(bytes)
        .map_err(|_| invalid("malformed or oversized native response"))?;
    if response.url.len() > URL_BYTES || response.status_text.len() > STATUS_BYTES {
        return Err(OperationError::new(
            FailureKind::ResourceExhausted,
            "navigation response string limit exceeded",
            None,
        ));
    }
    let mut headers = Vec::new();
    headers
        .try_reserve_exact(response.headers.0.len())
        .map_err(|_| invalid("response header allocation failed"))?;
    for (name, value) in response.headers.0 {
        headers.push(Header {
            name: name.0.into_owned(),
            value: value.0.into_owned(),
        });
    }
    headers.sort_unstable_by(|a, b| (&a.name, &a.value).cmp(&(&b.name, &b.value)));
    Ok(NavigationResponse {
        url: response.url.into_owned(),
        status: response.status,
        status_text: response.status_text.into_owned(),
        headers,
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeResponse<'a> {
    #[serde(borrow)]
    url: Cow<'a, str>,
    status: i64,
    #[serde(borrow)]
    status_text: Cow<'a, str>,
    #[serde(borrow)]
    headers: NativeHeaders<'a>,
}

struct NativeString<'a>(Cow<'a, str>);
impl<'de: 'a, 'a> Deserialize<'de> for NativeString<'a> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct StringVisitor<'a>(std::marker::PhantomData<&'a ()>);
        impl<'de: 'a, 'a> Visitor<'de> for StringVisitor<'a> {
            type Value = NativeString<'a>;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a response header string")
            }
            fn visit_borrowed_str<E: de::Error>(self, value: &'de str) -> Result<Self::Value, E> {
                Ok(NativeString(Cow::Borrowed(value)))
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(NativeString(Cow::Owned(value.to_owned())))
            }
        }
        deserializer.deserialize_str(StringVisitor(std::marker::PhantomData))
    }
}
struct NativeHeaders<'a>(Vec<(NativeString<'a>, NativeString<'a>)>);
impl<'de: 'a, 'a> Deserialize<'de> for NativeHeaders<'a> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct HeadersVisitor<'a>(std::marker::PhantomData<&'a ()>);
        impl<'de: 'a, 'a> Visitor<'de> for HeadersVisitor<'a> {
            type Value = NativeHeaders<'a>;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("at most 256 string-valued headers")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
                let mut values = Vec::new();
                values
                    .try_reserve_exact(HEADERS)
                    .map_err(de::Error::custom)?;
                let mut bytes = 0usize;
                while let Some(name) = map.next_key::<NativeString<'a>>()? {
                    if values.len() == HEADERS {
                        return Err(de::Error::custom("too many response headers"));
                    }
                    let value: NativeString<'a> = map.next_value()?;
                    bytes = bytes
                        .checked_add(name.0.len())
                        .and_then(|n| n.checked_add(value.0.len()))
                        .ok_or_else(|| de::Error::custom("response header size overflow"))?;
                    if bytes > SOURCE_BYTES {
                        return Err(de::Error::custom("response header byte limit exceeded"));
                    }
                    values.push((name, value));
                }
                Ok(NativeHeaders(values))
            }
        }
        deserializer.deserialize_map(HeadersVisitor(std::marker::PhantomData))
    }
}

fn wait_error(
    error: NavigationWaitError,
    text: Option<&str>,
    timeout_ms: u32,
) -> OperationError<NavigationErrorDetails> {
    use NavigationWaitError as E;
    let (kind, message) = match error {
        E::Timeout => (
            FailureKind::Timeout,
            format!("Timeout {timeout_ms}ms exceeded"),
        ),
        E::Detached => (FailureKind::TargetClosed, "Frame was detached".into()),
        E::PageClosed | E::Disconnected => (
            FailureKind::TargetClosed,
            "Target page, context or browser has been closed".into(),
        ),
        E::Canceled => (
            FailureKind::Cancelled,
            "Frame navigation was canceled".into(),
        ),
        E::Interrupted => (
            FailureKind::Backend,
            "Navigation interrupted by another navigation".into(),
        ),
        E::Network { .. } => (
            FailureKind::Backend,
            text.unwrap_or("Frame navigation failed").to_owned(),
        ),
        E::Download => (
            FailureKind::Backend,
            "Navigation resulted in a download".into(),
        ),
        E::Resources | E::Allocation | E::Capacity => (
            FailureKind::ResourceExhausted,
            "Frame navigation resource limit exceeded".into(),
        ),
        E::EventLoss => (
            FailureKind::ProtocolViolation,
            "Frame navigation event history was lost".into(),
        ),
        E::InvalidProfile
        | E::SizeOverflow
        | E::InvalidState
        | E::InvalidIdentity
        | E::TimeWentBackwards => (FailureKind::ProtocolViolation, error.to_string()),
    };
    let details = match error {
        E::Network { .. } => Some(NavigationErrorDetails::Network(message.clone())),
        E::EventLoss
        | E::InvalidProfile
        | E::SizeOverflow
        | E::InvalidState
        | E::InvalidIdentity
        | E::TimeWentBackwards => Some(NavigationErrorDetails::InvalidBackendResponse(
            message.clone(),
        )),
        _ => None,
    };
    OperationError::new(kind, message, details)
}

#[cfg(test)]
mod tests;
