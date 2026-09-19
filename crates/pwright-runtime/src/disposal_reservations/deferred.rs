//! Selected runtime commit peak and explicit post-lock notification ownership.
use super::*;

pub(super) fn commit_storage_bytes(
    objects: usize,
    calls: usize,
) -> Result<u64, DisposalReservationError> {
    let overflow = || DisposalReservationError::Invalid("deferred commit capacity overflow");
    // Vec growth has a minimum allocation for small non-ZST elements. Use at
    // least eight entries as well as doubling headroom. These are collection
    // payload envelopes, not an allocator metadata/physical-RAM guarantee.
    let grown = |n: usize| n.checked_mul(2).map(|n| n.max(8)).ok_or_else(overflow);
    let objects_grown = grown(objects)?;
    let calls_grown = grown(calls)?;
    // Current-order verification + graph disposal order + graph traversal stack
    // coexist. The reservation already charges its original disposal order.
    let object_scratch = objects_grown
        .checked_mul(2 * std::mem::size_of::<ObjectId>() + std::mem::size_of::<(ObjectId, bool)>())
        .and_then(|bytes| {
            objects
                .checked_mul(std::mem::size_of::<ObjectRecord<ObjectKindTag>>())
                .and_then(|removed| bytes.checked_add(removed))
        })
        .ok_or_else(overflow)?;
    let call_scratch = calls_grown
        .checked_mul(
            std::mem::size_of::<CallId>()
                + std::mem::size_of::<Arc<CancellationSignal>>()
                + std::mem::size_of::<Waker>(),
        )
        .ok_or_else(overflow)?;
    object_scratch
        .checked_add(call_scratch)
        .and_then(|bytes| bytes.checked_add(std::mem::size_of::<DeferredDisposalCommit>() + 256))
        .and_then(|bytes| u64::try_from(bytes).ok())
        .ok_or_else(overflow)
}

impl ReservedPreparedDisposal {
    /// Validate disposal, then run SHORT local publication and runtime
    /// retirement under the same runtime mutex. Transport shutdown cannot enter
    /// between the callback and retirement. The caller already holds the exact
    /// native/topology/route/Handle guards in its documented lock order.
    ///
    /// No native wait, runtime reentry, callback delivery or user waker is allowed
    /// inside `publish`, including capture destructors. All fallible publication
    /// preflight must precede its first mutation. Callback errors leave runtime state unchanged, but do not
    /// roll back external state the callback already changed. Panics poison the
    /// runtime and require terminal recovery; they are not transactional errors.
    ///
    /// Outer errors reject before publication; the inner Result is the caller's
    /// publication result. On success, release ALL outer locks before notifying
    /// the returned deferred owner. Callback capture/output/error heaps and any
    /// typed failure diagnostic require independent caller admission.
    pub fn commit_deferred_with_publication<Output, PublicationError>(
        self,
        failure: Option<RuntimeCallFailure>,
        publish: impl FnOnce() -> Result<Output, PublicationError>,
    ) -> Result<Result<(Output, DeferredDisposalCommit), PublicationError>, DisposalReservationError>
    {
        if self._permit.0.profile.deferred_calls.is_none() {
            return Err(DisposalReservationError::Invalid(
                "deferred commit was not admitted",
            ));
        }
        let result = self
            .transaction
            .commit_parts_with::<true, _, _>(failure, publish)?;
        match result {
            Ok((output, parts)) => Ok(Ok((
                output,
                DeferredDisposalCommit {
                    parts,
                    _memory: self._memory,
                    _permit: self._permit,
                },
            ))),
            Err(error) => {
                // Do not depend on a closure's capture drop order: bytes must
                // be returned before another caller can acquire this slot.
                drop(self._memory);
                drop(self._permit);
                Ok(Err(error))
            }
        }
    }

    /// Retire the current subtree and install cancellation signals without
    /// invoking user wakers. Notify only after all enclosing commit locks are
    /// released. The original reservation must select `with_deferred_commit`.
    pub fn commit_deferred(self) -> Result<DeferredDisposalCommit, DisposalReservationError> {
        self.commit_deferred_inner(None)
    }

    /// As above with an independently admitted caller-owned shared diagnostic.
    pub fn commit_deferred_with_failure(
        self,
        failure: RuntimeCallFailure,
    ) -> Result<DeferredDisposalCommit, DisposalReservationError> {
        self.commit_deferred_inner(Some(failure))
    }

    fn commit_deferred_inner(
        self,
        failure: Option<RuntimeCallFailure>,
    ) -> Result<DeferredDisposalCommit, DisposalReservationError> {
        if self._permit.0.profile.deferred_calls.is_none() {
            return Err(DisposalReservationError::Invalid(
                "deferred commit was not admitted",
            ));
        }
        let parts = self.transaction.commit_parts::<true>(failure)?;
        Ok(DeferredDisposalCommit {
            parts,
            _memory: self._memory,
            _permit: self._permit,
        })
    }
}

/// Retains the reservation's byte lease AND pending slot until all records and
/// saved wakers are dropped. Notification does not release this admission.
/// Drop never invokes a waker: the selecting coordinator must explicitly notify
/// after releasing every outer lock, including on post-commit error/teardown.
/// This is runtime retirement only, not native proof or event publication.
#[must_use = "notify after releasing enclosing locks, then drop retained records"]
pub struct DeferredDisposalCommit {
    parts: CommittedDisposalParts,
    _memory: ResourceLease,
    _permit: Permit,
}
impl DeferredDisposalCommit {
    pub fn removed(&self) -> &[ObjectRecord<ObjectKindTag>] {
        &self.parts.removed
    }
    /// Idempotent. A second call has no wakers to invoke.
    pub fn notify(&mut self) {
        self.parts.notify();
    }
}
