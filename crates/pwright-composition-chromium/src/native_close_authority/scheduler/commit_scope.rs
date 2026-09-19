//! Attachment-bound, synchronous native-evidence/current-topology scope.
use super::*;
use pwright_backend_cdp::ProductionCdpTopology;

/// Borrowed native keys from sealed scheduler evidence, not a current-route
/// lookup or a newly minted native receipt. The corresponding route can already
/// be absent after a parent-first local commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeCloseCommitTarget<'a> {
    Page {
        object: ObjectId,
        target_id: &'a str,
        session_id: &'a str,
    },
    Context {
        object: ObjectId,
        browser_context_id: &'a str,
    },
}

impl AttachedChromiumComposition {
    /// Validate sealed scheduler evidence against this exact attachment and
    /// execute a SHORT synchronous latest-state commit under its topology lock.
    /// Foreign connection/topology, stopped ownership, stale epochs and native
    /// identity rebindings reject before the callback is invoked.
    ///
    /// The callback must already own runtime/topology/publication capacity and
    /// its complete scratch budget. It must prepare/commit all local state with
    /// no native wait, release other locks, and return deferred notification
    /// owners for notification AFTER this method returns. A callback error or
    /// drop is not rollback of mutations the callback already committed.
    ///
    /// This supplies native proof + topology serialization, not runtime domain
    /// validation, publication, terminal settlement or a complete close service.
    /// No collection, native ID, subscriber, worker or lease pool is allocated.
    pub fn with_native_close_topology<Output>(
        &self,
        evidence: &NativeCloseEvidence,
        commit: impl FnOnce(NativeCloseCommitTarget<'_>, &mut ProductionCdpTopology) -> Output,
    ) -> Result<Output, Error> {
        evidence.with_topology(&self.connection, &self.topology, commit)
    }
}

impl NativeCloseEvidence {
    pub(super) fn with_topology<Output>(
        &self,
        connection: &Arc<CdpConnection>,
        topology: &SharedChromiumTopology,
        commit: impl FnOnce(NativeCloseCommitTarget<'_>, &mut ProductionCdpTopology) -> Output,
    ) -> Result<Output, Error> {
        let state = self.state();
        if !Arc::ptr_eq(connection, &state.connection) || !Arc::ptr_eq(topology, &state.topology) {
            return Err(fail(
                FailureKind::ProtocolViolation,
                "native close evidence belongs to another attachment",
            ));
        }
        let mut topology = topology.lock().map_err(|_| poisoned())?;
        // Recheck after lock acquisition: owner termination may have occurred
        // while this caller waited for the short topology scope.
        state.ensure_live()?;
        let target = match &self.0 {
            Evidence::Page(page, ancestor) => {
                if let Some(ancestor) = ancestor {
                    ancestor
                        .receipt
                        .certify_page_in_topology(page, topology.topology())?;
                } else {
                    validate_page_topology(page, topology.topology(), false)?;
                }
                NativeCloseCommitTarget::Page {
                    object: page.page(),
                    target_id: page.target_id(),
                    session_id: page.session_id(),
                }
            }
            Evidence::Context(context) => {
                context
                    .receipt
                    .validate_topology(topology.topology(), false)?;
                NativeCloseCommitTarget::Context {
                    object: context.receipt.context(),
                    browser_context_id: context.receipt.native_context(),
                }
            }
        };
        Ok(commit(target, &mut topology))
    }
}
