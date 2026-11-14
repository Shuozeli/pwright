//! Thin Node conversion port for the Rust-owned Page routing installation.

#[cfg(feature = "large-route-post-data")]
use std::future::Future;
use std::sync::Arc;

use crate::{NativeSession, binding_error, boundary};
#[cfg(feature = "route-request-history")]
use napi::bindgen_prelude::External;
use napi::{
    Env, JsString, Unknown,
    bindgen_prelude::{Buffer, PromiseRaw},
};
#[cfg(any(feature = "route-header-url-overrides", feature = "page-unroute"))]
use napi::{
    JsValue,
    bindgen_prelude::{JsObjectValue, Object},
};
use napi_derive::napi;
#[cfg(feature = "large-route-post-data")]
use pwright_node_host::ffi_budget::FfiReservation;
use pwright_node_host::{
    HostError, RouteAbortReason, RouteAction, RouteCall, RouteContinue, ffi_budget::FfiLane,
};

#[cfg(not(feature = "large-route-post-data"))]
const ROUTE_WATCH_ALLOWANCE: u64 = 512 * 1024;
// An idle watcher owns no route body. Reserve the bounded publication payload
// only after the host has produced a RouteCall, before its N-API conversion.
#[cfg(feature = "large-route-post-data")]
const ROUTE_WATCH_ALLOWANCE: u64 = boundary::SMALL_ALLOWANCE;
#[cfg(feature = "large-route-post-data")]
const ROUTE_PUBLICATION_BASE_ALLOWANCE: u64 = 512 * 1024;
#[cfg(feature = "large-route-post-data")]
const MAX_ROUTE_PUBLICATION_POST_DATA_BYTES: usize = 3 * 1024 * 1024;
const ROUTE_FULFILL_ALLOWANCE: u64 =
    pwright_node_host::MAX_ROUTE_FULFILL_BODY_BYTES as u64 + 128 * 1024;
#[cfg(feature = "route-header-url-overrides")]
const ROUTE_OVERRIDE_ALLOWANCE: u64 = 96 * 1024;

#[cfg(feature = "large-route-post-data")]
fn route_publication_additional_bytes(post_data_bytes: Option<usize>) -> Result<u64, HostError> {
    let body_bytes = post_data_bytes.unwrap_or(0);
    if body_bytes > MAX_ROUTE_PUBLICATION_POST_DATA_BYTES {
        return Err(HostError::ResourceExhausted(format!(
            "Route publication POST data exceeds byte limit ({MAX_ROUTE_PUBLICATION_POST_DATA_BYTES}): {body_bytes}"
        )));
    }
    let body_allowance = u64::try_from(body_bytes)
        .map_err(|_| HostError::ResourceExhausted("Route publication byte budget overflow".into()))?
        .checked_mul(4)
        .ok_or_else(|| {
            HostError::ResourceExhausted("Route publication byte budget overflow".into())
        })?;
    Ok(body_allowance
        .max(ROUTE_PUBLICATION_BASE_ALLOWANCE)
        .saturating_sub(ROUTE_WATCH_ALLOWANCE))
}

#[cfg(feature = "large-route-post-data")]
async fn reserve_route_publication_or_abort<T, Abort, AbortFuture>(
    reservation: &FfiReservation<T>,
    call_id: u64,
    post_data_bytes: Option<usize>,
    abort: Abort,
) -> Result<(), HostError>
where
    T: Send,
    Abort: FnOnce(u64) -> AbortFuture,
    AbortFuture: Future<Output = Result<(), HostError>>,
{
    let reserved = route_publication_additional_bytes(post_data_bytes)
        .and_then(|bytes| reservation.reserve_additional(bytes));
    if let Err(error) = reserved {
        // The native queue has already yielded this call. Do not leave its
        // browser request paused when the foreign-runtime publication fails.
        let _ = abort(call_id).await;
        return Err(error);
    }
    Ok(())
}

#[napi(object)]
pub struct NativeRouteHeader {
    pub name: String,
    pub value: String,
}

#[cfg(not(feature = "route-request-history"))]
#[napi(object)]
pub struct NativeRouteCall {
    pub call_id: String,
    pub registration_id: String,
    pub context_id: String,
    pub page_id: String,
    pub frame_id: Option<String>,
    pub url: String,
    pub method: String,
    pub resource_type: String,
    pub headers: Vec<NativeRouteHeader>,
    pub post_data: Option<napi::bindgen_prelude::Buffer>,
    pub is_navigation_request: bool,
    pub expires: bool,
}

