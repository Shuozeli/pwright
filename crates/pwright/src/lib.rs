#![doc = include_str!("../README.md")]
#![cfg_attr(
    not(feature = "navigation"),
    doc = r#"
```compile_fail
use pwright::navigation::Goto;
```
"#
)]
#![cfg_attr(
    not(feature = "evaluation"),
    doc = r#"
```compile_fail
use pwright::evaluation::EvaluateValue;
```
"#
)]
#![cfg_attr(
    not(feature = "dom-content"),
    doc = r#"
```compile_fail
use pwright::dom_content::FrameContent;
```
"#
)]
#![cfg_attr(
    not(feature = "content"),
    doc = r#"
```compile_fail
use pwright::content::PageScreenshot;
```
"#
)]
#![cfg_attr(
    not(feature = "context-lifecycle"),
    doc = r#"
```compile_fail
use pwright::context_lifecycle::BrowserContextClose;
```
"#
)]
#![cfg_attr(
    not(feature = "page-lifecycle"),
    doc = r#"
```compile_fail
use pwright::page_lifecycle::PageClose;
```
"#
)]
#![cfg_attr(
    not(feature = "page-creation"),
    doc = r#"
```compile_fail
use pwright::page_creation::BrowserContextNewPage;
```
"#
)]
#![cfg_attr(
    not(feature = "context-creation"),
    doc = r#"
```compile_fail
use pwright::context_creation::BrowserNewContext;
```
"#
)]
#![cfg_attr(
    not(feature = "handle-lifecycle"),
    doc = r#"
```compile_fail
use pwright::handle_lifecycle::HandleDispose;
```
"#
)]
#![cfg_attr(
    not(feature = "handle-evaluation"),
    doc = r#"
```compile_fail
use pwright::handle_evaluation::EvaluateHandle;
```
"#
)]
#![cfg_attr(
    not(feature = "locator"),
    doc = r#"
```compile_fail
use pwright::locator::Count;
```
"#
)]
#![cfg_attr(
    not(feature = "locator-screenshot"),
    doc = r#"
```compile_fail
use pwright::locator_screenshot::LocatorScreenshot;
```
"#
)]
#![cfg_attr(
    not(feature = "page-events"),
    doc = r#"
```compile_fail
use pwright::page_events::PageLoad;
```
"#
)]
#![cfg_attr(
    not(feature = "network-events"),
    doc = r#"
```compile_fail
use pwright::network_events::NetworkRequest;
```
"#
)]

//! This crate contains no implementation of its own. Every public module is
//! an explicit re-export selected by one Cargo feature.

#[cfg(feature = "core")]
pub use pwright_core as core;

#[cfg(feature = "navigation")]
pub use pwright_cap_navigation as navigation;

#[cfg(feature = "evaluation")]
pub use pwright_cap_evaluation as evaluation;

#[cfg(feature = "content")]
pub use pwright_cap_content as content;

#[cfg(feature = "context-lifecycle")]
pub use pwright_cap_context_lifecycle as context_lifecycle;

#[cfg(feature = "page-lifecycle")]
pub use pwright_cap_page_lifecycle as page_lifecycle;

#[cfg(feature = "page-creation")]
pub use pwright_cap_page_creation as page_creation;

#[cfg(feature = "handle-lifecycle")]
pub use pwright_cap_handle_lifecycle as handle_lifecycle;

#[cfg(feature = "handle-evaluation")]
pub use pwright_cap_handle_evaluation as handle_evaluation;

#[cfg(feature = "ordered-handle-delivery")]
pub use pwright_plugin_ordered_handle_delivery as ordered_handle_delivery;

#[cfg(feature = "context-creation")]
pub use pwright_cap_context_creation as context_creation;

#[cfg(feature = "locator")]
pub use pwright_cap_locator as locator;

#[cfg(feature = "locator-screenshot")]
pub use pwright_cap_locator_screenshot as locator_screenshot;

#[cfg(feature = "dom-content")]
pub use pwright_cap_dom_content as dom_content;

#[cfg(feature = "page-events")]
pub use pwright_cap_page_events as page_events;

