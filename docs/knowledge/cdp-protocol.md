# CDP Protocol Notes

How pwright maps high-level operations to Chrome DevTools Protocol commands.

## Connection Model

1. Connect to Chrome's WebSocket at `ws://host:port/devtools/browser/...`
2. Browser-level commands (Target domain) are sent without a `sessionId`
3. Tab-level commands require attaching to a target first via `Target.attachToTarget(flatten: true)`, which returns a `sessionId`
4. All subsequent commands for that tab include the `sessionId` in the JSON-RPC message

## Navigation Sequence

```
Page.navigate(url)
  └─ poll: Runtime.evaluate("document.readyState")
     └─ until "interactive" or "complete" (every 200ms)
```

This matches PinchTab's `NavigatePage` pattern — *not* using `Page.loadEventFired` which can race.

## Click Sequence

```
DOM.scrollIntoViewIfNeeded(backendNodeId)
DOM.focus(backendNodeId)
DOM.getBoxModel(backendNodeId)
  └─ extract content quad → compute center (x, y)
  └─ fallback: DOM.resolveNode → Runtime.callFunctionOn("getBoundingClientRect")
Input.dispatchMouseEvent(type: "mousePressed", button: "left", x, y)
Input.dispatchMouseEvent(type: "mouseReleased", button: "left", x, y)
```

## Fill Sequence

```
DOM.focus(backendNodeId)
DOM.resolveNode(backendNodeId) → objectId
Runtime.callFunctionOn(objectId, "function(v) {
    this.value = v;
    this.dispatchEvent(new Event('input', {bubbles: true}));
    this.dispatchEvent(new Event('change', {bubbles: true}));
}")
```

The dispatched events ensure React/Vue/Angular detect the value change.

## Key Press Sequence

```
Input.dispatchKeyEvent(type: "rawKeyDown", key, code, windowsVirtualKeyCode)
Input.insertText(text)          // only for keys that produce characters (Enter→\r, Tab→\t)
Input.dispatchKeyEvent(type: "keyUp", key, code, windowsVirtualKeyCode)
```

Named key definitions map key names to their CDP params:
- `Enter` → code: `"Enter"`, vk: `13`, text: `"\r"`
- `Tab` → code: `"Tab"`, vk: `9`, text: `"\t"`
- `Escape` → code: `"Escape"`, vk: `27`, no text
- `ArrowLeft/Right/Up/Down`, `Home`, `End`, `PageUp/Down`, `Delete`, `Backspace`, `F1-F12`

## Drag Sequence

```
DOM.scrollIntoViewIfNeeded(backendNodeId)
Input.dispatchMouseEvent(type: "mouseMoved", x, y)
Input.dispatchMouseEvent(type: "mousePressed", button: "left", x, y)
  └─ for i in 1..=steps:
     └─ Input.dispatchMouseEvent(type: "mouseMoved", lerp(x→endX), lerp(y→endY), buttons: 1)
Input.dispatchMouseEvent(type: "mouseReleased", button: "left", endX, endY)
```

Steps are proportional to distance: `clamp(dist/10, 5, 40)`.

## Accessibility Snapshot

```
Accessibility.getFullAXTree()
  └─ returns flat array of AX nodes with:
     - nodeId (AX tree internal)
     - backendDOMNodeId (links to DOM for actions)
     - role, name, value, properties (disabled, focused)
     - childIds (for depth calculation)
```

We build a parent map from `childIds`, compute depth for each node, filter by role (interactive roles: button, link, textbox, checkbox, etc.), and assign refs `e0, e1, ...` mapped to `backendDOMNodeId`.

## Resource Blocking

```
Network.setBlockedURLs(urls: ["*.png", "*.jpg", ...])
```

Patterns: images (`*.png, *.jpg, *.jpeg, *.gif, *.webp, *.svg, *.ico`) and media (images + `*.mp4, *.webm, *.ogg, *.mp3, *.wav, *.flac, *.aac`).
