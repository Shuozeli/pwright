// Audits the reviewed, pinned napi-rs class conversion seam. This is a source
// drift gate, not a substitute for Rust tag tests or independent-addon tests.
import assert from 'node:assert/strict';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { homedir } from 'node:os';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const workspace = fileURLToPath(new URL('../../../', import.meta.url));
const addon = join(workspace, 'crates/pwright-node-addon');
const source = join(addon, 'src');
const expected = [
  ['NativeContextSettings', 'context_settings.rs'],
  ['NativeEnvironment', 'environment.rs'],
  ['NativeFileInputPreparation', 'handles/file_input.rs'],
  ['NativeFileInputTimeoutSettings', 'handles/file_input/timeout_policy.rs'],
  ['NativeFrameNode', 'frame_tree.rs'],
  ['NativeFrameTree', 'frame_tree.rs'],
  ['NativeHandle', 'handles.rs'],
  ['NativeLocator', 'locator.rs'],
  ['NativeSession', 'lib.rs'],
  ['NativeSessionSignal', 'lib.rs'],
];

// Only lexical tokens are needed: comments and literals must not create fake
// classes or satisfy a validation-before-cast assertion. Balanced attributes
// support cfg/derive and qualified napi paths without matching DTO fields.
function tokens(text) {
  const result = [];
  for (let index = 0; index < text.length;) {
    const rest = text.slice(index);
    const whitespace = /^\s+/.exec(rest);
    if (whitespace) { index += whitespace[0].length; continue; }
    if (rest.startsWith('//')) {
      const end = text.indexOf('\n', index);
      index = end < 0 ? text.length : end + 1;
      continue;
    }
    if (rest.startsWith('/*')) {
      let depth = 1;
      index += 2;
      while (depth && index < text.length) {
        if (text.startsWith('/*', index)) { depth++; index += 2; }
        else if (text.startsWith('*/', index)) { depth--; index += 2; }
        else index++;
      }
      assert.equal(depth, 0, 'unterminated Rust block comment');
      continue;
    }
    const raw = /^(?:b|c)?r(#+)?"/.exec(rest);
    if (raw) {
      const end = text.indexOf(`"${raw[1] ?? ''}`, index + raw[0].length);
      assert.ok(end >= 0, 'unterminated Rust raw string');
      index = end + 1 + (raw[1]?.length ?? 0);
      result.push('<literal>');
      continue;
    }
    const literal = /^(?:(?:b|c)?"(?:\\[\s\S]|[^"\\])*"|(?:b)?'(?:\\[\s\S]|[^'\\])')/.exec(rest);
    if (literal) { index += literal[0].length; result.push('<literal>'); continue; }
    const identifier = /^(?:r#)?[A-Za-z_][A-Za-z0-9_]*/.exec(rest);
    if (identifier) { result.push(identifier[0]); index += identifier[0].length; continue; }
    if (rest.startsWith('::')) { result.push('::'); index += 2; continue; }
    result.push(text[index++]);
  }
  return result;
}

function closing(values, start, open, close) {
  assert.equal(values[start], open);
  let depth = 1;
  for (let index = start + 1; index < values.length; index++) {
    if (values[index] === open) depth++;
    else if (values[index] === close && --depth === 0) return index;
  }
  assert.fail(`unterminated Rust ${open}`);
}

function rawClasses(values) {
  const classes = [];
  let attributes = [];
  for (let index = 0; index < values.length; index++) {
    if (values[index] === '#' && values[index + 1] === '[') {
      const end = closing(values, index + 1, '[', ']');
      attributes.push(values.slice(index + 2, end));
      index = end;
      continue;
    }
    if (values[index] === 'pub') {
      if (values[index + 1] === '(') index = closing(values, index + 1, '(', ')');
      continue;
    }
    if (values[index] === 'struct') {
      const selected = attributes.find(attribute => {
        const end = attribute.indexOf('(');
        const path = end < 0 ? attribute : attribute.slice(0, end);
        return path.at(-1) === 'napi';
      });
      if (selected && !selected.includes('object')) classes.push(values[index + 1]);
    }
    attributes = [];
  }
  return classes;
}

function find(values, pattern, from = 0) {
  for (let index = from; index <= values.length - pattern.length; index++) {
    if (pattern.every((value, offset) => values[index + offset] === value)) return index;
  }
  return -1;
}

function files(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
    const path = join(directory, entry.name);
    return entry.isDirectory() ? files(path) : entry.name.endsWith('.rs') ? [path] : [];
  });
}

