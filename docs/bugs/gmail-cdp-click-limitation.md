# Gmail (and similar SPAs) cannot be controlled via pwright click

**Date:** 2026-03-16
**Severity:** High — blocks real-world automation of major web apps
**Affects:** `pwright click`, `pwright fill`, and any ref-based action commands

## Problem

### 1. Empty accessibility snapshots

Gmail's modern UI renders an empty accessibility tree from pwright's
perspective. `pwright snapshot` returns `(empty snapshot)` on Gmail inbox
and email views. This means **no element refs are available**, so all
ref-based commands (`click`, `fill`, `hover`, `select`, etc.) are unusable.

Other heavy SPAs (Google Docs, Slack web, etc.) likely have the same issue.

### 2. JS-dispatched events are ignored by Gmail

As a workaround, we tried using `pwright eval` to find elements by CSS
selector and call `.click()` or `dispatchEvent(new MouseEvent(...))` on
them. **Gmail ignores all of these.** Gmail uses a custom event delegation
system that only responds to real browser-level input events — it does not
trust synthetic JS events created via `document.createElement` or
`dispatchEvent`.

Specific things that were tried and failed:
- `element.click()` — ignored by Gmail's event system
- `new MouseEvent('click', {bubbles: true})` — ignored
- `new MouseEvent('mousedown/mouseup/click')` sequence — ignored
- Context menu via `new MouseEvent('contextmenu')` — ignored
- Setting checkbox `aria-checked` attribute directly — no effect
- Forcing hidden toolbar buttons visible via style overrides — Gmail's CSS
  framework overrides inline styles; buttons remain at 0×0 dimensions
- Gmail internal API POST requests — blocked by CSRF token (`at` param);
  `GM_ACTION_TOKEN` is set to `'null'` in the modern UI

### 3. No coordinate-based click command

pwright currently only supports clicking by **ref** (from snapshot). There
is no way to click at arbitrary **(x, y) pixel coordinates** using CDP's
`Input.dispatchMouseEvent`. This is the missing piece — if we could click
by coordinates, we could:
- Use `pwright eval` to find an element and get its `getBoundingClientRect()`
- Then use a coordinate-based click to send a real CDP input event

## Solution: `click-at` (coordinate-based CDP click) — IMPLEMENTED

### CLI commands added

```bash
pwright click-at <x> <y> [--button left|right|middle] [--click-count 1|2]
pwright dblclick <ref>
pwright hover-at <x> <y>
```

`click-at` sends real CDP `Input.dispatchMouseEvent` (mouseMoved + mousePressed
+ mouseReleased) at the given viewport coordinates. These are real input events
processed by the browser's input pipeline — Gmail and all SPAs respond to them.

`--click-count 2` for double-click, `--button right` for right-click/context menu.

### Workflow for Gmail deletion (once implemented)

```bash
# 1. Find the checkbox coordinates via JS
pwright eval "(() => {
  var row = document.querySelectorAll('tr.zA')[0];
  var cb = row.querySelector('[role=checkbox]');
  var r = cb.getBoundingClientRect();
  return Math.round(r.x + r.width/2) + ',' + Math.round(r.y + r.height/2);
})()"
# Output: 282,293

# 2. Click the checkbox with real CDP input
pwright click-at 282 293

# 3. Find the delete toolbar button (now visible after selection)
pwright eval "(() => {
  var del = document.querySelector('[data-tooltip=\"Delete\"]');
  var r = del.getBoundingClientRect();
  return Math.round(r.x + r.width/2) + ',' + Math.round(r.y + r.height/2);
})()"

# 4. Click delete
pwright click-at <x> <y>
```

### Bonus: improve snapshot for SPAs

The empty snapshot issue may be caused by Gmail using shadow DOM or
ARIA attributes that the current accessibility tree walker doesn't
traverse. Investigating `Accessibility.getFullAXTree` vs
`Accessibility.getPartialAXTree` in CDP may help recover some nodes.
This is a separate issue but worth noting.

## References

- CDP Input domain: https://chromedevtools.github.io/devtools-protocol/tot/Input/
- CDP Accessibility domain: https://chromedevtools.github.io/devtools-protocol/tot/Accessibility/
- Playwright's click implementation sends real `Input.dispatchMouseEvent`