// napi-derive expands object fields before rustc applies field-level cfg.
// Keep the opted-in shape separate, including its charged owner identity.
#[cfg(feature = "route-request-history")]
#[napi(object, object_from_js = false)]
pub struct NativeRouteCall {
    pub call_id: String,
    pub registration_id: String,
    pub context_id: String,
    pub page_id: String,
    pub frame_id: Option<String>,
    pub url: String,
    pub method: String,
    pub resource_type: String,
    pub headers: Vec<NativeRouteHeader>,
    pub post_data: Option<napi::bindgen_prelude::Buffer>,
    pub is_navigation_request: bool,
    pub expires: bool,
    pub native_request_id: Option<String>,
    /// Network-event identity for this exact interception hop. Browser/CDP
    /// request IDs may be reused across redirects, so they are not suitable
    /// as public Request wrapper keys.
    pub request_id: Option<String>,
    pub redirected_from: Option<String>,
    pub history_owner:
        Option<External<Arc<pwright_node_host::network_events::RequestHistoryOwner>>>,
}

impl From<RouteCall> for NativeRouteCall {
    fn from(value: RouteCall) -> Self {
        Self {
            call_id: value.id.to_string(),
            registration_id: value.registration_id.to_string(),
            context_id: value.context.get().to_string(),
            page_id: value.page.get().to_string(),
            frame_id: value.frame.map(|frame| frame.get().to_string()),
            url: value.url,
            method: value.method,
            resource_type: value.resource_type,
            headers: value
                .headers
                .into_iter()
                .map(|(name, value)| NativeRouteHeader { name, value })
                .collect(),
            post_data: value.post_data.map(Into::into),
            is_navigation_request: value.is_navigation_request,
            expires: value.expires,
            #[cfg(feature = "route-request-history")]
            native_request_id: value.native_request_id,
            #[cfg(feature = "route-request-history")]
            request_id: None,
            #[cfg(feature = "route-request-history")]
            redirected_from: None,
            #[cfg(feature = "route-request-history")]
            history_owner: None,
        }
    }
}

#[cfg(feature = "route-request-history")]
impl
    From<(
        RouteCall,
        Option<Arc<pwright_node_host::network_events::RequestHistoryOwner>>,
    )> for NativeRouteCall
{
    fn from(
        (route, history_owner): (
            RouteCall,
            Option<Arc<pwright_node_host::network_events::RequestHistoryOwner>>,
        ),
    ) -> Self {
        let mut call = Self::from(route);
        if let Some(owner) = &history_owner {
            call.request_id = Some(owner.request().id.clone());
            call.redirected_from = owner.request().redirected_from.clone();
        }
        call.history_owner = history_owner.map(External::new);
        call
    }
}

/// Build identity only; no additional request or response cache is installed.
#[napi(js_name = "routeRequestHistoryEnabled")]
pub fn route_request_history_enabled() -> bool {
    cfg!(feature = "route-request-history")
}

#[cfg(feature = "route-request-history")]
fn native_raw_headers(headers: Vec<(String, String)>) -> Vec<NativeRouteHeader> {
    headers
        .into_iter()
        .map(|(name, value)| NativeRouteHeader { name, value })
        .collect()
}

#[cfg(feature = "route-request-history")]
fn raw_header_copy_bytes(headers: &[NativeRouteHeader]) -> u64 {
    let bytes = headers.iter().fold(0_u64, |total, header| {
        total
            .saturating_add(u64::try_from(header.name.len()).unwrap_or(u64::MAX))
            .saturating_add(u64::try_from(header.value.len()).unwrap_or(u64::MAX))
    });
    bytes
        .saturating_mul(4)
        .saturating_add(
            u64::try_from(headers.len())
                .unwrap_or(u64::MAX)
                .saturating_mul(256),
        )
        .saturating_add(4 * 1024)
}

fn parse_call_id(value: JsString<'_>) -> napi::Result<u64> {
    boundary::bounded_string(value, 20, "route call ID")?
        .parse::<u64>()
        .ok()
        .filter(|value| *value != 0)
        .ok_or_else(|| binding_error(HostError::InvalidArgument("invalid route call ID".into())))
}

