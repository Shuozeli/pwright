//! Executor-neutral, bounded native work. No topology/runtime/publication commit.
//! Polling can be cancelled and resumed: command futures remain in this owner.
//! Drain the owner before dropping it once commands have been polled/written.
use super::*;
use pwright_backend_cdp_page_lifecycle::{
    PageCloseFuture,
    native_confirmation::{
        CloseInterruptionFuture, NativePageCloseCommandFuture, NativePageCloseConfirmation,
        NativePageCloseProfile,
    },
};
use pwright_cap_page_lifecycle::PageLifecycleErrorDetails;
use std::{
    marker::PhantomData,
    task::{Context, Poll},
};

/// T is the caller's retained admission/reply payload. Its inline layout is
/// included; any heap storage owned by T must have its own prior admission.
pub struct NativeCloseSchedulerProfile<T> {
    scope: ResourceScope,
    pending: NonZeroU16,
    payload: PhantomData<fn() -> T>,
}
impl<T> NativeCloseSchedulerProfile<T> {
    pub fn new(scope: ResourceScope, pending: NonZeroU16) -> Self {
        Self {
            scope,
            pending,
            payload: PhantomData,
        }
    }
    fn state_bytes(&self) -> u64 {
        (std::mem::size_of::<NativeCloseScheduler<T>>()
            + std::mem::size_of::<Admission>()
            + 4 * std::mem::size_of::<usize>()
            + usize::from(self.pending.get()) * std::mem::size_of::<Option<Entry<T>>>())
            as u64
    }
    fn slot_bytes(&self) -> u64 {
        // Fixed vector, returned completion, Arc headers and shared Context
        // receipt may coexist. Leaf command/binding bodies have separate leases.
        (std::mem::size_of::<NativeCloseCompletion<T>>()
            + std::mem::size_of::<Slot>()
            + std::mem::size_of::<SharedContext>()
            + std::mem::size_of::<NativePageCloseBinding>()
            + 8 * std::mem::size_of::<usize>()
            + 8192) as u64
    }
    pub fn requirements(&self) -> ResourceRequirements {
        let bytes = self.state_bytes() + u64::from(self.pending.get()) * self.slot_bytes();
        ResourceRequirements::none()
            .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
            .with_metadata(1, usize::from(self.pending.get()) + 1)
    }
}

struct Admission {
    ledger: ResourceLedger,
    scope: ResourceScope,
    limit: usize,
    slot_bytes: u64,
    pending: AtomicUsize,
    _storage: ResourceLease,
}
struct Slot {
    // Storage is returned before Drop on Pending makes the slot reusable.
    _storage: ResourceLease,
    _pending: SlotPending,
}
struct SlotPending(Arc<Admission>);
impl Drop for SlotPending {
    fn drop(&mut self) {
        self.0.pending.fetch_sub(1, Ordering::AcqRel);
    }
}
impl Admission {
    fn reserve(self: &Arc<Self>) -> Result<Arc<Slot>, Error> {
        self.pending
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < self.limit).then_some(n + 1)
            })
            .map_err(|_| {
                fail(
                    FailureKind::ResourceExhausted,
                    "native close scheduler capacity exhausted",
                )
            })?;
        let pending = SlotPending(self.clone());
        let storage = self
            .ledger
            .reserve(
                self.scope,
                ResourceClass::BackendStateBytes,
                self.slot_bytes,
            )
            .map_err(resource)?;
        Ok(Arc::new(Slot {
            _storage: storage,
            _pending: pending,
        }))
    }
}
struct SharedStop(Arc<dyn NativeCloseInterruption>);
impl NativeCloseInterruption for SharedStop {
    fn is_interrupted(&self) -> bool {
        self.0.is_interrupted()
    }
    fn interrupted(
        &self,
    ) -> Result<CloseInterruptionFuture, OperationError<PageLifecycleErrorDetails>> {
        self.0.interrupted()
    }
}

