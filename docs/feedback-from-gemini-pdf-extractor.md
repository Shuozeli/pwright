
---

## P1: `set_input_files()` After Menu Click Triggers OS File Picker

**Impact:** Blocks Chrome UI entirely. Requires manual intervention (VNC/kill process) to recover. Discovered during batch processing of 776 PDFs -- Chrome became unresponsive after the OS file picker opened and could not be dismissed via CDP.

**The problem:** The typical file upload flow is:
1. Click a UI button (e.g., "Upload files") which creates a hidden `<input type="file">`
2. Call `set_input_files()` on that input

But step 1 also triggers the native OS file chooser dialog. This dialog is outside Chrome's DOM -- CDP has no control over it. `Input.dispatchKeyEvent` with Escape doesn't reach it. `xdotool` requires DISPLAY access. The dialog blocks Chrome's UI thread, making all subsequent CDP commands hang or fail.

**What happens in batch processing:** The first PDF works (human may have pre-dismissed the dialog). But on subsequent iterations, the script opens a new tab, clicks the upload button, and the OS dialog opens. `set_input_files()` may or may not work depending on timing. After ~400 iterations, Chrome becomes unstable and the CDP connection drops.

**Root cause:** `DOM.setFileInputFiles` is designed to set files on an `<input type="file">` *without* opening the OS dialog. But if the application's JS click handler opens the dialog *before* `set_input_files` is called, both fire. There is no CDP method to suppress or dismiss the native file picker.

**Suggested fix in pwright-bridge:**

Option A: Add a `set_input_files_silent()` that creates the input element via JS, sets files, and dispatches a change event -- bypassing the application's click handler entirely:
```rust
pub async fn set_input_files_silent(&self, files: &[String]) -> CdpResult<()> {
    // 1. Find or create <input type="file">
    // 2. Call DOM.setFileInputFiles (no OS dialog)
    // 3. Dispatch 'change' event on the input
    // No click on any trigger button needed
}
```

Option B: Document the hazard prominently and recommend callers use `DOM.setFileInputFiles` directly without clicking trigger buttons.

**Current workaround:** Avoid clicking any upload trigger button. If the hidden `<input type="file">` already exists in the DOM, call `set_input_files()` directly. If it doesn't exist yet, create it via `page.evaluate("document.body.appendChild(Object.assign(document.createElement('input'), {type: 'file'}))")` and then set files on it.