#[cfg(feature = "page-unroute")]
fn bounded_registration_ids(value: Unknown<'_>) -> napi::Result<Vec<u64>> {
    if !value.is_array()? {
        return Err(binding_error(HostError::InvalidArgument(
            "route registration IDs must be an array".into(),
        )));
    }
    // SAFETY: is_array checked above; the callback scope retains this handle.
    let entries = unsafe { value.cast::<Object<'_>>()? };
    let count = entries.get_array_length()?;
    if count == 0 || count > 256 {
        return Err(binding_error(HostError::InvalidArgument(
            "route registration ID count must be in [1, 256]".into(),
        )));
    }
    let mut ids = Vec::with_capacity(count as usize);
    for index in 0..count {
        let raw: JsString<'_> = entries.get_element(index)?;
        let id = boundary::bounded_string(raw, 20, "route registration ID")?
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .ok_or_else(|| {
                binding_error(HostError::InvalidArgument(
                    "invalid route registration ID".into(),
                ))
            })?;
        if ids.contains(&id) {
            return Err(binding_error(HostError::InvalidArgument(
                "duplicate route registration ID".into(),
            )));
        }
        ids.push(id);
    }
    Ok(ids)
}

#[cfg(feature = "route-header-url-overrides")]
fn bounded_route_headers(value: Unknown<'_>) -> napi::Result<Vec<(String, String)>> {
    if !value.is_array()? {
        return Err(binding_error(HostError::InvalidArgument(
            "route continuation headers must be an array".into(),
        )));
    }
    // SAFETY: is_array checked above; the callback scope retains this handle.
    let entries = unsafe { value.cast::<Object<'_>>()? };
    let count = entries.get_array_length()?;
    if count > 256 {
        return Err(binding_error(HostError::InvalidArgument(
            "route continuation headers exceed count limit (256)".into(),
        )));
    }
    let mut headers = Vec::with_capacity(count as usize);
    let mut bytes = 0usize;
    for index in 0..count {
        let header: Object<'_> = entries.get_element(index)?;
        let name: JsString<'_> = header.get_named_property("name")?;
        let value: JsString<'_> = header.get_named_property("value")?;
        let name_bytes = name.utf8_len()?;
        let value_bytes = value.utf8_len()?;
        bytes = bytes
            .checked_add(name_bytes)
            .and_then(|total| total.checked_add(value_bytes))
            .ok_or_else(|| {
                binding_error(HostError::InvalidArgument(
                    "route continuation header size overflow".into(),
                ))
            })?;
        if bytes > 64 * 1024 {
            return Err(binding_error(HostError::InvalidArgument(
                "route continuation headers exceed byte limit (65536)".into(),
            )));
        }
        headers.push((
            name.into_utf8()?.into_owned()?,
            value.into_utf8()?.into_owned()?,
        ));
    }
    Ok(headers)
}

#[napi]
impl NativeSession {
    #[napi]
    pub fn page_route<'env>(
        &self,
        env: &'env Env,
        page_id: JsString<'env>,
        pattern: JsString<'env>,
        times: Option<u32>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let page_id = boundary::bounded_string(page_id, 20, "page ID")?;
        let pattern = boundary::bounded_string(pattern, 4 * 1024, "route pattern")?;
        let host = Arc::clone(&self.host);
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.page_route(&page_id, pattern, times.map(u64::from))
                .await
                .map(|registration| registration.to_string())
        })
    }
}