// Exercise comments, raw strings, attributes, qualified macros, and DTO exclusion
// so a broader source match cannot silently weaken the inventory gate.
assert.deepEqual(rawClasses(tokens(`
  // #[napi] pub struct Comment;
  const TEXT: &str = r#"#[napi] pub struct Quoted;"#;
  #[cfg(feature = "selected")] #[napi] #[derive(Debug)] pub(crate) struct Raw;
  #[napi(object, object_from_js = false)] pub struct Dto;
  #[napi_derive::napi] pub struct Qualified;
`)), ['Raw', 'Qualified']);

const discovered = [];
for (const path of files(source)) {
  const values = tokens(readFileSync(path, 'utf8'));
  const file = relative(source, path).replaceAll('\\', '/');
  assert.equal(find(values, ['mod', 'napi']), -1,
    `${file} must not shadow the crate-wide napi facade`);
  for (let index = 0; index < values.length - 1; index++) {
    if (values[index] === 'as' && values[index + 1] === 'napi') {
      assert.ok(file === 'lib.rs' && find(values.slice(Math.max(0, index - 3), index + 3),
        ['extern', 'crate', 'self', 'as', 'napi', ';']) === 0,
      `${file} must not replace the crate-wide napi alias`);
    }
  }
  if (!['lib.rs', 'native_napi.rs'].includes(file)) {
    assert.equal(values.includes('napi_runtime'), false,
      `${file} must use the facade, not the underlying runtime dependency`);
  }
  const classes = rawClasses(values);
  if (!classes.length) continue;
  for (const name of classes) discovered.push([name, file]);
}
assert.deepEqual(discovered.sort(([left], [right]) => left.localeCompare(right)), expected,
  'raw Node classes changed: audit conversion/tag coverage and update the explicit inventory');

const root = tokens(readFileSync(join(source, 'lib.rs'), 'utf8'));
for (const declaration of [
  ['extern', 'crate', 'self', 'as', 'napi', ';'],
  ['pub', 'use', 'napi_runtime', '::', '*', ';'],
  ['pub', 'use', 'native_napi', '::', 'bindgen_prelude', ';'],
]) {
  assert.ok(find(root, declaration) >= 0, `crate-wide napi facade changed: ${declaration.join(' ')}`);
}

// The actual typed registry, not this script's inventory alone, drives the Rust
// module-local/pairwise-distinct tag proof and the no-cast addon diagnostic.
const registryTokens = tokens(readFileSync(join(source, 'native_classes.rs'), 'utf8'));
const registries = [];
for (let start = find(registryTokens, ['native_classes', '!', '(']); start >= 0;
  start = find(registryTokens, ['native_classes', '!', '('], start + 1)) {
  const end = closing(registryTokens, start + 2, '(', ')');
  registries.push(registryTokens.slice(start + 3, end).filter(value => value !== ',').sort());
}
const classNames = expected.map(([name]) => name).sort();
const withoutPreparation = classNames.filter(name => !['NativeFileInputPreparation', 'NativeFileInputTimeoutSettings'].includes(name));
assert.deepEqual(registries.map(names => names.join(',')).sort(), [
  classNames, withoutPreparation, withoutPreparation.filter(name => name !== 'NativeHandle'),
].map(names => names.join(',')).sort(),
  'retained-upload, Handle-only and ordinary typed class registries must cover exactly their selected raw classes');

const manifest = readFileSync(join(addon, 'Cargo.toml'), 'utf8');
const napiDependency = /^napi_runtime\s*=\s*\{[^\n]*\}/m.exec(manifest)?.[0];
assert.ok(napiDependency, 'review the napi dependency declaration');
assert.doesNotMatch(manifest, /^napi\s*=/m,
  'an extern-prelude napi dependency would compete with the crate-wide facade');
assert.match(napiDependency, /package\s*=\s*"napi"/);
assert.match(napiDependency, /version\s*=\s*"=3\.12\.2"/);
assert.match(napiDependency, /features\s*=\s*\[[^\]]*"napi8"/,
  'napi8 is required: lower API versions disable native type-tag validation');
assert.match(manifest, /^napi-derive\s*=\s*"=3\.6\.3"\s*$/m);

