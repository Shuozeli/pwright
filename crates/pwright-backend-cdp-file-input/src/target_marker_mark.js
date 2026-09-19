/* Adapted from Playwright 1.58.2 dom.js and injectedScriptSource.js.
 * Copyright (c) Microsoft Corporation. SPDX-License-Identifier: Apache-2.0
 * See ../LICENSE.playwright. Uses the current utility-realm language intrinsics.
 */
function(marker, node, callId) {
  if (!callId) return;
  if (node.nodeType !== 1) return;
  const markedElements = new Set([node]);
  if (marker.state?.callId !== callId)
    marker.state = undefined;
  const previous = marker.state?.elements || new Set();
  const unmarkEvent = new CustomEvent('__playwright_unmark_target__', {
    bubbles: true,
    cancelable: true,
    detail: callId,
    composed: true
  });
  for (const element of previous) {
    if (!markedElements.has(element))
      element.dispatchEvent(unmarkEvent);
  }
  const markEvent = new CustomEvent('__playwright_mark_target__', {
    bubbles: true,
    cancelable: true,
    detail: callId,
    composed: true
  });
  for (const element of markedElements) {
    if (!previous.has(element))
      element.dispatchEvent(markEvent);
  }
  marker.state = { callId, elements: markedElements };
}