/// One attachment-derived Page source and ancestor domain. No per-call tasks,
/// unbounded completion channel, or mutation gates are retained by this owner.
pub struct NativeCloseScheduler<T> {
    entries: Vec<Option<Entry<T>>>,
    ready_cursor: usize,
    authority: NativeCloseAuthority,
    pages: NativePageCloseConfirmation,
    admission: Arc<Admission>,
}
struct Entry<T> {
    payload: T,
    stage: Stage,
    slot: Arc<Slot>,
}
enum Stage {
    Transition,
    Page {
        binding: Arc<NativePageCloseBinding>,
        phase: PagePhase,
        ancestor: Option<Arc<SharedContext>>,
    },
    Context(NativeContextCloseFuture),
    Complete(Result<NativeCloseEvidence, Error>),
}
enum PagePhase {
    Command {
        future: NativePageCloseCommandFuture,
        started: bool,
    },
    Destruction(PageCloseFuture),
}
struct SharedContext {
    receipt: NativeContextCloseReceipt,
    // Keep both its byte lease and its scheduler permit while children use it.
    _slot: Arc<Slot>,
}

pub struct NativeCloseCompletion<T> {
    payload: T,
    result: Result<NativeCloseEvidence, Error>,
    _slot: Arc<Slot>,
}
impl<T> NativeCloseCompletion<T> {
    pub fn payload(&self) -> &T {
        &self.payload
    }
    pub fn result(&self) -> &Result<NativeCloseEvidence, Error> {
        &self.result
    }
    /// Consume after synchronous retirement/publication handling. The native
    /// proof cannot be moved out to escape its retained scheduler admission.
    pub fn into_payload(self) -> T {
        self.payload
    }

    /// Move the payload into synchronous completion handling while borrowing
    /// native evidence. Evidence and scheduler admission remain alive throughout
    /// the callback, including when it consumes move-only capacity reservations.
    /// The borrowed evidence cannot escape through the returned value.
    ///
    /// This does not validate evidence, retire objects or publish close events.
    /// The callback must use the attachment-bound native commit scope and release
    /// all outer locks before notifying users. Callback/output-owned heaps need
    /// their own admission. Joined children may retain an ancestor's slot after
    /// this callback returns. No scheduler lock is held by this method.
    ///
    /// ```compile_fail
    /// use pwright_composition_chromium::native_close_authority::scheduler::{
    ///     NativeCloseCompletion, NativeCloseEvidence,
    /// };
    /// fn escape(completion: NativeCloseCompletion<()>) -> &'static NativeCloseEvidence {
    ///     completion.consume(|_, result| result.as_ref().unwrap())
    /// }
    /// ```
    pub fn consume<Output>(
        self,
        consume: impl FnOnce(T, &Result<NativeCloseEvidence, Error>) -> Output,
    ) -> Output {
        let output = consume(self.payload, &self.result);
        drop(self.result);
        drop(self._slot);
        output
    }
}
/// Native evidence only, with private constructors. Call validate immediately
/// before a serialized, latest-state retirement commit; never treat this as a
/// public close event or as permission to abandon a still-written command.
pub struct NativeCloseEvidence(Evidence);
enum Evidence {
    Page(Arc<NativePageCloseBinding>, Option<Arc<SharedContext>>),
    Context(Arc<SharedContext>),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeCloseTarget {
    Page(ObjectId),
    Context(ObjectId),
}
impl NativeCloseEvidence {
    pub fn target(&self) -> NativeCloseTarget {
        match &self.0 {
            Evidence::Page(page, _) => NativeCloseTarget::Page(page.page()),
            Evidence::Context(context) => NativeCloseTarget::Context(context.receipt.context()),
        }
    }
    pub fn validate(&self) -> Result<NativeCloseTarget, Error> {
        match &self.0 {
            Evidence::Page(page, Some(context)) => {
                context.receipt.certify_page(page)?;
            }
            Evidence::Page(page, None) => validate_page(page, false)?,
            Evidence::Context(context) => context.receipt.validate(false)?,
        }
        Ok(self.target())
    }

