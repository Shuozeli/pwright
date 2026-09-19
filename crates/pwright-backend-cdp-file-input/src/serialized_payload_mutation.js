/* Portions adapted from Playwright 1.58.2 utilityScriptSerializers.ts and
 * injectedScript.ts.
 * Copyright (c) Microsoft Corporation.
 * SPDX-License-Identifier: Apache-2.0
 * See ../LICENSE.playwright for the complete license.
 */
function(node, serializedFiles) {
  const refs = new Map();
  function parse(value) {
    if (value === undefined)
      return undefined;
    if (typeof value === 'object' && value) {
      if ('ref' in value)
        return refs.get(value.ref);
      if ('v' in value) {
        if (value.v === 'undefined') return undefined;
        if (value.v === 'null') return null;
        if (value.v === 'NaN') return NaN;
        if (value.v === 'Infinity') return Infinity;
        if (value.v === '-Infinity') return -Infinity;
        if (value.v === '-0') return -0;
        return undefined;
      }
      if ('a' in value) {
        const result = [];
        refs.set(value.id, result);
        for (const item of value.a)
          result.push(parse(item));
        return result;
      }
      if ('o' in value) {
        const result = {};
        refs.set(value.id, result);
        for (const { k, v } of value.o) {
          if (k !== '__proto__')
            result[k] = parse(v);
        }
        return result;
      }
    }
    return value;
  }

  const payloads = parse(serializedFiles);
  if (node.nodeType !== Node.ELEMENT_NODE)
    return 'Node is not of type HTMLElement';
  const element = node;
  if (element.nodeName !== 'INPUT')
    return 'Not an <input> element';
  const input = element;
  const type = (input.getAttribute('type') || '').toLowerCase();
  if (type !== 'file')
    return 'Not an input[type=file] element';
  const files = payloads.map(file => {
    const bytes = Uint8Array.from(atob(file.buffer), c => c.charCodeAt(0));
    return new File([bytes], file.name, {
      type: file.mimeType,
      lastModified: file.lastModifiedMs,
    });
  });
  const transfer = new DataTransfer();
  for (const file of files)
    transfer.items.add(file);
  input.files = transfer.files;
  input.dispatchEvent(new Event('input', { bubbles: true, composed: true }));
  input.dispatchEvent(new Event('change', { bubbles: true }));
}
