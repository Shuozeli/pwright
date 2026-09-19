//! Host-owned synchronous metadata preparation. The outer lock excludes all
//! host readers while admission can still fail; no native await holds it.
use super::{MAX_FAILED_PAGES, Route};
use crate::HostError;
use pwright_core::ObjectId;
use pwright_network_model::extra_headers::{
    EffectiveHeaders,
    settings::{HeaderSettings, HeaderUpdate},
};
use std::sync::{Arc, Mutex};

pub(super) struct DispatchPlan {
    pub(super) routes: Vec<Route>,
    pub(super) pages: Vec<(ObjectId, Option<EffectiveHeaders>)>,
}
impl DispatchPlan {
    /// Vec/route storage is covered by the already admitted host request body;
    /// effective values independently consume the HeaderPool's owner leases.
    pub(super) fn capture(
        settings: &HeaderSettings,
        routes: Vec<Route>,
    ) -> Result<Self, HostError> {
        let mut pages = Vec::with_capacity(MAX_FAILED_PAGES);
        for route in &routes {
            if pages.iter().any(|(page, _)| *page == route.page) {
                continue;
            }
            if pages.len() == MAX_FAILED_PAGES {
                return Err(HostError::ResourceExhausted(
                    "extra header dispatch Page capacity exhausted".into(),
                ));
            }
            // Context dispatch reads its first Page synchronously, then awaits
            // that Page before reading the next. Freezing later Pages here
            // would replay stale desired values after an overlapping setter.
            let headers = if pages.is_empty() {
                Some(
                    settings
                        .effective(route.context, route.page)
                        .map_err(super::settings_error)?,
                )
            } else {
                None
            };
            pages.push((route.page, headers));
        }
        Ok(Self { routes, pages })
    }
}

