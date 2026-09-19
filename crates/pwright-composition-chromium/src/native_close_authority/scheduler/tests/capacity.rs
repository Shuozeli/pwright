//! Real CDP scheduler, native evidence and local runtime/topology capacity.
//! Publication uses a bounded test observer, not the Node host event queue.
use super::*;
use crate::close_capacity::tests::fixture as local;
use crate::{ChromiumCloseCapacityBundle, ChromiumClosePlan, ChromiumCloseTarget};

#[tokio::test]
async fn parent_dispatches_with_page_capacity_pending_then_commits_and_child_joins() {
    // Arrange: both native authority and reservations use the SAME topology,
    // moved intact into the authority before any native command is admitted.
    let local::Fixture {
        topology,
        topology_pool,
        runtime,
        runtime_pool,
        observer,
        resources,
    } = local::Fixture::new(2);
    let mut f =
        wire::GenericFixture::<ChromiumCloseCapacityBundle>::with_topology(2, topology).await;
    let reserve = |target| {
        ChromiumCloseCapacityBundle::reserve(
            target,
            &f.scheduler.authority.0.topology.lock().unwrap(),
            &topology_pool,
            &runtime_pool,
            &observer,
        )
        .unwrap()
    };
    let page = reserve(local::PAGE);
    let context = reserve(local::CONTEXT);
    assert_eq!(runtime.lifecycle_transaction_active(), Ok(false));
    assert!(f.scheduler.push_page(PAGE, page).is_ok());
    acknowledge_page(&mut f).await;
    assert!(f.scheduler.push_context(CONTEXT, context).is_ok());
    // Act: the ancestor must reach the wire while Page destruction is withheld.
    let packet = command(&mut f).await;
    // Assert: capacity alone did not retire anything or freeze either domain.
    assert_eq!(packet["method"], "Target.disposeBrowserContext");
    assert_eq!(runtime.is_object_live(PAGE), Ok(true));
    assert_eq!(runtime.lifecycle_transaction_active(), Ok(false));
    assert_eq!(topology_pool.pending(), 2);
    assert_eq!(runtime_pool.pending(), 2);
    f.peer.reply(&packet, json!({})).await;
    let a = next(&mut f).await;
    let b = next(&mut f).await;
    let (page, context) = if a.payload().target() == local::PAGE {
        (a, b)
    } else {
        (b, a)
    };
    for (completion, expected) in [(context, vec![4, 3, 2]), (page, vec![])] {
        let (publication, update, mut retired) = completion.consume(|bundle, result| {
            let proof = result.as_ref().unwrap();
            proof
                .with_topology(
                    &f.scheduler.authority.0.connection,
                    &f.scheduler.authority.0.topology,
                    |native, topology| {
                        let parts = bundle.into_parts();
                        let object = match native {
                            NativeCloseCommitTarget::Page { object, .. } => {
                                assert_eq!(parts.target, ChromiumCloseTarget::Page(object));
                                object
                            }
                            NativeCloseCommitTarget::Context { object, .. } => {
                                assert_eq!(
                                    parts.target,
                                    ChromiumCloseTarget::BrowserContext(object)
                                );
                                object
                            }
                        };
                        let runtime = parts
                            .runtime
                            .prepare(&runtime.lifecycle_handle(), object)
                            .unwrap();
                        let prepared = parts
                            .topology
                            .prepare(topology, |next| match native {
                                NativeCloseCommitTarget::Context {
                                    browser_context_id, ..
                                } => next.close_context(EPOCH, browser_context_id),
                                NativeCloseCommitTarget::Page { target_id, .. } => {
                                    next.detach_target(EPOCH, "session-a", Some(target_id))
                                }
                            })
                            .unwrap();
                        let publication = parts
                            .publication
                            .prepare(ChromiumClosePlan::new(
                                parts.target,
                                prepared.output().effects(),
                            ))
                            .unwrap();
                        let (update, retired) = runtime
                            .commit_deferred_with_publication(None, || prepared.commit(topology))
                            .unwrap()
                            .unwrap();
                        (publication, update, retired)
                    },
                )
                .unwrap()
        });
        // No native/topology/runtime lock crosses user notification.
        assert!(f.scheduler.authority.0.topology.try_lock().is_ok());
        publication.committed(update.effects()).unwrap();
        assert_eq!(
            retired
                .removed()
                .iter()
                .map(|record| record.id().get())
                .collect::<Vec<_>>(),
            expected
        );
        retired.notify();
    }
    assert_eq!(runtime.is_object_live(PAGE), Ok(false));
    assert_eq!(runtime.is_object_live(CONTEXT), Ok(false));
    assert_eq!(observer.0.committed.load(Ordering::Acquire), 2);
    assert_eq!(observer.0.pending.load(Ordering::Acquire), 0);
    assert_eq!(topology_pool.pending(), 0);
    assert_eq!(runtime_pool.pending(), 0);
    assert_eq!(f.scheduler.admission.pending.load(Ordering::Acquire), 0);
    assert!(f.peer.no_commands());
    f.finish().await;
    drop((observer, runtime_pool, runtime, topology_pool));
    let metrics = resources.metrics().unwrap();
    assert_eq!(metrics.active_leases, 0);
    for class in ResourceClass::ALL {
        assert_eq!(metrics.class(class).current, 0);
    }
}
