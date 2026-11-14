//! Fixed-width, caller-polled session initialization. No tasks or retries.
use std::future::Future;

use pwright_cdp::{connection::Result, generated::page::FrameTree};

pub(crate) async fn initialize_with_attachment(
    frame_tree: impl Future<Output = Result<FrameTree>>,
    runtime: impl Future<Output = Result<()>>,
    registration: impl Future<Output = Result<()>>,
    attachment: impl Future<Output = Result<()>>,
) -> Result<FrameTree> {
    // Recursive target events stay in the caller-owned driver's bounded inbox.
    // No world creation or ready publication may bypass an unanswered branch.
    let (tree, (), (), ()) = tokio::try_join!(frame_tree, runtime, registration, attachment)?;
    Ok(tree)
}

pub(crate) async fn initialize_with_registration(
    frame_tree: impl Future<Output = Result<FrameTree>>,
    runtime: impl Future<Output = Result<()>>,
    registration: impl Future<Output = Result<()>>,
) -> Result<FrameTree> {
    // Fixed width, same caller and cancellation owner as the paired variant.
    // Never publish readiness or create worlds from a partially completed join.
    let (tree, (), ()) = tokio::try_join!(frame_tree, runtime, registration)?;
    Ok(tree)
}

pub(crate) async fn initialize(
    parallel: bool,
    frame_tree: impl Future<Output = Result<FrameTree>>,
    runtime: impl Future<Output = Result<()>>,
) -> Result<FrameTree> {
    if parallel {
        // On error or owner cancellation, dropping the sibling future drops
        // its CDP pending registration. Queued wire bytes keep their own lease.
        let (tree, ()) = tokio::try_join!(frame_tree, runtime)?;
        Ok(tree)
    } else {
        let tree = frame_tree.await?;
        runtime.await?;
        Ok(tree)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        cell::Cell,
        future::{Future, poll_fn},
        pin::Pin,
        task::{Context, Poll, Waker},
    };

    use pwright_cdp::{
        connection::{CdpError, Result},
        generated::page::FrameTree,
    };

    use super::{initialize, initialize_with_attachment, initialize_with_registration};

    #[test]
    fn attachment_profile_requires_seven_pending_slots_and_is_opt_in() {
        // Arrange
        let serial = profile(6);
        // Act
        let result = serial.with_parallel_recursive_attachment();
        // Assert
        assert!(!serial.parallel_recursive_attachment());
        assert!(matches!(
            result,
            Err(
                crate::ChromiumTopologyEventDriverProfileError::InsufficientAttachmentCommands {
                    capacity: 6
                }
            )
        ));
    }

    #[test]
    fn attachment_profile_reserves_two_seven_owner_phases_with_idempotent_composition() {
        // Arrange
        let serial = profile(7);
        // Act
        let joined = serial.with_parallel_recursive_attachment().unwrap();
        // Assert
        assert!(joined.parallel_recursive_attachment());
        assert_eq!(
            joined.maximum_retained_bytes(),
            serial.maximum_retained_bytes() + 16 * 4096
        );
        assert_eq!(
            joined.requirements(),
            serial.requirements().with_class(
                pwright_core::ResourceClass::BackendStateBytes,
                joined.maximum_retained_bytes(),
                joined.maximum_retained_bytes()
            )
        );
        assert_eq!(joined.with_parallel_recursive_attachment().unwrap(), joined);
        assert_eq!(
            joined.with_parallel_utility_world_registration().unwrap(),
            joined
        );
        assert_eq!(
            joined.with_parallel_session_prerequisites().unwrap(),
            joined
        );
        assert_eq!(
            serial
                .with_parallel_utility_world_registration()
                .unwrap()
                .with_parallel_recursive_attachment()
                .unwrap(),
            joined
        );
    }

    #[test]
    fn each_pending_attachment_branch_blocks_world_creation_and_resume() {
        for pending in 0..4 {
            // Arrange
            let next_phase = Cell::new(false);
            let branch = |index| {
                poll_fn(move |_| {
                    if pending == index {
                        Poll::Pending
                    } else {
                        Poll::Ready(Ok(()))
                    }
                })
            };
            let mut work = Box::pin(async {
                initialize_with_attachment(
                    async {
                        branch(0).await?;
                        Ok(tree())
                    },
                    branch(1),
                    branch(2),
                    branch(3),
                )
                .await?;
                next_phase.set(true);
                Ok::<_, CdpError>(())
            });
            // Act
            let outcome = work.as_mut().poll(&mut Context::from_waker(Waker::noop()));
            // Assert
            assert!(outcome.is_pending(), "pending branch {pending}");
            assert!(!next_phase.get());
        }
    }

    #[test]
    fn attachment_error_or_owner_drop_cancels_every_sibling() {
        for failing in 0..=4 {
            // Arrange: 4 means no branch fails; cancellation belongs to the caller.
            let polled = std::array::from_fn::<_, 4, _>(|_| Cell::new(false));
            let dropped = std::array::from_fn::<_, 4, _>(|_| Cell::new(false));
            let mut results = std::array::from_fn::<_, 4, _>(|index| {
                (index == failing).then_some(Err(CdpError::Closed))
            });
            let [first, second, third, fourth] = &mut results;
            let mut work = Box::pin(initialize_with_attachment(
                async {
                    Gate {
                        polled: &polled[0],
                        dropped: &dropped[0],
                        result: first,
                    }
                    .await?;
                    Ok(tree())
                },
                Gate {
                    polled: &polled[1],
                    dropped: &dropped[1],
                    result: second,
                },
                Gate {
                    polled: &polled[2],
                    dropped: &dropped[2],
                    result: third,
                },
                Gate {
                    polled: &polled[3],
                    dropped: &dropped[3],
                    result: fourth,
                },
            ));
            // Act
            let outcome = work.as_mut().poll(&mut Context::from_waker(Waker::noop()));
            drop(work);
            // Assert
            if failing == 4 {
                assert!(outcome.is_pending());
                assert!(polled.iter().all(Cell::get));
            } else {
                assert!(matches!(outcome, Poll::Ready(Err(CdpError::Closed))));
            }
            assert!(dropped.iter().all(Cell::get));
        }
    }

    #[tokio::test]
    async fn attachment_success_returns_unchanged_tree() {
        // Arrange
        let expected = tree();
        // Act
        let actual = initialize_with_attachment(
            std::future::ready(Ok(expected.clone())),
            std::future::ready(Ok(())),
            std::future::ready(Ok(())),
            std::future::ready(Ok(())),
        )
        .await
        .unwrap();
        // Assert
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            serde_json::to_value(expected).unwrap()
        );
    }

    struct Gate<'a, T> {
        polled: &'a Cell<bool>,
        dropped: &'a Cell<bool>,
        result: &'a mut Option<Result<T>>,
    }

    impl<T> Future for Gate<'_, T> {
        type Output = Result<T>;

        fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
            let this = self.get_mut();
            this.polled.set(true);
            this.result.take().map_or(Poll::Pending, Poll::Ready)
        }
    }

    impl<T> Drop for Gate<'_, T> {
        fn drop(&mut self) {
            self.dropped.set(true);
        }
    }

    fn tree() -> FrameTree {
        serde_json::from_value(serde_json::json!({"frame": {
            "id": "fixture", "loaderId": "loader", "url": "about:blank",
            "domainAndRegistry": "", "securityOrigin": "://", "mimeType": "text/html",
            "secureContextType": "InsecureScheme", "crossOriginIsolatedContextType": "NotIsolated",
            "gatedAPIFeatures": []
        }}))
        .unwrap()
    }

    #[test]
    fn parallel_prerequisites_poll_both_and_owner_drop_cancels_both() {
        // Arrange
        let polled = [Cell::new(false), Cell::new(false)];
        let dropped = [Cell::new(false), Cell::new(false)];
        let mut frame_result = None;
        let mut runtime_result = None;
        let mut work = Box::pin(initialize(
            true,
            Gate {
                polled: &polled[0],
                dropped: &dropped[0],
                result: &mut frame_result,
            },
            Gate {
                polled: &polled[1],
                dropped: &dropped[1],
                result: &mut runtime_result,
            },
        ));
        // Act
        let outcome = work.as_mut().poll(&mut Context::from_waker(Waker::noop()));
        drop(work);
        // Assert
        assert!(outcome.is_pending());
        assert!(polled.iter().all(Cell::get));
        assert!(dropped.iter().all(Cell::get));
    }

    #[test]
    fn default_keeps_runtime_unpolled_until_frame_tree_finishes() {
        // Arrange
        let runtime_polled = Cell::new(false);
        let mut work = Box::pin(initialize(
            false,
            std::future::pending(),
            poll_fn(|_| {
                runtime_polled.set(true);
                Poll::Ready(Ok(()))
            }),
        ));
        // Act
        let outcome = work.as_mut().poll(&mut Context::from_waker(Waker::noop()));
        // Assert
        assert!(outcome.is_pending());
        assert!(!runtime_polled.get());
    }

    #[test]
    fn successful_frame_tree_cannot_bypass_pending_runtime() {
        // Arrange
        let mut work = Box::pin(initialize(
            true,
            std::future::ready(Ok(tree())),
            std::future::pending(),
        ));
        // Act
        let outcome = work.as_mut().poll(&mut Context::from_waker(Waker::noop()));
        // Assert
        assert!(outcome.is_pending());
    }

    #[test]
    fn successful_runtime_cannot_bypass_pending_frame_tree() {
        // Arrange
        let mut work = Box::pin(initialize(
            true,
            std::future::pending(),
            std::future::ready(Ok(())),
        ));
        // Act
        let outcome = work.as_mut().poll(&mut Context::from_waker(Waker::noop()));
        // Assert
        assert!(outcome.is_pending());
    }

    #[test]
    fn frame_tree_error_cancels_runtime() {
        // Arrange
        let dropped = Cell::new(false);
        let polled = Cell::new(false);
        let mut result = None;
        let mut work = Box::pin(initialize(
            true,
            std::future::ready(Err(CdpError::Closed)),
            Gate {
                polled: &polled,
                dropped: &dropped,
                result: &mut result,
            },
        ));
        // Act
        let outcome = work.as_mut().poll(&mut Context::from_waker(Waker::noop()));
        // Assert
        assert!(matches!(outcome, Poll::Ready(Err(CdpError::Closed))));
        assert!(dropped.get());
    }

    #[test]
    fn runtime_error_cancels_pending_frame_tree() {
        // Arrange
        let dropped = Cell::new(false);
        let polled = Cell::new(false);
        let mut result = None;
        let mut work = Box::pin(initialize(
            true,
            Gate {
                polled: &polled,
                dropped: &dropped,
                result: &mut result,
            },
            std::future::ready(Err(CdpError::Closed)),
        ));
        // Act
        let outcome = work.as_mut().poll(&mut Context::from_waker(Waker::noop()));
        // Assert
        assert!(matches!(outcome, Poll::Ready(Err(CdpError::Closed))));
        assert!(polled.get());
        assert!(dropped.get());
    }

    #[tokio::test]
    async fn successful_prerequisites_return_the_original_tree() {
        // Arrange
        let expected = tree();
        // Act
        let actual = initialize(
            true,
            std::future::ready(Ok(expected.clone())),
            std::future::ready(Ok(())),
        )
        .await
        .unwrap();
        // Assert
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            serde_json::to_value(expected).unwrap()
        );
    }

    fn profile(capacity: usize) -> crate::ChromiumTopologyEventDriverProductionProfile {
        let connection = pwright_cdp::CdpConnectionProductionProfile::new(
            pwright_core::ResourceScope::Connection,
            pwright_cdp::connection::ConnectionConfig {
                pending_command_capacity: capacity,
                max_response_bytes: 4096,
                ..Default::default()
            },
        );
        let topology = pwright_backend_cdp::CdpTopologyProductionProfile::new(
            pwright_backend_cdp::CdpTopologyLimits::new(16, 2, 2, 8, 8, 128),
            pwright_core::ResourceScope::extension(41, 1),
        )
        .unwrap();
        crate::ChromiumTopologyEventDriverProductionProfile::from_profiles(
            pwright_core::ResourceScope::extension(41, 2),
            &connection,
            topology,
        )
        .unwrap()
    }

    #[test]
    fn parallel_profile_requires_two_pending_command_slots() {
        // Arrange
        let serial = profile(1);
        // Act
        let result = serial.with_parallel_session_prerequisites();
        // Assert
        assert!(!serial.parallel_session_prerequisites());
        assert!(matches!(
            result,
            Err(
                crate::ChromiumTopologyEventDriverProfileError::InsufficientPendingCommands {
                    capacity: 1
                }
            )
        ));
    }

    #[test]
    fn parallel_profile_reserves_exactly_one_more_response_expansion() {
        // Arrange
        let serial = profile(2);
        // Act
        let parallel = serial.with_parallel_session_prerequisites().unwrap();
        // Assert
        assert!(parallel.parallel_session_prerequisites());
        assert_eq!(
            parallel.maximum_retained_bytes(),
            serial.maximum_retained_bytes() + 4 * 4096
        );
        assert_eq!(
            parallel.requirements(),
            serial.requirements().with_class(
                pwright_core::ResourceClass::BackendStateBytes,
                parallel.maximum_retained_bytes(),
                parallel.maximum_retained_bytes()
            )
        );
    }

    #[test]
    fn repeated_parallel_selection_does_not_double_charge() {
        // Arrange
        let parallel = profile(2).with_parallel_session_prerequisites().unwrap();
        // Act
        let repeated = parallel.with_parallel_session_prerequisites().unwrap();
        // Assert
        assert_eq!(repeated, parallel);
    }

    #[test]
    fn registration_profile_requires_three_pending_slots() {
        // Arrange
        let paired = profile(2).with_parallel_session_prerequisites().unwrap();
        // Act
        let result = paired.with_parallel_utility_world_registration();
        // Assert
        assert!(!paired.parallel_utility_world_registration());
        assert!(matches!(
            result,
            Err(
                crate::ChromiumTopologyEventDriverProfileError::InsufficientRegistrationCommands {
                    capacity: 2
                }
            )
        ));
    }

    #[test]
    fn registration_profile_reserves_two_more_expansions_without_double_charging() {
        // Arrange
        let serial = profile(3);
        // Act
        let triple = serial.with_parallel_utility_world_registration().unwrap();
        // Assert
        assert!(triple.parallel_session_prerequisites());
        assert!(triple.parallel_utility_world_registration());
        assert_eq!(
            triple.maximum_retained_bytes(),
            serial.maximum_retained_bytes() + 8 * 4096
        );
        assert_eq!(
            triple.requirements(),
            serial.requirements().with_class(
                pwright_core::ResourceClass::BackendStateBytes,
                triple.maximum_retained_bytes(),
                triple.maximum_retained_bytes()
            )
        );
        assert_eq!(
            triple.with_parallel_utility_world_registration().unwrap(),
            triple
        );
        assert_eq!(
            triple.with_parallel_session_prerequisites().unwrap(),
            triple
        );
        assert_eq!(
            serial
                .with_parallel_session_prerequisites()
                .unwrap()
                .with_parallel_utility_world_registration()
                .unwrap(),
            triple
        );
    }

    #[test]
    fn registration_join_polls_and_cancels_all_three_on_owner_drop() {
        // Arrange
        let polled = [Cell::new(false), Cell::new(false), Cell::new(false)];
        let dropped = [Cell::new(false), Cell::new(false), Cell::new(false)];
        let (mut tree_result, mut runtime_result, mut registration_result) = (None, None, None);
        let mut work = Box::pin(initialize_with_registration(
            Gate {
                polled: &polled[0],
                dropped: &dropped[0],
                result: &mut tree_result,
            },
            Gate {
                polled: &polled[1],
                dropped: &dropped[1],
                result: &mut runtime_result,
            },
            Gate {
                polled: &polled[2],
                dropped: &dropped[2],
                result: &mut registration_result,
            },
        ));
        // Act
        let outcome = work.as_mut().poll(&mut Context::from_waker(Waker::noop()));
        drop(work);
        // Assert
        assert!(outcome.is_pending());
        assert!(polled.iter().all(Cell::get));
        assert!(dropped.iter().all(Cell::get));
    }

    #[test]
    fn each_pending_prerequisite_prevents_the_next_phase() {
        for pending in 0..3 {
            // Arrange
            let next_phase = Cell::new(false);
            let mut work = Box::pin(async {
                initialize_with_registration(
                    poll_fn(|_| {
                        if pending == 0 {
                            Poll::Pending
                        } else {
                            Poll::Ready(Ok(tree()))
                        }
                    }),
                    poll_fn(|_| {
                        if pending == 1 {
                            Poll::Pending
                        } else {
                            Poll::Ready(Ok(()))
                        }
                    }),
                    poll_fn(|_| {
                        if pending == 2 {
                            Poll::Pending
                        } else {
                            Poll::Ready(Ok(()))
                        }
                    }),
                )
                .await?;
                next_phase.set(true);
                Ok::<_, CdpError>(())
            });
            // Act
            let outcome = work.as_mut().poll(&mut Context::from_waker(Waker::noop()));
            // Assert
            assert!(outcome.is_pending(), "pending prerequisite {pending}");
            assert!(!next_phase.get());
        }
    }

    #[test]
    fn each_registration_join_error_cancels_siblings_and_preserves_error() {
        for failing in 0..3 {
            // Arrange
            let polled = [Cell::new(false), Cell::new(false), Cell::new(false)];
            let dropped = [Cell::new(false), Cell::new(false), Cell::new(false)];
            let mut tree_result = (failing == 0).then_some(Err(CdpError::Closed));
            let mut runtime_result = (failing == 1).then_some(Err(CdpError::Closed));
            let mut registration_result = (failing == 2).then_some(Err(CdpError::Closed));
            let mut work = Box::pin(initialize_with_registration(
                Gate {
                    polled: &polled[0],
                    dropped: &dropped[0],
                    result: &mut tree_result,
                },
                Gate {
                    polled: &polled[1],
                    dropped: &dropped[1],
                    result: &mut runtime_result,
                },
                Gate {
                    polled: &polled[2],
                    dropped: &dropped[2],
                    result: &mut registration_result,
                },
            ));
            // Act
            let outcome = work.as_mut().poll(&mut Context::from_waker(Waker::noop()));
            // Assert
            assert!(matches!(outcome, Poll::Ready(Err(CdpError::Closed))));
            assert!(dropped.iter().all(Cell::get));
        }
    }

    #[tokio::test]
    async fn registration_join_returns_the_original_tree_only_after_success() {
        // Arrange
        let expected = tree();
        // Act
        let actual = initialize_with_registration(
            std::future::ready(Ok(expected.clone())),
            std::future::ready(Ok(())),
            std::future::ready(Ok(())),
        )
        .await
        .unwrap();
        // Assert
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            serde_json::to_value(expected).unwrap()
        );
    }
}