#[cfg(all(test, feature = "large-route-post-data"))]
mod large_route_publication_tests {
    use std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    };

    use pwright_node_host::{
        HostError,
        ffi_budget::{FfiBudget, FfiLane, FfiLimits},
    };

    use super::{
        MAX_ROUTE_PUBLICATION_POST_DATA_BYTES, ROUTE_WATCH_ALLOWANCE,
        reserve_route_publication_or_abort,
    };

    fn watch_budget(reserved_bytes: u64) -> FfiBudget {
        FfiBudget::new(FfiLimits {
            connects: 0,
            commands: 0,
            closes: 0,
            watches: 16,
            reserved_bytes,
            future_bytes: [0; 4],
        })
        .unwrap()
    }

    #[test]
    fn six_idle_route_watchers_use_small_admissions() {
        // Arrange
        let budget = watch_budget(64 * 1024 * 1024);

        // Act
        let watchers = (0..6)
            .map(|_| {
                budget
                    .admit::<()>(FfiLane::Watch, ROUTE_WATCH_ALLOWANCE)
                    .unwrap()
            })
            .collect::<Vec<_>>();

        // Assert
        assert_eq!(budget.metrics().unwrap().watches, 6);
        assert_eq!(
            budget.metrics().unwrap().reserved_bytes,
            6 * ROUTE_WATCH_ALLOWANCE
        );
        for watcher in watchers {
            watcher
                .run_sync(|| Ok(()))
                .deliver(|result| result.unwrap())
                .unwrap();
        }
        assert_eq!(budget.metrics().unwrap().reserved_bytes, 0);
    }

    #[tokio::test]
    async fn two_megabyte_route_charges_actual_publication_until_delivery() {
        // Arrange
        let budget = watch_budget(64 * 1024 * 1024);
        let execution = budget
            .admit::<()>(FfiLane::Watch, ROUTE_WATCH_ALLOWANCE)
            .unwrap();
        let reservation = execution.reservation();
        let aborts = Arc::new(AtomicU64::new(0));
        let aborted = Arc::clone(&aborts);

        // Act
        let reserved = reserve_route_publication_or_abort(
            &reservation,
            41,
            Some(2 * 1024 * 1024),
            move |_| async move {
                aborted.fetch_add(1, Ordering::SeqCst);
                Ok(())
            },
        )
        .await;
        let ticket = execution.run(async { Ok(()) }).await;

        // Assert
        assert!(reserved.is_ok());
        assert_eq!(aborts.load(Ordering::SeqCst), 0);
        ticket
            .deliver(|result| {
                assert!(result.is_ok());
                assert_eq!(budget.metrics().unwrap().reserved_bytes, 8 * 1024 * 1024);
            })
            .unwrap();
        assert_eq!(budget.metrics().unwrap().reserved_bytes, 0);
    }

    #[tokio::test]
    async fn over_cap_route_is_aborted_without_extra_publication_reservation() {
        // Arrange
        let budget = watch_budget(64 * 1024 * 1024);
        let execution = budget
            .admit::<()>(FfiLane::Watch, ROUTE_WATCH_ALLOWANCE)
            .unwrap();
        let reservation = execution.reservation();
        let aborted_call = Arc::new(AtomicU64::new(0));
        let aborted = Arc::clone(&aborted_call);

        // Act
        let result = reserve_route_publication_or_abort(
            &reservation,
            73,
            Some(MAX_ROUTE_PUBLICATION_POST_DATA_BYTES + 1),
            move |id| async move {
                aborted.store(id, Ordering::SeqCst);
                Ok(())
            },
        )
        .await;

        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        assert_eq!(aborted_call.load(Ordering::SeqCst), 73);
        assert_eq!(
            budget.metrics().unwrap().reserved_bytes,
            ROUTE_WATCH_ALLOWANCE
        );
        execution
            .run_sync(|| Ok(()))
            .deliver(|result| result.unwrap())
            .unwrap();
        assert_eq!(budget.metrics().unwrap().reserved_bytes, 0);
    }

    #[tokio::test]
    async fn exhausted_publication_budget_aborts_the_paused_route() {
        // Arrange
        let budget = watch_budget(7 * 1024 * 1024);
        let execution = budget
            .admit::<()>(FfiLane::Watch, ROUTE_WATCH_ALLOWANCE)
            .unwrap();
        let reservation = execution.reservation();
        let aborted_call = Arc::new(AtomicU64::new(0));
        let aborted = Arc::clone(&aborted_call);

        // Act
        let result = reserve_route_publication_or_abort(
            &reservation,
            89,
            Some(2 * 1024 * 1024),
            move |id| async move {
                aborted.store(id, Ordering::SeqCst);
                Ok(())
            },
        )
        .await;

        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        assert_eq!(aborted_call.load(Ordering::SeqCst), 89);
        assert_eq!(
            budget.metrics().unwrap().reserved_bytes,
            ROUTE_WATCH_ALLOWANCE
        );
        execution
            .run_sync(|| Ok(()))
            .deliver(|result| result.unwrap())
            .unwrap();
        assert_eq!(budget.metrics().unwrap().reserved_bytes, 0);
    }
}

