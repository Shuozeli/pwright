/* Portions adapted from Playwright 1.58.2 injectedScript.ts.
 * Copyright (c) Microsoft Corporation.
 * SPDX-License-Identifier: Apache-2.0
 * See ../LICENSE.playwright for the complete license.
 */
function(node, payloads) {
  let element = node && (node.nodeType === Node.ELEMENT_NODE ? node : node.parentElement);
  if (!element) return { status: 'disconnected', target: null };
  if (!element.matches('input, textarea, select') && !element.isContentEditable)
    element = element.closest('button, [role=button], [role=checkbox], [role=radio]') || element;
  if (!element.matches('a, input, textarea, button, select, [role=link], [role=button], [role=checkbox], [role=radio]') && !element.isContentEditable) {
    const label = element.closest('label');
    if (label && label.control) element = label.control;
  }
  const target = {
    input: element.tagName === 'INPUT',
    multiple: !!element.multiple,
    directory: !!element.webkitdirectory,
  };
  // Keep validation and mutation in one synchronous DOM call, retaining the
  // exact retargeted element. Rust validates this snapshot with UploadPlan.
  if (!target.input || (payloads.length > 1 && !target.multiple && !target.directory) || target.directory)
    return { status: 'rejected', target };
  // Pinned Playwright ignores the injected helper's text-input error string.
  if ((element.getAttribute('type') || '').toLowerCase() !== 'file')
    return { status: 'noop', target };
  const files = payloads.map(file => {
    const bytes = Uint8Array.from(atob(file.buffer), c => c.charCodeAt(0));
    return new File([bytes], file.name, { type: file.mimeType, lastModified: file.lastModifiedMs });
  });
  const transfer = new DataTransfer();
  for (const file of files) transfer.items.add(file);
  element.files = transfer.files;
  element.dispatchEvent(new Event('input', { bubbles: true, composed: true }));
  element.dispatchEvent(new Event('change', { bubbles: true }));
  return { status: 'applied', target };
}
