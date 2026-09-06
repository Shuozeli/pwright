use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;

use pwright_core::{
    CallId, EventTag, MethodTag, ObjectId, ProtocolVersion, ResourceClass, ResourceError,
    ResourceLease, ResourceLedger, ResourceScope,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameLimits {
    pub payload_bytes: usize,
    pub attachments: usize,
    pub attachment_bytes: usize,
    pub total_bytes: usize,
}

impl FrameLimits {
    pub const fn new(
        payload_bytes: usize,
        attachments: usize,
        attachment_bytes: usize,
        total_bytes: usize,
    ) -> Self {
        Self {
            payload_bytes,
            attachments,
            attachment_bytes,
            total_bytes,
        }
    }

    /// Largest payload-plus-attachment body admitted by these limits.
    pub const fn max_frame_bytes(self) -> usize {
        let structural = self
            .payload_bytes
            .saturating_add(self.attachments.saturating_mul(self.attachment_bytes));
        if self.total_bytes < structural {
            self.total_bytes
        } else {
            structural
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameRoute {
    Handshake,
    Command {
        call_id: CallId,
        target: ObjectId,
        method: MethodTag,
    },
    Result {
        call_id: CallId,
    },
    Event {
        source: ObjectId,
        event: EventTag,
    },
    Control,
}

#[derive(Debug, Eq, PartialEq)]
pub struct BinaryAttachment {
    bytes: Box<[u8]>,
}

impl BinaryAttachment {
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

/// Exclusive reservation for the bytes that will become one [`WireFrame`].
///
/// Stream decoders acquire this reservation after validating declared lengths
/// but before allocating payload and attachment copies. It is intentionally
/// non-cloneable and can only be consumed by [`WireFrame::new_reserved`].
pub struct WireFrameReservation {
    total_bytes: usize,
    wire_bytes: Option<ResourceLease>,
    resource_ledger: ResourceLedger,
}

impl fmt::Debug for WireFrameReservation {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WireFrameReservation")
            .field("total_bytes", &self.total_bytes)
            .finish_non_exhaustive()
    }
}

struct WireFrameStorage {
    payload: Box<[u8]>,
    attachments: Box<[BinaryAttachment]>,
    _wire_bytes: Option<ResourceLease>,
    resource_ledger: Option<ResourceLedger>,
}

impl fmt::Debug for WireFrameStorage {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WireFrameStorage")
            .field("payload_bytes", &self.payload.len())
            .field("attachments", &self.attachments.len())
            .field("budgeted", &self.resource_ledger.is_some())
            .finish_non_exhaustive()
    }
}

/// One immutable wire frame.
///
/// Clones share immutable storage rather than duplicating payload and
/// attachment bytes. A budgeted frame therefore retains exactly one resource
/// lease until its final clone is dropped.
#[derive(Clone, Debug)]
pub struct WireFrame {
    version: ProtocolVersion,
    route: FrameRoute,
    storage: Arc<WireFrameStorage>,
    total_bytes: usize,
}

impl PartialEq for WireFrame {
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version
            && self.route == other.route
            && self.total_bytes == other.total_bytes
            && self.storage.payload == other.storage.payload
            && self.storage.attachments == other.storage.attachments
    }
}

impl Eq for WireFrame {}

impl WireFrame {
    pub fn new(
        version: ProtocolVersion,
        route: FrameRoute,
        payload: Vec<u8>,
        attachments: Vec<Vec<u8>>,
        limits: FrameLimits,
    ) -> Result<Self, FrameError> {
        let total_bytes = validate_parts(&payload, &attachments, limits)?;
        Ok(Self::from_parts(
            version,
            route,
            payload,
            attachments,
            total_bytes,
            None,
            None,
        ))
    }

    /// Builds a frame whose aggregate retained bytes consume the injected
    /// connection ledger's wire-byte budget.
    pub fn new_budgeted(
        version: ProtocolVersion,
        route: FrameRoute,
        payload: Vec<u8>,
        attachments: Vec<Vec<u8>>,
        limits: FrameLimits,
        ledger: &ResourceLedger,
        scope: ResourceScope,
    ) -> Result<Self, FrameError> {
        let total_bytes = validate_parts(&payload, &attachments, limits)?;
        let reservation = Self::reserve_bytes(ledger, scope, total_bytes)?;
        Ok(Self::from_parts(
            version,
            route,
            payload,
            attachments,
            total_bytes,
            reservation.wire_bytes,
            Some(reservation.resource_ledger),
        ))
    }

    /// Reserves the exact retained byte count for a frame before its byte
    /// buffers are allocated.
    pub fn reserve_bytes(
        ledger: &ResourceLedger,
        scope: ResourceScope,
        total_bytes: usize,
    ) -> Result<WireFrameReservation, FrameError> {
        let units = u64::try_from(total_bytes).map_err(|_| FrameError::TotalLengthOverflow)?;
        let wire_bytes = if units == 0 {
            None
        } else {
            Some(ledger.reserve(scope, ResourceClass::WireBytes, units)?)
        };
        Ok(WireFrameReservation {
            total_bytes,
            wire_bytes,
            resource_ledger: ledger.clone(),
        })
    }

    /// Builds a frame from an exact reservation acquired before allocating its
    /// payload and attachment buffers.
    pub fn new_reserved(
        version: ProtocolVersion,
        route: FrameRoute,
        payload: Vec<u8>,
        attachments: Vec<Vec<u8>>,
        limits: FrameLimits,
        reservation: WireFrameReservation,
    ) -> Result<Self, FrameError> {
        let total_bytes = validate_parts(&payload, &attachments, limits)?;
        if total_bytes != reservation.total_bytes {
            return Err(FrameError::ReservationSizeMismatch {
                reserved: reservation.total_bytes,
                actual: total_bytes,
            });
        }
        Ok(Self::from_parts(
            version,
            route,
            payload,
            attachments,
            total_bytes,
            reservation.wire_bytes,
            Some(reservation.resource_ledger),
        ))
    }

    /// Builds a frame from a maximum-size reservation acquired before a codec
    /// allocated its output buffers.
    ///
    /// The reservation is atomically shrunk and moved from its staging scope
    /// to the final route scope. Failed validation or resizing drops the
    /// maximum reservation and retains no frame.
    pub fn new_reserved_up_to(
        version: ProtocolVersion,
        route: FrameRoute,
        payload: Vec<u8>,
        attachments: Vec<Vec<u8>>,
        limits: FrameLimits,
        reservation: WireFrameReservation,
    ) -> Result<Self, FrameError> {
        let total_bytes = validate_parts(&payload, &attachments, limits)?;
        let WireFrameReservation {
            total_bytes: reserved,
            mut wire_bytes,
            resource_ledger,
        } = reservation;
        if total_bytes > reserved {
            return Err(FrameError::ReservationTooSmall {
                reserved,
                actual: total_bytes,
            });
        }
        if total_bytes == 0 {
            wire_bytes = None;
        } else if let Some(resource) = wire_bytes.as_mut() {
            resource.try_replace(
                route.resource_scope(),
                &[pwright_core::ResourceRequest::new(
                    ResourceClass::WireBytes,
                    u64::try_from(total_bytes).map_err(|_| FrameError::TotalLengthOverflow)?,
                )],
            )?;
        } else {
            return Err(FrameError::ReservationTooSmall {
                reserved,
                actual: total_bytes,
            });
        }
        Ok(Self::from_parts(
            version,
            route,
            payload,
            attachments,
            total_bytes,
            wire_bytes,
            Some(resource_ledger),
        ))
    }

    fn from_parts(
        version: ProtocolVersion,
        route: FrameRoute,
        payload: Vec<u8>,
        attachments: Vec<Vec<u8>>,
        total_bytes: usize,
        wire_bytes: Option<ResourceLease>,
        resource_ledger: Option<ResourceLedger>,
    ) -> Self {
        Self {
            version,
            route,
            storage: Arc::new(WireFrameStorage {
                payload: payload.into_boxed_slice(),
                attachments: attachments
                    .into_iter()
                    .map(|bytes| BinaryAttachment {
                        bytes: bytes.into_boxed_slice(),
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                _wire_bytes: wire_bytes,
                resource_ledger,
            }),
            total_bytes,
        }
    }

    pub const fn version(&self) -> ProtocolVersion {
        self.version
    }

    pub const fn route(&self) -> FrameRoute {
        self.route
    }

    pub fn payload(&self) -> &[u8] {
        &self.storage.payload
    }

    pub fn attachments(&self) -> &[BinaryAttachment] {
        &self.storage.attachments
    }

    pub const fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    /// Reports whether this frame was admitted by this exact connection
    /// ledger. Zero-byte budgeted frames retain the identity even though they
    /// need no byte lease.
    pub fn is_budgeted_by(&self, ledger: &ResourceLedger) -> bool {
        self.storage
            .resource_ledger
            .as_ref()
            .is_some_and(|owned| owned.shares_budget_with(ledger))
    }

    pub fn validate(&self, limits: FrameLimits) -> Result<(), FrameError> {
        if self.storage.payload.len() > limits.payload_bytes {
            return Err(FrameError::PayloadTooLarge {
                actual: self.storage.payload.len(),
                limit: limits.payload_bytes,
            });
        }
        if self.storage.attachments.len() > limits.attachments {
            return Err(FrameError::TooManyAttachments {
                actual: self.storage.attachments.len(),
                limit: limits.attachments,
            });
        }
        for (index, attachment) in self.storage.attachments.iter().enumerate() {
            if attachment.len() > limits.attachment_bytes {
                return Err(FrameError::AttachmentTooLarge {
                    index,
                    actual: attachment.len(),
                    limit: limits.attachment_bytes,
                });
            }
        }
        if self.total_bytes > limits.total_bytes {
            return Err(FrameError::TotalBytesTooLarge {
                actual: self.total_bytes,
                limit: limits.total_bytes,
            });
        }
        Ok(())
    }
}

impl FrameRoute {
    /// Resource attribution for bytes retained by this route.
    pub const fn resource_scope(self) -> ResourceScope {
        match self {
            Self::Command { call_id, .. } | Self::Result { call_id } => {
                ResourceScope::Call(call_id)
            }
            Self::Event { source, .. } => ResourceScope::Object(source),
            Self::Handshake | Self::Control => ResourceScope::Connection,
        }
    }
}

fn validate_parts(
    payload: &[u8],
    attachments: &[Vec<u8>],
    limits: FrameLimits,
) -> Result<usize, FrameError> {
    if payload.len() > limits.payload_bytes {
        return Err(FrameError::PayloadTooLarge {
            actual: payload.len(),
            limit: limits.payload_bytes,
        });
    }
    if attachments.len() > limits.attachments {
        return Err(FrameError::TooManyAttachments {
            actual: attachments.len(),
            limit: limits.attachments,
        });
    }

    let mut total_bytes = payload.len();
    for (index, attachment) in attachments.iter().enumerate() {
        if attachment.len() > limits.attachment_bytes {
            return Err(FrameError::AttachmentTooLarge {
                index,
                actual: attachment.len(),
                limit: limits.attachment_bytes,
            });
        }
        total_bytes = total_bytes
            .checked_add(attachment.len())
            .ok_or(FrameError::TotalLengthOverflow)?;
    }
    if total_bytes > limits.total_bytes {
        return Err(FrameError::TotalBytesTooLarge {
            actual: total_bytes,
            limit: limits.total_bytes,
        });
    }

    Ok(total_bytes)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FrameError {
    PayloadTooLarge {
        actual: usize,
        limit: usize,
    },
    TooManyAttachments {
        actual: usize,
        limit: usize,
    },
    AttachmentTooLarge {
        index: usize,
        actual: usize,
        limit: usize,
    },
    TotalLengthOverflow,
    TotalBytesTooLarge {
        actual: usize,
        limit: usize,
    },
    DeclaredLengthTooLarge {
        declared: usize,
        limit: usize,
    },
    AllocationFailed {
        declared: usize,
    },
    FragmentOverrun {
        declared: usize,
        attempted: usize,
    },
    IncompleteFragments {
        declared: usize,
        received: usize,
    },
    ReservationSizeMismatch {
        reserved: usize,
        actual: usize,
    },
    ReservationTooSmall {
        reserved: usize,
        actual: usize,
    },
    Resource(ResourceError),
}

impl Display for FrameError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::PayloadTooLarge { actual, limit } => {
                write!(
                    formatter,
                    "payload has {actual} bytes, exceeding limit {limit}"
                )
            }
            Self::TooManyAttachments { actual, limit } => write!(
                formatter,
                "frame has {actual} attachments, exceeding limit {limit}"
            ),
            Self::AttachmentTooLarge {
                index,
                actual,
                limit,
            } => write!(
                formatter,
                "attachment {index} has {actual} bytes, exceeding limit {limit}"
            ),
            Self::TotalLengthOverflow => formatter.write_str("frame byte length overflowed"),
            Self::TotalBytesTooLarge { actual, limit } => write!(
                formatter,
                "frame has {actual} aggregate bytes, exceeding limit {limit}"
            ),
            Self::DeclaredLengthTooLarge { declared, limit } => write!(
                formatter,
                "declared fragment length {declared} exceeds limit {limit}"
            ),
            Self::AllocationFailed { declared } => {
                write!(formatter, "could not reserve {declared} fragment bytes")
            }
            Self::FragmentOverrun {
                declared,
                attempted,
            } => write!(
                formatter,
                "fragment stream attempted {attempted} bytes for declared length {declared}"
            ),
            Self::IncompleteFragments { declared, received } => write!(
                formatter,
                "fragment stream ended at {received} bytes before declared length {declared}"
            ),
            Self::ReservationSizeMismatch { reserved, actual } => write!(
                formatter,
                "frame reserved {reserved} bytes but retained {actual} bytes"
            ),
            Self::ReservationTooSmall { reserved, actual } => write!(
                formatter,
                "frame reserved at most {reserved} bytes but retained {actual} bytes"
            ),
            Self::Resource(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for FrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Resource(error) => Some(error),
            _ => None,
        }
    }
}

impl From<ResourceError> for FrameError {
    fn from(error: ResourceError) -> Self {
        Self::Resource(error)
    }
}

/// Exact-length assembly for one bounded payload or attachment.
pub struct BoundedBytesAssembler {
    declared: usize,
    bytes: Vec<u8>,
    wire_bytes: Option<ResourceLease>,
}

/// Exact assembled bytes retaining any shared wire-budget lease.
pub struct AssembledBytes {
    bytes: Box<[u8]>,
    _wire_bytes: Option<ResourceLease>,
}

impl AssembledBytes {
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

impl std::ops::Deref for AssembledBytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_bytes()
    }
}

impl fmt::Debug for AssembledBytes {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AssembledBytes")
            .field("len", &self.bytes.len())
            .finish_non_exhaustive()
    }
}

impl BoundedBytesAssembler {
    pub fn new(declared: usize, limit: usize) -> Result<Self, FrameError> {
        Self::validate_declared(declared, limit)?;
        Self::allocate(declared, None)
    }

    /// Reserves the complete declared byte length before allocating the
    /// fragment buffer. The returned assembled value retains that lease.
    pub fn new_budgeted(
        declared: usize,
        limit: usize,
        ledger: &ResourceLedger,
        scope: ResourceScope,
    ) -> Result<Self, FrameError> {
        Self::validate_declared(declared, limit)?;
        let units = u64::try_from(declared).map_err(|_| FrameError::TotalLengthOverflow)?;
        let wire_bytes = if units == 0 {
            None
        } else {
            Some(ledger.reserve(scope, ResourceClass::WireBytes, units)?)
        };
        Self::allocate(declared, wire_bytes)
    }

    fn validate_declared(declared: usize, limit: usize) -> Result<(), FrameError> {
        if declared > limit {
            return Err(FrameError::DeclaredLengthTooLarge { declared, limit });
        }
        Ok(())
    }

    fn allocate(declared: usize, wire_bytes: Option<ResourceLease>) -> Result<Self, FrameError> {
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(declared)
            .map_err(|_| FrameError::AllocationFailed { declared })?;
        Ok(Self {
            declared,
            bytes,
            wire_bytes,
        })
    }

    pub fn push(&mut self, chunk: &[u8]) -> Result<(), FrameError> {
        let attempted = self
            .bytes
            .len()
            .checked_add(chunk.len())
            .ok_or(FrameError::TotalLengthOverflow)?;
        if attempted > self.declared {
            return Err(FrameError::FragmentOverrun {
                declared: self.declared,
                attempted,
            });
        }
        self.bytes.extend_from_slice(chunk);
        Ok(())
    }

    pub fn received(&self) -> usize {
        self.bytes.len()
    }

    pub fn remaining(&self) -> usize {
        self.declared - self.bytes.len()
    }

    pub fn finish(self) -> Result<AssembledBytes, FrameError> {
        if self.bytes.len() != self.declared {
            return Err(FrameError::IncompleteFragments {
                declared: self.declared,
                received: self.bytes.len(),
            });
        }
        Ok(AssembledBytes {
            bytes: self.bytes.into_boxed_slice(),
            _wire_bytes: self.wire_bytes,
        })
    }
}

#[cfg(test)]
mod tests {
    use pwright_core::{
        CallId, MethodTag, ObjectId, ProtocolVersion, ResourceClass, ResourceError, ResourceLedger,
        ResourceLimits, ResourceScope,
    };

    use super::{BoundedBytesAssembler, FrameError, FrameLimits, FrameRoute, WireFrame};

    fn limits() -> FrameLimits {
        FrameLimits::new(8, 2, 4, 12)
    }

    fn route() -> FrameRoute {
        FrameRoute::Command {
            call_id: CallId::new(1),
            target: ObjectId::new(2),
            method: MethodTag::new(3),
        }
    }

    fn wire_ledger(global: u64, per_scope: u64) -> ResourceLedger {
        ResourceLedger::new(ResourceLimits::deny_all(4, 4).with_class(
            ResourceClass::WireBytes,
            global,
            per_scope,
        ))
    }

    #[test]
    fn frame_preserves_typed_route_payload_and_attachments() {
        // Arrange
        let version = ProtocolVersion::new(1, 2);

        // Act
        let frame = WireFrame::new(
            version,
            route(),
            vec![1, 2, 3],
            vec![vec![4, 5], vec![6]],
            limits(),
        )
        .expect("bounded frame should build");

        // Assert
        assert_eq!(frame.version(), version);
        assert_eq!(frame.route(), route());
        assert_eq!(frame.payload(), &[1, 2, 3]);
        assert_eq!(frame.attachments()[0].as_bytes(), &[4, 5]);
        assert_eq!(frame.attachments()[1].as_bytes(), &[6]);
        assert_eq!(frame.total_bytes(), 6);
    }

    #[test]
    fn budgeted_frame_and_its_clones_share_one_retained_byte_lease() {
        // Arrange
        let ledger = wire_ledger(12, 12);
        let frame = WireFrame::new_budgeted(
            ProtocolVersion::new(1, 0),
            route(),
            vec![1, 2, 3],
            vec![vec![4, 5, 6]],
            limits(),
            &ledger,
            ResourceScope::Call(CallId::new(1)),
        )
        .expect("six wire bytes should fit");

        // Act
        let clone = frame.clone();
        drop(frame);
        let while_cloned = ledger.metrics().expect("metrics should be available");
        drop(clone);
        let after_last_drop = ledger.metrics().expect("metrics should be available");

        // Assert
        assert_eq!(while_cloned.class(ResourceClass::WireBytes).current, 6);
        assert_eq!(while_cloned.active_leases, 1);
        assert_eq!(after_last_drop.class(ResourceClass::WireBytes).current, 0);
        assert_eq!(after_last_drop.active_leases, 0);
        assert_eq!(after_last_drop.class(ResourceClass::WireBytes).peak, 6);
    }

    #[test]
    fn budgeted_frame_records_exact_ledger_identity_even_without_bytes() {
        // Arrange
        let ledger = wire_ledger(12, 12);
        let same_budget = ledger.clone();
        let different_budget = wire_ledger(12, 12);
        let bytes = WireFrame::new_budgeted(
            ProtocolVersion::new(1, 0),
            route(),
            vec![1],
            Vec::new(),
            limits(),
            &ledger,
            ResourceScope::Call(CallId::new(1)),
        )
        .expect("one byte should fit");
        let empty = WireFrame::new_budgeted(
            ProtocolVersion::new(1, 0),
            FrameRoute::Control,
            Vec::new(),
            Vec::new(),
            limits(),
            &ledger,
            ResourceScope::Connection,
        )
        .expect("empty budgeted frame should build");
        let unbudgeted = WireFrame::new(
            ProtocolVersion::new(1, 0),
            route(),
            vec![1],
            Vec::new(),
            limits(),
        )
        .expect("unbudgeted frame should build");

        // Act
        let observations = (
            bytes.is_budgeted_by(&same_budget),
            bytes.is_budgeted_by(&different_budget),
            empty.is_budgeted_by(&ledger),
            unbudgeted.is_budgeted_by(&ledger),
        );

        // Assert
        assert_eq!(observations, (true, false, true, false));
    }

    #[test]
    fn shared_wire_budget_rejects_aggregate_frames_before_retention() {
        // Arrange
        let ledger = wire_ledger(8, 8);
        let first = WireFrame::new_budgeted(
            ProtocolVersion::new(1, 0),
            route(),
            vec![1, 2, 3, 4],
            Vec::new(),
            limits(),
            &ledger,
            ResourceScope::Call(CallId::new(1)),
        )
        .expect("first frame should fit");

        // Act
        let error = WireFrame::new_budgeted(
            ProtocolVersion::new(1, 0),
            FrameRoute::Result {
                call_id: CallId::new(2),
            },
            vec![5, 6, 7, 8, 9],
            Vec::new(),
            limits(),
            &ledger,
            ResourceScope::Call(CallId::new(2)),
        )
        .expect_err("aggregate retained frames should exceed the connection budget");

        // Assert
        assert_eq!(
            error,
            FrameError::Resource(ResourceError::GlobalLimitExceeded {
                class: ResourceClass::WireBytes,
                current: 4,
                requested: 5,
                limit: 8,
            })
        );
        assert_eq!(
            ledger
                .metrics()
                .expect("metrics should be available")
                .class(ResourceClass::WireBytes)
                .current,
            4
        );
        drop(first);
    }

    #[test]
    fn exact_preallocation_reservation_moves_into_the_finished_frame() {
        // Arrange
        let ledger = wire_ledger(8, 8);
        let reservation = WireFrame::reserve_bytes(&ledger, ResourceScope::Call(CallId::new(1)), 6)
            .expect("six bytes should reserve before allocation");

        // Act
        let frame = WireFrame::new_reserved(
            ProtocolVersion::new(1, 0),
            route(),
            vec![1, 2, 3],
            vec![vec![4, 5, 6]],
            limits(),
            reservation,
        )
        .expect("exact reservation should move into the frame");
        let retained = ledger.metrics().expect("metrics should be available");
        drop(frame);
        let released = ledger.metrics().expect("metrics should be available");

        // Assert
        assert_eq!(retained.class(ResourceClass::WireBytes).current, 6);
        assert_eq!(retained.active_leases, 1);
        assert_eq!(released.class(ResourceClass::WireBytes).current, 0);
        assert_eq!(released.active_leases, 0);
    }

    #[test]
    fn mismatched_preallocation_reservation_is_rejected_and_released() {
        // Arrange
        let ledger = wire_ledger(8, 8);
        let reservation = WireFrame::reserve_bytes(&ledger, ResourceScope::Call(CallId::new(1)), 6)
            .expect("six bytes should reserve before allocation");

        // Act
        let error = WireFrame::new_reserved(
            ProtocolVersion::new(1, 0),
            route(),
            vec![1, 2, 3],
            vec![vec![4, 5]],
            limits(),
            reservation,
        )
        .expect_err("five retained bytes must not consume a six-byte proof");

        // Assert
        assert_eq!(
            error,
            FrameError::ReservationSizeMismatch {
                reserved: 6,
                actual: 5,
            }
        );
        assert_eq!(
            ledger
                .metrics()
                .expect("metrics should be available")
                .class(ResourceClass::WireBytes)
                .current,
            0
        );
    }

    #[test]
    fn frame_rejects_payload_over_its_limit() {
        // Arrange
        let payload = vec![0; 9];

        // Act
        let error = WireFrame::new(
            ProtocolVersion::new(1, 0),
            route(),
            payload,
            Vec::new(),
            limits(),
        )
        .expect_err("oversized payload should fail");

        // Assert
        assert_eq!(
            error,
            FrameError::PayloadTooLarge {
                actual: 9,
                limit: 8
            }
        );
    }

    #[test]
    fn frame_rejects_attachment_count_and_individual_size() {
        // Arrange
        let too_many = vec![Vec::new(), Vec::new(), Vec::new()];
        let too_large = vec![vec![0; 5]];

        // Act
        let count_error = WireFrame::new(
            ProtocolVersion::new(1, 0),
            route(),
            Vec::new(),
            too_many,
            limits(),
        )
        .expect_err("attachment count should be bounded");
        let size_error = WireFrame::new(
            ProtocolVersion::new(1, 0),
            route(),
            Vec::new(),
            too_large,
            limits(),
        )
        .expect_err("attachment bytes should be bounded");

        // Assert
        assert_eq!(
            count_error,
            FrameError::TooManyAttachments {
                actual: 3,
                limit: 2
            }
        );
        assert_eq!(
            size_error,
            FrameError::AttachmentTooLarge {
                index: 0,
                actual: 5,
                limit: 4,
            }
        );
    }

    #[test]
    fn frame_rejects_aggregate_bytes_over_the_limit() {
        // Arrange
        let payload = vec![0; 8];
        let attachments = vec![vec![0; 4], vec![0; 1]];

        // Act
        let error = WireFrame::new(
            ProtocolVersion::new(1, 0),
            route(),
            payload,
            attachments,
            limits(),
        )
        .expect_err("aggregate bytes should be bounded");

        // Assert
        assert_eq!(
            error,
            FrameError::TotalBytesTooLarge {
                actual: 13,
                limit: 12,
            }
        );
    }

    #[test]
    fn assembler_accepts_fragmented_exact_length_input() {
        // Arrange
        let mut assembler = BoundedBytesAssembler::new(5, 8).expect("declared size should fit");

        // Act
        assembler.push(&[1, 2]).expect("first chunk should fit");
        assembler
            .push(&[3, 4, 5])
            .expect("second chunk should complete");
        let bytes = assembler.finish().expect("exact input should finish");

        // Assert
        assert_eq!(&*bytes, &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn budgeted_assembler_retains_declared_bytes_through_finished_output() {
        // Arrange
        let ledger = wire_ledger(5, 5);
        let scope = ResourceScope::Call(CallId::new(1));
        let mut assembler = BoundedBytesAssembler::new_budgeted(5, 8, &ledger, scope)
            .expect("declared bytes should reserve before assembly");
        let after_reserve = ledger.metrics().expect("resource metrics should load");
        assembler.push(&[1, 2]).expect("first fragment should fit");
        assembler
            .push(&[3, 4, 5])
            .expect("second fragment should fit");

        // Act
        let bytes = assembler.finish().expect("exact input should finish");
        let after_finish = ledger.metrics().expect("resource metrics should load");
        let scope_metrics = ledger
            .scope_metrics(scope)
            .expect("scope metrics should load")
            .expect("finished bytes should retain their scope");
        drop(bytes);
        let after_drop = ledger.metrics().expect("resource metrics should load");

        // Assert
        assert_eq!(after_reserve.class(ResourceClass::WireBytes).current, 5);
        assert_eq!(after_finish.class(ResourceClass::WireBytes).current, 5);
        assert_eq!(scope_metrics.units(ResourceClass::WireBytes), 5);
        assert_eq!(after_drop.class(ResourceClass::WireBytes).current, 0);
        assert_eq!(after_drop.class(ResourceClass::WireBytes).peak, 5);
        assert_eq!(after_drop.active_leases, 0);
    }

    #[test]
    fn budgeted_assembler_rejects_shared_limit_before_retaining_fragments() {
        // Arrange
        let ledger = wire_ledger(4, 4);

        // Act
        let error = BoundedBytesAssembler::new_budgeted(5, 8, &ledger, ResourceScope::Connection)
            .err()
            .expect("shared wire budget should reject the declaration");
        let metrics = ledger.metrics().expect("resource metrics should load");

        // Assert
        assert_eq!(
            error,
            FrameError::Resource(ResourceError::GlobalLimitExceeded {
                class: ResourceClass::WireBytes,
                current: 0,
                requested: 5,
                limit: 4,
            })
        );
        assert_eq!(metrics.class(ResourceClass::WireBytes).current, 0);
        assert_eq!(metrics.class(ResourceClass::WireBytes).peak, 0);
        assert_eq!(metrics.rejected_reservations, 1);
    }

    #[test]
    fn incomplete_budgeted_assembly_releases_its_full_declaration() {
        // Arrange
        let ledger = wire_ledger(3, 3);
        let mut assembler =
            BoundedBytesAssembler::new_budgeted(3, 3, &ledger, ResourceScope::Connection)
                .expect("declared bytes should reserve");
        assembler.push(&[1]).expect("partial fragment should fit");

        // Act
        let error = assembler
            .finish()
            .expect_err("incomplete input should fail and release ownership");
        let metrics = ledger.metrics().expect("resource metrics should load");

        // Assert
        assert_eq!(
            error,
            FrameError::IncompleteFragments {
                declared: 3,
                received: 1,
            }
        );
        assert_eq!(metrics.class(ResourceClass::WireBytes).current, 0);
        assert_eq!(metrics.class(ResourceClass::WireBytes).peak, 3);
        assert_eq!(metrics.active_leases, 0);
    }

    #[test]
    fn assembler_rejects_declared_size_before_allocation() {
        // Arrange
        let declared = 9;

        // Act
        let error = BoundedBytesAssembler::new(declared, 8)
            .err()
            .expect("oversized declaration should fail");

        // Assert
        assert_eq!(
            error,
            FrameError::DeclaredLengthTooLarge {
                declared: 9,
                limit: 8,
            }
        );
    }

    #[test]
    fn assembler_rejects_overrun_without_appending_the_chunk() {
        // Arrange
        let mut assembler = BoundedBytesAssembler::new(3, 3).expect("declared size should fit");
        assembler.push(&[1, 2]).expect("first chunk should fit");

        // Act
        let error = assembler
            .push(&[3, 4])
            .expect_err("overrunning chunk should fail");

        // Assert
        assert_eq!(
            error,
            FrameError::FragmentOverrun {
                declared: 3,
                attempted: 4,
            }
        );
        assert_eq!(assembler.received(), 2);
        assert_eq!(assembler.remaining(), 1);
    }

    #[test]
    fn assembler_rejects_incomplete_input() {
        // Arrange
        let mut assembler = BoundedBytesAssembler::new(3, 3).expect("declared size should fit");
        assembler.push(&[1]).expect("partial chunk should fit");

        // Act
        let error = assembler
            .finish()
            .expect_err("incomplete input should not become a frame part");

        // Assert
        assert_eq!(
            error,
            FrameError::IncompleteFragments {
                declared: 3,
                received: 1,
            }
        );
    }
}