#[cfg(feature = "route-request-history")]
#[napi]
impl NativeSession {
    /// Return the preserved response header sequence for this exact route hop.
    /// The source remains charged to its RequestHistoryOwner until released.
    #[napi]
    pub fn response_raw_headers<'env>(
        &self,
        env: &'env Env,
        owner: &External<Arc<pwright_node_host::network_events::RequestHistoryOwner>>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Watch, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let owner = Arc::clone(owner.as_ref());
        boundary::spawn_published_sized(
            env,
            execution,
            self.publications.clone(),
            async move {
                let response = owner.response().await?.ok_or_else(|| {
                    HostError::Backend("routed Request has no Response headers".into())
                })?;
                Ok(native_raw_headers(response.headers))
            },
            |headers: &Vec<NativeRouteHeader>| raw_header_copy_bytes(headers),
        )
    }
}

#[cfg(all(test, feature = "route-request-history"))]
mod route_request_history_tests {
    use super::{native_raw_headers, raw_header_copy_bytes, route_request_history_enabled};

    #[test]
    fn route_request_history_marker_is_enabled() {
        // Arrange / Act
        let enabled = route_request_history_enabled();

        // Assert
        assert!(enabled);
    }

    #[test]
    fn raw_response_headers_keep_order_case_and_duplicate_values() {
        // Arrange
        let source = vec![
            ("Content-Type".to_owned(), "text/html".to_owned()),
            ("Set-Cookie".to_owned(), "a=1".to_owned()),
            ("set-cookie".to_owned(), "b=2".to_owned()),
        ];

        // Act
        let headers = native_raw_headers(source);

        // Assert
        assert_eq!(headers.len(), 3);
        assert_eq!(headers[0].name, "Content-Type");
        assert_eq!(headers[0].value, "text/html");
        assert_eq!(headers[1].name, "Set-Cookie");
        assert_eq!(headers[1].value, "a=1");
        assert_eq!(headers[2].name, "set-cookie");
        assert_eq!(headers[2].value, "b=2");
        assert!(raw_header_copy_bytes(&headers) >= 4 * 1024);
    }
}

#[cfg(all(test, not(feature = "route-request-history")))]
mod route_request_history_feature_off_tests {
    use super::route_request_history_enabled;

    #[test]
    fn route_request_history_marker_is_disabled() {
        // Arrange / Act
        let enabled = route_request_history_enabled();

        // Assert
        assert!(!enabled);
    }
}

#[cfg(feature = "route-regexp")]
#[napi]
impl NativeSession {
    #[napi(js_name = "pageRouteRegExp")]
    pub fn page_route_reg_exp<'env>(
        &self,
        env: &'env Env,
        page_id: JsString<'env>,
        source: JsString<'env>,
        flags: JsString<'env>,
        times: Option<u32>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let page_id = boundary::bounded_string(page_id, 20, "page ID")?;
        let source = boundary::bounded_string(source, 4 * 1024, "route RegExp source")?;
        let flags = boundary::bounded_string(flags, 4, "route RegExp flags")?;
        let host = Arc::clone(&self.host);
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.page_route_regexp(&page_id, source, flags, times.map(u64::from))
                .await
                .map(|registration| registration.to_string())
        })
    }
}

#[cfg(feature = "page-unroute")]
#[napi]
impl NativeSession {
    #[napi]
    pub fn page_unroute<'env>(
        &self,
        env: &'env Env,
        page_id: JsString<'env>,
        registration_ids: Unknown<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let page_id = boundary::bounded_string(page_id, 20, "page ID")?;
        let registration_ids = bounded_registration_ids(registration_ids)?;
        let host = Arc::clone(&self.host);
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.page_unroute(&page_id, &registration_ids).await
        })
    }
}