const lock = readFileSync(join(workspace, 'Cargo.lock'), 'utf8');
const packages = lock.split(/^\[\[package\]\]\s*$/m).slice(1).map(block => ({
  name: /^name = "([^"]+)"/m.exec(block)?.[1],
  version: /^version = "([^"]+)"/m.exec(block)?.[1],
}));
const versions = { napi: '3.12.2', 'napi-derive': '3.6.3', 'napi-derive-backend': '6.1.2' };
for (const [name, version] of Object.entries(versions)) {
  assert.deepEqual(packages.filter(pkg => pkg.name === name).map(pkg => pkg.version), [version],
    `${name} changed: re-audit native type-tag generation and validation before updating this pin`);
}

const registry = join(process.env.CARGO_HOME || join(homedir(), '.cargo'), 'registry', 'src');
function dependencyFile(name, file) {
  assert.ok(existsSync(registry), 'Cargo registry sources are unavailable; fetch locked dependencies before auditing');
  const candidates = readdirSync(registry, { withFileTypes: true })
    .filter(entry => entry.isDirectory())
    .map(entry => join(registry, entry.name, `${name}-${versions[name]}`, file))
    .filter(path => existsSync(path));
  assert.ok(candidates.length, `locked ${name} source missing: ${file}`);
  const contents = candidates.map(path => readFileSync(path, 'utf8'));
  assert.ok(contents.every(text => text === contents[0]), `ambiguous ${name} source copies: ${file}`);
  return tokens(contents[0]);
}

function functionBody(values, name, anchor = []) {
  const start = anchor.length ? find(values, anchor) : 0;
  assert.ok(start >= 0, `reviewed section disappeared: ${anchor.join(' ')}`);
  const declaration = find(values, ['fn', name], start);
  assert.ok(declaration >= 0, `reviewed function disappeared: ${name}`);
  const body = values.indexOf('{', declaration);
  assert.ok(body >= 0, `reviewed function has no body: ${name}`);
  return values.slice(body + 1, closing(values, body, '{', '}'));
}

function validationBefore(values, name, cast, anchor = []) {
  const body = functionBody(values, name, anchor);
  const validation = find(body, ['validate_type_tag', '(']);
  const conversion = find(body, cast);
  assert.ok(validation >= 0 && conversion > validation,
    `${name} no longer validates its type tag before the reviewed native conversion`);
  const validationEnd = closing(body, validation + 1, '(', ')');
  assert.ok(find(body.slice(validation + 2, validationEnd), ['type_tag', '(']) >= 0,
    `${name} must validate the actual TypeTag trait, not a separate static identity`);
  assert.equal(body[validationEnd + 1], '?', `${name} must propagate type-tag mismatch before conversion`);
}

const generator = dependencyFile('napi-derive-backend', 'src/codegen/struct.rs');
assert.ok(find(generator, ['napi', '::', 'bindgen_prelude', '::', 'type_tag_from_ident', '(']) >= 0,
  'the lexical napi type-tag generation seam changed');
const generatedTag = functionBody(generator, 'type_tag');
assert.deepEqual(generatedTag, ['#', 'type_tag_body'], 'review changed generated TypeTag body');
validationBefore(generator, 'from_napi_ref', ['wrapped_val', '.', 'cast']);
validationBefore(generator, 'from_napi_mut_ref', ['wrapped_val', '.', 'cast']);
validationBefore(dependencyFile('napi', 'src/bindgen_runtime/callback_info.rs'),
  'unwrap_raw', ['wrapped_val', '.', 'cast']);
validationBefore(dependencyFile('napi', 'src/bindgen_runtime/class_accessor.rs'),
  'class_accessor_unwrap_this', ['wrapped_val', '.', 'cast']);
validationBefore(dependencyFile('napi', 'src/bindgen_runtime/js_values/class.rs'),
  'from_napi_value', ['Box', '::', 'from_raw'], ['FromNapiValue', 'for', 'ClassInstance']);
validationBefore(dependencyFile('napi', 'src/bindgen_runtime/js_values/value_ref.rs'),
  'from_napi_value', ['Reference', '::', 'from_value_ptr'], ['FromNapiValue', 'for', 'Reference']);

console.log(`Native class boundary verified (${discovered.length} opaque classes, crate-wide module-local facade, pinned napi8 validation-before-cast paths).`);