    fn state(&self) -> &Arc<State> {
        match &self.0 {
            Evidence::Page(page, _) => &page.ticket.pending.0,
            Evidence::Context(context) => context.receipt.state(),
        }
    }
}

mod commit_scope;
pub use commit_scope::NativeCloseCommitTarget;

impl AttachedChromiumComposition {
    /// Declare all three profiles plus Page event-subscriber capacity before
    /// attach. The scheduler cannot be paired with a foreign Page source.
    pub fn native_close_scheduler<T, I: NativeCloseInterruption + 'static>(
        &self,
        profile: NativeCloseSchedulerProfile<T>,
        authority: NativeCloseAuthorityProfile,
        pages: NativePageCloseProfile,
        interruption: I,
    ) -> Result<NativeCloseScheduler<T>, Error> {
        NativeCloseScheduler::bind(
            self.connection().clone(),
            self.shared_topology(),
            self.resources(),
            profile,
            authority,
            pages,
            interruption,
        )
    }
}
impl<T> NativeCloseScheduler<T> {
    fn bind<I: NativeCloseInterruption + 'static>(
        connection: Arc<CdpConnection>,
        topology: SharedChromiumTopology,
        resources: &ProductionResourceLedger,
        profile: NativeCloseSchedulerProfile<T>,
        authority: NativeCloseAuthorityProfile,
        pages: NativePageCloseProfile,
        interruption: I,
    ) -> Result<Self, Error> {
        if authority.pending.get() < profile.pending.get()
            || pages.event_subscribers() < usize::from(profile.pending.get())
        {
            return Err(fail(
                FailureKind::ProtocolViolation,
                "native close subprofiles cannot cover scheduler capacity",
            ));
        }
        let ledger = resources.claim(profile.requirements()).map_err(resource)?;
        let storage = ledger
            .reserve(
                profile.scope,
                ResourceClass::BackendStateBytes,
                profile.state_bytes(),
            )
            .map_err(resource)?;
        let admission = Arc::new(Admission {
            ledger,
            scope: profile.scope,
            limit: usize::from(profile.pending.get()),
            slot_bytes: profile.slot_bytes(),
            pending: AtomicUsize::new(0),
            _storage: storage,
        });
        let authority = NativeCloseAuthority::bind(
            connection.clone(),
            topology,
            resources,
            authority,
            interruption,
        )?;
        let pages = NativePageCloseConfirmation::new(
            connection,
            resources,
            pages,
            SharedStop(authority.0.interruption.clone()),
        )
        .map_err(page_error)?;
        let mut entries = Vec::with_capacity(admission.limit);
        entries.resize_with(admission.limit, || None);
        Ok(Self {
            entries,
            ready_cursor: 0,
            authority,
            pages,
            admission,
        })
    }
    pub fn is_empty(&self) -> bool {
        self.entries.iter().all(Option::is_none)
    }

    /// Failure returns the payload untouched. No command is written by admission.
    pub fn push_page(&mut self, page: ObjectId, payload: T) -> Result<(), (T, Error)> {
        let admitted = (|| {
            self.authority.0.ensure_live()?;
            let slot = self.admission.reserve()?;
            let binding = Arc::new(self.authority.bind_page(page)?);
            let before_native = binding.clone();
            let phase = PagePhase::Command {
                future: self
                    .pages
                    .prepare_command_with_validation(binding.target_id(), move || {
                        validate_page(&before_native, true).map_err(|error| {
                            OperationError::new(error.kind(), error.message(), None)
                        })
                    })
                    .map_err(page_error)?
                    .start(),
                started: false,
            };
            Ok((
                slot,
                Stage::Page {
                    binding,
                    phase,
                    ancestor: None,
                },
            ))
        })();
        self.insert(payload, admitted)
    }
    pub fn push_context(&mut self, context: ObjectId, payload: T) -> Result<(), (T, Error)> {
        let admitted = (|| {
            self.authority.0.ensure_live()?;
            let slot = self.admission.reserve()?;
            let future = self.authority.prepare_context(context)?.start();
            Ok((slot, Stage::Context(future)))
        })();
        self.insert(payload, admitted)
    }
    fn insert(
        &mut self,
        payload: T,
        admitted: Result<(Arc<Slot>, Stage), Error>,
    ) -> Result<(), (T, Error)> {
        let (slot, stage) = match admitted {
            Ok(value) => value,
            Err(error) => return Err((payload, error)),
        };
        let Some(entry) = self.entries.iter_mut().find(|entry| entry.is_none()) else {
            return Err((
                payload,
                fail(
                    FailureKind::ResourceExhausted,
                    "native scheduler has no free entry",
                ),
            ));
        };
        *entry = Some(Entry {
            payload,
            stage,
            slot,
        });
        Ok(())
    }

    /// Poll every native operation before returning one completion. A stalled
    /// Page never prevents Context dispatch. Cancelling a poll_fn wrapping this
    /// method does not cancel or remove any native operation.
    pub fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<NativeCloseCompletion<T>>> {
        self.poll_contexts(cx);
        for entry in self.entries.iter_mut().flatten() {
            poll_page(entry, &self.pages, cx);
        }
        let index = (0..self.entries.len())
            .map(|offset| (offset + self.ready_cursor) % self.entries.len())
            .find(|index| {
                matches!(
                    &self.entries[*index],
                    Some(Entry {
                        stage: Stage::Complete(_),
                        ..
                    })
                )
            });
        if let Some(index) = index {
            // Refilled low slots cannot starve an older ready completion.
            self.ready_cursor = (index + 1) % self.entries.len();
            let entry = self.entries[index]
                .take()
                .expect("located complete native entry");
            let Stage::Complete(result) = entry.stage else {
                unreachable!("located complete native stage")
            };
            Poll::Ready(Some(NativeCloseCompletion {
                payload: entry.payload,
                result,
                _slot: entry.slot,
            }))
        } else if self.is_empty() {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
    fn poll_contexts(&mut self, cx: &mut Context<'_>) {
        for index in 0..self.entries.len() {
            let receipt = {
                let Some(entry) = &mut self.entries[index] else {
                    continue;
                };
                let Stage::Context(future) = &mut entry.stage else {
                    continue;
                };
                let Poll::Ready(result) = future.as_mut().poll(cx) else {
                    continue;
                };
                match result {
                    Err(error) => {
                        entry.stage = Stage::Complete(Err(error));
                        continue;
                    }
                    Ok(receipt) => Arc::new(SharedContext {
                        receipt,
                        _slot: entry.slot.clone(),
                    }),
                }
            };
            // No native await or lifecycle mutation between certification and
            // linking. Revalidate again when the Page settles and at commit.
            for entry in self.entries.iter_mut().flatten() {
                if let Stage::Page {
                    binding, ancestor, ..
                } = &mut entry.stage
                    && receipt.receipt.certify_page(binding).is_ok()
                {
                    *ancestor = Some(receipt.clone());
                }
            }
            self.entries[index]
                .as_mut()
                .expect("retained Context entry")
                .stage = Stage::Complete(Ok(NativeCloseEvidence(Evidence::Context(receipt))));
        }
    }
}

fn poll_page<T>(entry: &mut Entry<T>, source: &NativePageCloseConfirmation, cx: &mut Context<'_>) {
    let Stage::Page {
        binding,
        phase,
        ancestor,
    } = &mut entry.stage
    else {
        return;
    };
    let Poll::Ready(result) = poll_page_native(binding, phase, ancestor.as_deref(), source, cx)
    else {
        return;
    };
    let old = std::mem::replace(&mut entry.stage, Stage::Transition);
    let Stage::Page {
        binding, ancestor, ..
    } = old
    else {
        unreachable!("retained native Page entry")
    };
    entry.stage = Stage::Complete(result.map(|from_ancestor| {
        NativeCloseEvidence(Evidence::Page(
            binding,
            if from_ancestor { ancestor } else { None },
        ))
    }));
}

fn poll_page_native(
    binding: &NativePageCloseBinding,
    phase: &mut PagePhase,
    ancestor: Option<&SharedContext>,
    source: &NativePageCloseConfirmation,
    cx: &mut Context<'_>,
) -> Poll<Result<bool, Error>> {
    let ancestor_valid =
        || ancestor.is_some_and(|context| context.receipt.certify_page(binding).is_ok());
    if let PagePhase::Command { future, started } = phase {
        if !*started {
            // An unpolled child has no written command to retain. Otherwise,
            // validate current identity once before the first native poll.
            if ancestor_valid() {
                return Poll::Ready(Ok(true));
            }
            if let Err(error) = validate_page(binding, true) {
                return Poll::Ready(Err(error));
            }
            *started = true;
        }
        // Never short-circuit a written command, even with valid ancestor proof.
        let Poll::Ready(result) = future.as_mut().poll(cx) else {
            return Poll::Pending;
        };
        match result {
            Ok(ack) if ack.belongs_to(source) && ack.target_id() == binding.target_id() => {
                *phase = PagePhase::Destruction(ack.wait_for_destruction());
            }
            Ok(_) => {
                return Poll::Ready(Err(fail(
                    FailureKind::ProtocolViolation,
                    "native Page acknowledgement source mismatch",
                )));
            }
            Err(error) => {
                return Poll::Ready(if ancestor_valid() {
                    Ok(true)
                } else {
                    Err(page_error(error))
                });
            }
        }
    }
    if ancestor_valid() {
        return Poll::Ready(Ok(true));
    }
    let PagePhase::Destruction(future) = phase else {
        unreachable!("acknowledged native Page phase")
    };
    future.as_mut().poll(cx).map(|result| {
        result
            .map_err(page_error)
            .and_then(|()| validate_page(binding, false))
            .map(|()| false)
    })
}
fn page_error(error: OperationError<PageLifecycleErrorDetails>) -> Error {
    let text = error.message();
    let mut end = text.len().min(1024);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    OperationError::new(error.kind(), &text[..end], None)
}
fn validate_page(page: &NativePageCloseBinding, require_present: bool) -> Result<(), Error> {
    let state = &page.ticket.pending.0;
    state.ensure_live()?;
    let topology = state.topology.lock().map_err(|_| poisoned())?;
    validate_page_topology(page, topology.topology(), require_present)
}

fn validate_page_topology(
    page: &NativePageCloseBinding,
    topology: &pwright_backend_cdp::CdpTopology,
    require_present: bool,
) -> Result<(), Error> {
    let state = &page.ticket.pending.0;
    if topology.epoch() != state.epoch
        || page.epoch != state.epoch
        || (require_present
            && (topology.page_object_id(&page.target) != Some(page.page)
                || topology.context_object_id(&page.native_context) != Some(page.context)))
        || topology
            .context_object_id(&page.native_context)
            .is_some_and(|id| id != page.context)
        || topology
            .page_object_id(&page.target)
            .is_some_and(|id| id != page.page)
        || topology
            .target_session_route(&page.session)
            .is_some_and(|route| route.page_target_id() != page.target.as_ref())
        || topology
            .browser_context_route(page.context)
            .is_some_and(|route| {
                route.browser_context_id() != page.native_context.as_ref()
                    || route.ownership() != CdpContextOwnership::Owned
            })
        || topology.page_session_routes().any(|route| {
            route.object_id() == page.page
                && (route.target_id() != page.target.as_ref()
                    || route.session_id() != page.session.as_ref()
                    || route.browser_context_object_id() != page.context)
        })
    {
        return Err(fail(
            FailureKind::TargetClosed,
            "native Page retirement identity changed",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
