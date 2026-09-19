// Pinned oracle for the Rust host, sharing the Node comparison's public actions.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { writeFile } from 'node:fs/promises';
import { isAbsolute } from 'node:path';
import { collectDragKeyboard } from './drag-keyboard-cases.mjs';

assert.equal(process.env.PWRIGHT_TEST_CDP, 'http://127.0.0.1:9222');
const fixture = new URL(process.env.PWRIGHT_DRAG_FIXTURE);
assert.equal(fixture.protocol, 'http:');
assert.ok(fixture.hostname.endsWith('.ts.net'));
const output = process.env.PWRIGHT_DRAG_KEYBOARD_ORACLE;
assert.ok(typeof output === 'string' && isAbsolute(output));
const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const { result } = await collectDragKeyboard(requireOracle('playwright'), {
  endpoint: process.env.PWRIGHT_TEST_CDP, fixture, label: 'oracle',
});
const encoded = JSON.stringify(result, null, 2);
assert.ok(Buffer.byteLength(encoded) <= 128 * 1024);
await writeFile(output, encoded, { flag: 'wx' });
console.log(`Recorded ${result.observations.length} drag/Keyboard cases after owned Context cleanup`);
