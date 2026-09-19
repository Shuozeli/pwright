//! Borrowed Page-only policy, followed by a bounded copy for native navigation.
use super::{Installation, admit};
use crate::{HostError, HostSession};
use pwright_core::ObjectId;
use pwright_network_model::extra_headers::PageHeaders;
use std::sync::{Arc, atomic::Ordering};

pub(super) const MAX_OPERATIONS: usize = 8;
pub(super) const OPERATION_BYTES: usize = 32 * 1024;
const MAX_REFERRER_BYTES: usize = 16 * 1024;

struct Permit(Arc<Installation>);
impl Drop for Permit {
    fn drop(&mut self) {
        self.0.navigations.fetch_sub(1, Ordering::AcqRel);
    }
}

/// The permit stays alive after `value` moves into the command, through await.
pub(crate) struct NavigationReferrer {
    pub(crate) value: Option<String>,
    _permit: Permit,
}

fn resolve(
    headers: Option<&PageHeaders>,
    option: Option<String>,
) -> Result<Option<String>, HostError> {
    if option
        .as_ref()
        .is_some_and(|value| value.capacity() > MAX_REFERRER_BYTES)
    {
        return Err(HostError::ResourceExhausted(
            "navigation referer storage exceeds 16 KiB".into(),
        ));
    }
    let Some(headers) = headers else {
        return Ok(option);
    };
    let selected = headers
        .navigation_referrer(option.as_deref())
        .map_err(HostError::backend)?;
    if let Some(value) = selected
        && value.len() > MAX_REFERRER_BYTES
    {
        return Err(HostError::ResourceExhausted(
            "navigation referer storage exceeds 16 KiB".into(),
        ));
    }
    // A supplied string is already owned. Only the implicit Page header needs
    // copying; its pool snapshot is borrowed and does not survive preparation.
    if option.is_some() {
        Ok(option)
    } else {
        Ok(selected.map(str::to_owned))
    }
}

impl HostSession {
    pub(crate) fn navigation_referrer(
        &self,
        page_id: &str,
        option: Option<String>,
    ) -> Result<NavigationReferrer, HostError> {
        let id = page_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Page ID".into()))?;
        let state = self.state.lock().map_err(HostError::backend)?;
        let live = state.as_ref().ok_or(HostError::TargetClosed)?;
        let topology = live.composition.topology().map_err(HostError::backend)?;
        let context = topology
            .topology()
            .page_session_routes()
            .find(|page| page.object_id() == id)
            .map(|page| page.browser_context_object_id())
            .ok_or(HostError::TargetClosed)?;
        let installation = &live.extra_headers;
        admit(&installation.navigations, MAX_OPERATIONS)?;
        let permit = Permit(installation.clone());
        let headers = installation
            .settings
            .lock()
            .map_err(HostError::backend)?
            .page_headers(context, id)
            .map_err(HostError::backend)?;
        let value = resolve(headers.as_ref(), option)?;
        Ok(NavigationReferrer {
            value,
            _permit: permit,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extra_headers::{HeaderPool, HeaderRef, tests::ledger, value_profile};

    #[test]
    fn implicit_referrer_copies_first_original_page_header() {
        // Arrange
        let resources = ledger();
        let pool = HeaderPool::new(value_profile().unwrap(), &resources).unwrap();
        let headers = pool
            .page(&[
                HeaderRef {
                    name: "Referer",
                    value: "first",
                },
                HeaderRef {
                    name: "referer",
                    value: "last",
                },
            ])
            .unwrap();
        // Act
        let result = resolve(Some(&headers), None).unwrap();
        // Assert
        assert_eq!(result.as_deref(), Some("first"));
    }

    #[test]
    fn equal_option_is_moved_without_a_second_string_allocation() {
        // Arrange
        let resources = ledger();
        let pool = HeaderPool::new(value_profile().unwrap(), &resources).unwrap();
        let headers = pool
            .page(&[HeaderRef {
                name: "Referer",
                value: "same",
            }])
            .unwrap();
        let option = String::from("same");
        let pointer = option.as_ptr();
        // Act
        let result = resolve(Some(&headers), Some(option)).unwrap().unwrap();
        // Assert
        assert_eq!(result.as_ptr(), pointer);
    }

    #[test]
    fn implicit_referrer_obeys_navigation_copy_bound_before_allocating() {
        // Arrange
        let resources = ledger();
        let pool = HeaderPool::new(value_profile().unwrap(), &resources).unwrap();
        let value = "x".repeat(MAX_REFERRER_BYTES + 1);
        let headers = pool
            .page(&[HeaderRef {
                name: "Referer",
                value: &value,
            }])
            .unwrap();
        // Act
        let result = resolve(Some(&headers), None);
        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        assert!(
            MAX_REFERRER_BYTES + std::mem::size_of::<NavigationReferrer>() + 4096
                <= OPERATION_BYTES
        );
    }
}