#[napi]
impl NativeSession {
    #[napi]
    pub fn next_page_route_call<'env>(
        &self,
        env: &'env Env,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Watch, ROUTE_WATCH_ALLOWANCE)
            .map_err(binding_error)?;
        #[cfg(feature = "large-route-post-data")]
        let reservation = execution.reservation();
        let host = Arc::clone(&self.host);
        #[cfg(feature = "route-request-history")]
        {
            boundary::spawn_with_env(
                env,
                execution,
                async move {
                    let call = host.next_page_route_call_with_history().await?;
                    #[cfg(feature = "large-route-post-data")]
                    if let Some((route, _)) = &call {
                        let abort_host = Arc::clone(&host);
                        reserve_route_publication_or_abort(
                            &reservation,
                            route.id,
                            route.post_data.as_ref().map(Vec::len),
                            move |call_id| async move {
                                abort_host
                                    .page_route_action(
                                        call_id,
                                        RouteAction::Abort(RouteAbortReason::Failed),
                                    )
                                    .await
                            },
                        )
                        .await?;
                    }
                    Ok(call)
                },
                |_env, call| Ok(call.map(NativeRouteCall::from)),
            )
        }
        #[cfg(not(feature = "route-request-history"))]
        boundary::spawn_with_env(
            env,
            execution,
            async move {
                let call = host.next_page_route_call().await;
                #[cfg(feature = "large-route-post-data")]
                if let Some(route) = &call {
                    let abort_host = Arc::clone(&host);
                    reserve_route_publication_or_abort(
                        &reservation,
                        route.id,
                        route.post_data.as_ref().map(Vec::len),
                        move |call_id| async move {
                            abort_host
                                .page_route_action(
                                    call_id,
                                    RouteAction::Abort(RouteAbortReason::Failed),
                                )
                                .await
                        },
                    )
                    .await?;
                }
                Ok(call)
            },
            |_env, call| Ok(call.map(NativeRouteCall::from)),
        )
    }

    #[napi]
    pub fn page_route_continue<'env>(
        &self,
        env: &'env Env,
        call_id: JsString<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let call_id = parse_call_id(call_id)?;
        let host = Arc::clone(&self.host);
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.page_route_action(call_id, RouteAction::Continue(RouteContinue::default()))
                .await
        })
    }
}

#[cfg(feature = "route-header-url-overrides")]
#[napi]
impl NativeSession {
    #[napi]
    pub fn page_route_continue_with_headers<'env>(
        &self,
        env: &'env Env,
        call_id: JsString<'env>,
        headers: Unknown<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, ROUTE_OVERRIDE_ALLOWANCE)
            .map_err(binding_error)?;
        let call_id = parse_call_id(call_id)?;
        let headers = bounded_route_headers(headers)?;
        let options = RouteContinue::new(None, None, Some(headers), None).map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.page_route_action(call_id, RouteAction::Continue(options))
                .await
        })
    }
}

#[napi]
impl NativeSession {
    #[napi]
    pub fn page_route_fallback<'env>(
        &self,
        env: &'env Env,
        call_id: JsString<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let call_id = parse_call_id(call_id)?;
        let host = Arc::clone(&self.host);
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.page_route_fallback(call_id, RouteContinue::default())
                .await
        })
    }
}

#[cfg(feature = "route-header-url-overrides")]
#[napi]
impl NativeSession {
    #[napi]
    pub fn page_route_fallback_with_url<'env>(
        &self,
        env: &'env Env,
        call_id: JsString<'env>,
        url: JsString<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, ROUTE_OVERRIDE_ALLOWANCE)
            .map_err(binding_error)?;
        let call_id = parse_call_id(call_id)?;
        let url = boundary::bounded_string(url, 64 * 1024, "route fallback URL")?;
        let options = RouteContinue::new(Some(url), None, None, None).map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.page_route_fallback(call_id, options).await
        })
    }
}

#[napi]
impl NativeSession {
    #[napi]
    pub fn page_route_abort<'env>(
        &self,
        env: &'env Env,
        call_id: JsString<'env>,
        error_code: Option<JsString<'env>>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let call_id = parse_call_id(call_id)?;
        let error_code = error_code
            .map(|value| boundary::bounded_string(value, 64, "route abort reason"))
            .transpose()?;
        let reason = RouteAbortReason::parse(error_code.as_deref()).map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.page_route_action(call_id, RouteAction::Abort(reason))
                .await
        })
    }

    #[napi]
    pub fn page_route_fulfill<'env>(
        &self,
        env: &'env Env,
        call_id: JsString<'env>,
        status: i32,
        headers: Vec<NativeRouteHeader>,
        body: Buffer,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, ROUTE_FULFILL_ALLOWANCE)
            .map_err(binding_error)?;
        let call_id = parse_call_id(call_id)?;
        let headers = headers
            .into_iter()
            .map(|header| (header.name, header.value))
            .collect();
        let fulfill =
            pwright_node_host::RouteFulfill::new(i64::from(status), headers, body.to_vec())
                .map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.page_route_action(call_id, RouteAction::Fulfill(fulfill))
                .await
        })
    }
}
