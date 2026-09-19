/* Adapted from Playwright 1.58.2 dom.ts/injectedScript.ts.
 * Copyright (c) Microsoft Corporation. SPDX-License-Identifier: Apache-2.0
 * See ../LICENSE.playwright. Fixed policy status codes are interpreted in Rust.
 */
function(node, flags) {
  let element = node.nodeType === Node.ELEMENT_NODE ? node : node.parentElement;
  if (!element) return 0;
  if (!element.matches('input, textarea, select') && !element.isContentEditable)
    element = element.closest('button, [role=button], [role=checkbox], [role=radio]') || element;
  if (!element.matches('a, input, textarea, button, select, [role=link], [role=button], [role=checkbox], [role=radio]') && !element.isContentEditable) {
    const label = element.closest('label');
    if (label && label.control) element = label.control;
  }
  if (element.tagName !== 'INPUT') return 1;
  if (flags.multiple && !element.multiple && !element.webkitdirectory) return 2;
  if (flags.directoryUpload && !element.webkitdirectory) return 3;
  if (!flags.directoryUpload && element.webkitdirectory) return 4;
  return element;
}