/// Cancellation after publication may have been observed by an initializer.
/// Drop therefore restores desired state but records uncertainty, never claims
/// that discarding an unexecuted queue entry proves absence of native effects.
pub(super) struct PreparedUpdate {
    settings: Arc<Mutex<HeaderSettings>>,
    update: Option<HeaderUpdate>,
}
impl PreparedUpdate {
    pub(super) fn new(settings: Arc<Mutex<HeaderSettings>>, update: HeaderUpdate) -> Self {
        Self {
            settings,
            update: Some(update),
        }
    }
    pub(super) fn restore_plan(&self, primary: &DispatchPlan) -> Result<DispatchPlan, HostError> {
        let settings = self.settings.lock().map_err(HostError::backend)?;
        self.update
            .as_ref()
            .ok_or(HostError::TargetClosed)?
            .restore_prior()
            .map_err(HostError::backend)?;
        DispatchPlan::capture(&settings, primary.routes.clone())
    }
    pub(super) fn commit(&mut self) -> Result<(), HostError> {
        let _settings = self.settings.lock().map_err(HostError::backend)?;
        self.update
            .take()
            .ok_or(HostError::TargetClosed)?
            .commit()
            .map_err(HostError::backend)
    }
    pub(super) fn rollback(&mut self) -> Result<(), HostError> {
        let _settings = self.settings.lock().map_err(HostError::backend)?;
        self.update
            .take()
            .ok_or(HostError::TargetClosed)?
            .rollback()
            .map_err(HostError::backend)
    }
}
impl Drop for PreparedUpdate {
    fn drop(&mut self) {
        // A poisoned outer lock already rejects every host metadata read. Drop
        // the inner update in either case to release its pending-writer owner.
        let _settings = self.settings.lock();
        drop(self.update.take());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extra_headers::{
        HeaderPool, HeaderRef, settings_profile, tests::ledger, value_profile,
    };
    use pwright_core::ResourceClass;

    fn route(page: u64, session: &str) -> Route {
        Route {
            context: ObjectId::new(1),
            page: ObjectId::new(page),
            watch: super::super::session_retirement::Watch::new(session, "main", session),
        }
    }

    #[test]
    fn dispatch_freezes_first_page_and_defers_later_pages_across_writes() {
        // Arrange
        let resources = ledger();
        let pool = HeaderPool::new(value_profile().unwrap(), &resources).unwrap();
        let settings = HeaderSettings::new(settings_profile().unwrap(), pool, &resources).unwrap();
        let first = settings
            .begin_context(
                ObjectId::new(1),
                &[HeaderRef {
                    name: "X-Layer",
                    value: "first",
                }],
            )
            .unwrap();
        let plan = DispatchPlan::capture(
            &settings,
            vec![route(2, "main"), route(2, "oopif"), route(3, "second")],
        )
        .unwrap();
        // Act
        settings
            .begin_context(
                ObjectId::new(1),
                &[HeaderRef {
                    name: "X-Layer",
                    value: "second",
                }],
            )
            .unwrap()
            .commit()
            .unwrap();
        first.commit().unwrap();
        // Assert
        assert_eq!(plan.pages.len(), 2);
        assert_eq!(
            plan.pages[0]
                .1
                .as_ref()
                .unwrap()
                .iter()
                .next()
                .unwrap()
                .value,
            "first"
        );
        assert!(plan.pages[1].1.is_none());
        assert_eq!(
            settings
                .effective(ObjectId::new(1), ObjectId::new(2))
                .unwrap()
                .iter()
                .next()
                .unwrap()
                .value,
            "second"
        );
        drop(plan);
        drop(settings);
        for class in ResourceClass::ALL {
            assert_eq!(resources.metrics().unwrap().class(class).current, 0);
        }
    }

    #[test]
    fn frozen_compensation_and_late_abandonment_do_not_overwrite_a_later_setter() {
        // Arrange
        let resources = ledger();
        let pool = HeaderPool::new(value_profile().unwrap(), &resources).unwrap();
        let settings = Arc::new(Mutex::new(
            HeaderSettings::new(settings_profile().unwrap(), pool, &resources).unwrap(),
        ));
        let values = |value| {
            [HeaderRef {
                name: "X-Layer",
                value,
            }]
        };
        let context = ObjectId::new(1);
        settings
            .lock()
            .unwrap()
            .begin_context(context, &values("old"))
            .unwrap()
            .commit()
            .unwrap();
        let update = settings
            .lock()
            .unwrap()
            .begin_context(context, &values("bad"))
            .unwrap();
        let guard = PreparedUpdate::new(settings.clone(), update);
        let primary =
            DispatchPlan::capture(&settings.lock().unwrap(), vec![route(2, "main")]).unwrap();
        let compensation = guard.restore_plan(&primary).unwrap();
        // Act
        settings
            .lock()
            .unwrap()
            .begin_context(context, &values("new"))
            .unwrap()
            .commit()
            .unwrap();
        drop(guard);
        // Assert
        assert_eq!(
            primary.pages[0]
                .1
                .as_ref()
                .unwrap()
                .iter()
                .next()
                .unwrap()
                .value,
            "bad"
        );
        assert_eq!(
            compensation.pages[0]
                .1
                .as_ref()
                .unwrap()
                .iter()
                .next()
                .unwrap()
                .value,
            "old"
        );
        assert_eq!(
            settings
                .lock()
                .unwrap()
                .effective(context, ObjectId::new(2))
                .unwrap()
                .iter()
                .next()
                .unwrap()
                .value,
            "new"
        );
        drop(primary);
        drop(compensation);
        drop(settings);
        for class in ResourceClass::ALL {
            assert_eq!(resources.metrics().unwrap().class(class).current, 0);
        }
    }

    #[test]
    fn oversized_page_plan_releases_all_partial_effective_owners() {
        // Arrange
        let resources = ledger();
        let pool = HeaderPool::new(value_profile().unwrap(), &resources).unwrap();
        let settings = HeaderSettings::new(settings_profile().unwrap(), pool, &resources).unwrap();
        let before = resources
            .metrics()
            .unwrap()
            .class(ResourceClass::LiveObject)
            .current;
        let routes = (0..=MAX_FAILED_PAGES)
            .map(|page| route(page as u64 + 2, "native"))
            .collect();
        // Act
        let result = DispatchPlan::capture(&settings, routes);
        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        assert_eq!(
            resources
                .metrics()
                .unwrap()
                .class(ResourceClass::LiveObject)
                .current,
            before
        );
    }
}