#[cfg(feature = "network-events")]
pub use pwright_cap_network_events as network_events;

#[cfg(feature = "client")]
pub use pwright_client as client;

#[cfg(feature = "client-navigation")]
pub use pwright_client_navigation as client_navigation;

#[cfg(feature = "client-evaluation")]
pub use pwright_client_evaluation as client_evaluation;

#[cfg(feature = "client-content")]
pub use pwright_client_content as client_content;

#[cfg(feature = "client-context-lifecycle")]
pub use pwright_client_context_lifecycle as client_context_lifecycle;

#[cfg(feature = "client-page-lifecycle")]
pub use pwright_client_page_lifecycle as client_page_lifecycle;

#[cfg(feature = "client-page-creation")]
pub use pwright_client_page_creation as client_page_creation;

#[cfg(feature = "client-handle-lifecycle")]
pub use pwright_client_handle_lifecycle as client_handle_lifecycle;

#[cfg(feature = "client-handle-evaluation")]
pub use pwright_client_handle_evaluation as client_handle_evaluation;

#[cfg(feature = "client-context-creation")]
pub use pwright_client_context_creation as client_context_creation;

#[cfg(feature = "client-locator")]
pub use pwright_client_locator as client_locator;

#[cfg(feature = "client-locator-screenshot")]
pub use pwright_client_locator_screenshot as client_locator_screenshot;

#[cfg(feature = "client-dom-content")]
pub use pwright_client_dom_content as client_dom_content;

#[cfg(feature = "client-page-events")]
pub use pwright_client_page_events as client_page_events;

#[cfg(feature = "client-network-events")]
pub use pwright_client_network_events as client_network_events;

#[cfg(feature = "cdp")]
pub use pwright_cdp as cdp;

#[cfg(feature = "chromium-content")]
pub use pwright_composition_chromium_content as chromium_content;

#[cfg(feature = "chromium-context-lifecycle")]
pub use pwright_composition_chromium_context_lifecycle as chromium_context_lifecycle;

#[cfg(feature = "chromium-page-lifecycle")]
pub use pwright_composition_chromium_page_lifecycle as chromium_page_lifecycle;

#[cfg(feature = "chromium-page-creation")]
pub use pwright_composition_chromium_page_creation as chromium_page_creation;

#[cfg(feature = "chromium-handle-lifecycle")]
pub use pwright_composition_chromium_handle_lifecycle as chromium_handle_lifecycle;

#[cfg(feature = "chromium-handle-evaluation")]
pub use pwright_composition_chromium_handle_evaluation as chromium_handle_evaluation;

#[cfg(feature = "chromium-context-creation")]
pub use pwright_composition_chromium_context_creation as chromium_context_creation;

#[cfg(feature = "chromium-locator")]
pub use pwright_composition_chromium_locator as chromium_locator;

#[cfg(feature = "chromium-locator-screenshot")]
pub use pwright_composition_chromium_locator_screenshot as chromium_locator_screenshot;

#[cfg(feature = "chromium-selector-geometry")]
pub use pwright_backend_cdp_selector_geometry as chromium_selector_geometry;

#[cfg(feature = "chromium-dom-content")]
pub use pwright_composition_chromium_dom_content as chromium_dom_content;

#[cfg(feature = "chromium-evaluation")]
pub use pwright_composition_chromium_evaluation as chromium_evaluation;

#[cfg(feature = "chromium-navigation")]
pub use pwright_composition_chromium_navigation as chromium_navigation;

#[cfg(feature = "chromium-page-events")]
pub use pwright_composition_chromium_page_events as chromium_page_events;

#[cfg(feature = "chromium-network-events")]
pub use pwright_composition_chromium_network_events as chromium_network_events;

#[cfg(feature = "runtime")]
pub use pwright_runtime as runtime;

#[cfg(feature = "runtime-tokio")]
pub use pwright_runtime_tokio as runtime_tokio;

#[cfg(feature = "transport")]
pub use pwright_transport as transport;

#[cfg(feature = "transport-tokio")]
pub use pwright_transport_tokio as transport_tokio;
