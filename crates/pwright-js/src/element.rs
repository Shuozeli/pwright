//! Element interaction JavaScript snippets.
//!
//! These are used via `Runtime.callFunctionOn(objectId, fn, args)` where
//! `this` is bound to the resolved DOM element.

/// Get element center via getBoundingClientRect.
/// Returns `{ x: number, y: number }`.
///
/// Used via `callFunctionOn` — `this` is the element.
pub const GET_BOUNDING_CENTER: &str = r#"function() {
    var r = this.getBoundingClientRect();
    return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
}"#;

/// Set an input element's value and dispatch input+change events.
///
/// Used via `callFunctionOn(objectId, SET_VALUE, [{ value: "text" }])`.
pub const SET_VALUE: &str = r#"function(v) {
    this.value = v;
    this.dispatchEvent(new Event('input', {bubbles: true}));
    this.dispatchEvent(new Event('change', {bubbles: true}));
}"#;

/// Get textContent of an element.
/// Used via `callFunctionOn` — `this` is the element.
pub const GET_TEXT_CONTENT: &str = "function() { return this.textContent; }";

/// Get innerText of an element (layout-aware).
/// Used via `callFunctionOn` — `this` is the element.
pub const GET_INNER_TEXT: &str = "function() { return this.innerText; }";

/// Get the value of an input/textarea/select element.
/// Used via `callFunctionOn` — `this` is the element.
pub const GET_INPUT_VALUE: &str = "function() { return this.value; }";

/// Blur (unfocus) an element.
/// Used via `callFunctionOn` — `this` is the element.
pub const BLUR: &str = "function() { this.blur(); }";

/// Dispatch a custom event.
/// Used via `callFunctionOn(objectId, DISPATCH_EVENT, [{ value: "click" }])`.
pub const DISPATCH_EVENT: &str = r#"function(type) {
    this.dispatchEvent(new Event(type, {bubbles: true}));
}"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_bounding_center_is_function() {
        assert!(GET_BOUNDING_CENTER.starts_with("function()"));
        assert!(GET_BOUNDING_CENTER.contains("getBoundingClientRect"));
    }

    #[test]
    fn test_set_value_dispatches_events() {
        assert!(SET_VALUE.starts_with("function(v)"));
        assert!(SET_VALUE.contains("this.value = v"));
        assert!(SET_VALUE.contains("dispatchEvent"));
        assert!(SET_VALUE.contains("input"));
        assert!(SET_VALUE.contains("change"));
    }
}
