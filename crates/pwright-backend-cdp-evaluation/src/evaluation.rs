//! Bounded, value-only JavaScript evaluation over one target-local CDP session.

use std::collections::BTreeSet;
use std::future::Future;
use std::num::NonZeroU32;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use pwright_browser_backend::NativeBackendAccess;
use pwright_cap_evaluation::{
    EvaluateParams, EvaluateResult, EvaluationErrorDetails, EvaluationMode, EvaluationNumber,
    EvaluationProperty, EvaluationValue, EvaluationValueLimits, TypedArrayKind,
};
use pwright_cdp::CdpClient;
use pwright_cdp::connection::CdpError;
use pwright_core::{
    FailureKind, OperationError, ProductionResourceError, ProductionResourceLedger, ResourceClass,
    ResourceError, ResourceLease, ResourceLedger, ResourceRequirements, ResourceScope, Service,
};
use serde_json::{Map, Value};

#[doc(hidden)]
pub const EVALUATION_RUNTIME: &str = r#"function(c) {
  'use strict';
  const handles = Array.prototype.slice.call(arguments, 1);
  const isOwn = c.constructor.prototype.propertyIsEnumerable;
  // `c` is materialized by CDP itself, so its prototype keeps the realm's
  // intrinsic Object.prototype even after page code overwrites global
  // URL/Date/RegExp bindings. Avoid consulting those mutable globals while
  // classifying result values.
  const objectTag = value => c.constructor.prototype.toString.call(value);
  // Chunk-enabled calls retain only genuinely large top-level strings as CDP
  // objects. Every ordinary envelope returns as a primitive string in the
  // original Runtime.callFunctionOn reply, so a navigation cannot destroy its
  // context between evaluation and a second projection command. Do not use the
  // page's mutable JSON/toJSON hooks while constructing that inline envelope.
  const quoteInline = value => {
    let result = '"';
    const hex = '0123456789abcdef';
    for (let i = 0; i < value.length; i++) {
      const code = value.charCodeAt(i);
      if (code === 0x22) result += '\\"';
      else if (code === 0x5c) result += '\\\\';
      else if (code === 0x08) result += '\\b';
      else if (code === 0x0c) result += '\\f';
      else if (code === 0x0a) result += '\\n';
      else if (code === 0x0d) result += '\\r';
      else if (code === 0x09) result += '\\t';
      else if (code < 0x20) {
        result += '\\u00' + hex[(code >>> 4) & 15] + hex[code & 15];
      } else {
        result += value[i];
      }
    }
    return result + '"';
  };
  const serializeInline = value => {
    if (value === null) return 'null';
    if (typeof value === 'string') return quoteInline(value);
    if (typeof value === 'number' || typeof value === 'boolean') return '' + value;
    if (objectTag(value) === '[object Array]') {
      let result = '[';
      for (let i = 0; i < value.length; i++) {
        if (i !== 0) result += ',';
        result += serializeInline(value[i]);
      }
      return result + ']';
    }
    let result = '{';
    let first = true;
    for (const propertyName in value) {
      if (!isOwn.call(value, propertyName)) continue;
      if (!first) result += ',';
      first = false;
      result += quoteInline(propertyName) + ':' + serializeInline(value[propertyName]);
    }
    return result + '}';
  };
  const finish = envelope => {
    if (c.chunkUtf16Units === 0) return envelope;
    const value = envelope && envelope.ok === true && envelope.value;
    if (value && value.t === 'chunked-str') return envelope;
    return '__pwright_evaluation_json__' + serializeInline(envelope);
  };
  const utf8Length = value => {
    let bytes = 0;
    for (let i = 0; i < value.length; i++) {
      const code = value.charCodeAt(i);
      if (code < 0x80) bytes += 1;
      else if (code < 0x800) bytes += 2;
      else if (code >= 0xd800 && code <= 0xdbff && i + 1 < value.length &&
               value.charCodeAt(i + 1) >= 0xdc00 && value.charCodeAt(i + 1) <= 0xdfff) {
        bytes += 4;
        i += 1;
      } else bytes += 3;
    }
    return bytes;
  };
  const clipped = value => {
    value = String(value);
    if (utf8Length(value) <= c.errorBytes) return value;
    let bytes = 0;
    let end = 0;
    while (end < value.length) {
      const code = value.charCodeAt(end);
      let width = code < 0x80 ? 1 : code < 0x800 ? 2 : 3;
      let units = 1;
      if (code >= 0xd800 && code <= 0xdbff && end + 1 < value.length &&
          value.charCodeAt(end + 1) >= 0xdc00 && value.charCodeAt(end + 1) <= 0xdfff) {
        width = 4;
        units = 2;
      }
      if (bytes + width > c.errorBytes) break;
      bytes += width;
      end += units;
    }
    return value.slice(0, end);
  };
  const errorRecord = error => ({
    name: clipped(error && error.name !== undefined ? error.name : 'Error'),
    message: clipped(error && error.message !== undefined ? error.message : error),
    stack: error && typeof error.stack === 'string' ? clipped(error.stack) : null
  });
  const typedConstructors = {
    Int8Array, Uint8Array, Uint8ClampedArray, Int16Array, Uint16Array,
    Int32Array, Uint32Array, Float32Array, Float64Array,
    ...(typeof BigInt64Array === 'undefined' ? {} : { BigInt64Array }),
    ...(typeof BigUint64Array === 'undefined' ? {} : { BigUint64Array })
  };
  const fromBase64 = text => {
    const binary = atob(text);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
    return bytes;
  };
  const decodeRefs = new Map();
  const decode = value => {
    switch (value.t) {
      case 'u': return undefined;
      case 'null': return null;
      case 'bool': return value.v;
      case 'num': return value.v;
      case 'nan': return NaN;
      case 'inf': return Infinity;
      case '-inf': return -Infinity;
      case '-0': return -0;
      case 'str': return value.v;
      case 'bigint': return BigInt(value.v);
      case 'date': return new Date(value.v);
      case 'url': return new URL(value.v);
      case 'regexp': return new RegExp(value.p, value.f);
      case 'error': {
        const error = new Error(value.m);
        error.name = value.n;
        if (value.s !== null) error.stack = value.s;
        return error;
      }
      case 'typed': {
        const Constructor = typedConstructors[value.k];
        if (!Constructor) throw new TypeError('unsupported typed array kind');
        const bytes = fromBase64(value.b);
        return new Constructor(bytes.buffer, 0, bytes.byteLength / Constructor.BYTES_PER_ELEMENT);
      }
      case 'array': {
        const result = [];
        if (value.id !== null) decodeRefs.set(value.id, result);
        for (const item of value.v) result.push(decode(item));
        return result;
      }
      case 'object': {
        const result = {};
        if (value.id !== null) decodeRefs.set(value.id, result);
        for (const property of value.v) result[property.k] = decode(property.v);
        return result;
      }
      case 'ref': {
        if (!decodeRefs.has(value.id)) throw new TypeError('unknown evaluation reference');
        return decodeRefs.get(value.id);
      }
      case 'handle': {
        if (!Number.isInteger(value.i) || value.i < 0 || value.i >= handles.length)
          throw new TypeError('unknown Handle argument slot');
        return handles[value.i];
      }
      default: throw new TypeError('unknown evaluation value tag');
    }
  };
  const state = { depth: 0, nodes: 0, entries: 0, strings: 0, binary: 0, nextId: 1 };
  const seen = new Map();
  const throwLimit = message => {
    const error = new Error(message);
    error.__pwrightLimit = true;
    throw error;
  };
  const claimString = value => {
    state.strings += utf8Length(value);
    if (state.strings > c.maxInlineStringBytes) throwLimit('evaluation string-byte limit exceeded');
  };
  const claimNode = depth => {
    if (depth > c.maxDepth) throwLimit('evaluation depth limit exceeded');
    state.nodes += 1;
    if (state.nodes > c.maxNodes) throwLimit('evaluation node limit exceeded');
  };
  const claimEntry = () => {
    state.entries += 1;
    if (state.entries > c.maxEntries) throwLimit('evaluation entry limit exceeded');
  };
  const bytesToBase64 = bytes => {
    const parts = [];
    for (let offset = 0; offset < bytes.length; offset += 0x8000) {
      let binary = '';
      const end = Math.min(offset + 0x8000, bytes.length);
      for (let i = offset; i < end; i++) binary += String.fromCharCode(bytes[i]);
      parts.push(btoa(binary));
    }
    if (parts.length <= 1) return parts[0] || '';
    let binary = '';
    for (const part of parts) binary += atob(part);
    return btoa(binary);
  };
  const encode = (value, depth) => {
    claimNode(depth);
    if (value === undefined) return { t: 'u' };
    if (value === null) return { t: 'null' };
    if (typeof value === 'boolean') return { t: 'bool', v: value };
    if (typeof value === 'number') {
      if (Number.isNaN(value)) return { t: 'nan' };
      if (value === Infinity) return { t: 'inf' };
      if (value === -Infinity) return { t: '-inf' };
      if (Object.is(value, -0)) return { t: '-0' };
      return { t: 'num', v: value };
    }
    if (typeof value === 'string') {
      const bytes = utf8Length(value);
      if (depth === 0 && bytes > c.maxInlineStringBytes) {
        if (bytes > c.maxStringBytes || c.chunkUtf16Units === 0)
          throwLimit('evaluation string-byte limit exceeded');
        return { t: 'chunked-str', v: value, b: bytes };
      }
      claimString(value);
      return { t: 'str', v: value };
    }
    if (typeof value === 'bigint') {
      const text = value.toString();
      claimString(text);
      return { t: 'bigint', v: text };
    }
    if (typeof value === 'symbol' || typeof value === 'function') return { t: 'u' };
    if (value && typeof value === 'object') {
      let browserReference = null;
      if (typeof globalThis.Window === 'function' && value instanceof globalThis.Window)
        browserReference = 'ref: <Window>';
      else if (typeof globalThis.Document === 'function' && value instanceof globalThis.Document)
        browserReference = 'ref: <Document>';
      else if (typeof globalThis.Node === 'function' && value instanceof globalThis.Node)
        browserReference = 'ref: <Node>';
      if (browserReference !== null) {
        claimString(browserReference);
        return { t: 'str', v: browserReference };
      }
    }
    const tag = objectTag(value);
    if (tag === '[object Date]') {
      const text = value.toISOString();
      claimString(text);
      return { t: 'date', v: text };
    }
    if (tag === '[object URL]') {
      const text = value.href;
      claimString(text);
      return { t: 'url', v: text };
    }
    if (tag === '[object RegExp]') {
      claimString(value.source);
      claimString(value.flags);
      return { t: 'regexp', p: value.source, f: value.flags };
    }
    // Chromium 154 added `navigationId` to PerformanceMeasure's JSON shape.
    // Playwright's public value contract at our pinned v1.58.0 revision only
    // publishes the stable PerformanceEntry fields below.
    if (tag === '[object PerformanceMeasure]') {
      return encode({
        name: value.name,
        entryType: value.entryType,
        startTime: value.startTime,
        duration: value.duration
      }, depth);
    }
    if (value instanceof Error) {
      const record = errorRecord(value);
      const header = record.name + ': ' + record.message;
      if (typeof value.stack !== 'string' || !value.stack.startsWith(header))
        record.stack = clipped(header + '\n' + value.stack);
      claimString(record.name);
      claimString(record.message);
      if (record.stack !== null) claimString(record.stack);
      return { t: 'error', n: record.name, m: record.message, s: record.stack };
    }
    if (ArrayBuffer.isView(value) && !(value instanceof DataView)) {
      const kind = value.constructor && value.constructor.name;
      if (!isOwn.call(typedConstructors, kind)) throw new TypeError('unsupported typed array kind');
      const bytes = new Uint8Array(value.buffer, value.byteOffset, value.byteLength);
      state.binary += bytes.byteLength;
      if (state.binary > c.maxBinaryBytes) throwLimit('evaluation binary-byte limit exceeded');
      return { t: 'typed', k: kind, b: bytesToBase64(bytes) };
    }
    if (seen.has(value)) return { t: 'ref', id: seen.get(value) };
    const id = state.nextId++;
    if (id > 0xffffffff) throwLimit('evaluation reference-ID space exhausted');
    seen.set(value, id);
    if (Array.isArray(value)) {
      const items = [];
      for (const item of value) {
        claimEntry();
        items.push(encode(item, depth + 1));
      }
      return { t: 'array', id, v: items };
    }
    const properties = [];
    for (const key in value) {
      if (!isOwn.call(value, key)) continue;
      let item;
      try {
        item = value[key];
      } catch (_) {
        continue;
      }
      claimEntry();
      claimString(key);
      if (key === 'toJSON' && typeof item === 'function') {
        claimNode(depth + 1);
        properties.push({ k: key, v: { t: 'object', id: null, v: [] } });
      } else {
        properties.push({ k: key, v: encode(item, depth + 1) });
      }
    }
    if (properties.length === 0) {
      try {
        if (typeof value.toJSON === 'function') return encode(value.toJSON(), depth);
      } catch (_) {
      }
    }
    return { t: 'object', id, v: properties };
  };
  let argument;
  if (c.hasArgument) {
    try {
      argument = decode(c.argument);
    } catch (error) {
      return finish({ ok: false, kind: 'invalid-value', message: clipped(error && error.message !== undefined ? error.message : error) });
    }
  }
  // Keep the public API name in browser-created Error stacks. Chromium records
  // this method frame when user code constructs an Error during evaluate().
  const evaluator = {
    evaluate() {
      // Indirect evaluation runs in the selected main world, not this wrapper's scope.
      if (c.mode === 'expression') return globalThis.eval(c.source);
      const fn = globalThis.eval('(' + c.source + ')');
      if (typeof fn !== 'function') throw new TypeError('evaluation source is not a function');
      if (c.receiverSlot === null) return fn(argument);
      if (!Number.isInteger(c.receiverSlot) || c.receiverSlot < 0 || c.receiverSlot >= handles.length)
        throw new TypeError('unknown Handle receiver slot');
      return fn(handles[c.receiverSlot], argument);
    }
  };
  const javascriptFailure = error =>
    finish({ ok: false, kind: 'javascript', error: errorRecord(error) });
  const encodeResult = result => {
    try {
      return finish({ ok: true, value: encode(result, 0) });
    } catch (error) {
      return finish({ ok: false, kind: error && error.__pwrightLimit ? 'resource-exhausted' : 'invalid-value', message: clipped(error && error.message !== undefined ? error.message : error) });
    }
  };
  try {
    const result = evaluator.evaluate();
    // Keep ordinary values synchronous, as in Playwright's utility-script
    // path. Only actual thenables need an asynchronous settlement turn.
    if (result !== null && typeof result === 'object' && typeof result.then === 'function') {
      return (async () => {
        try {
          // Match Playwright's object-only Promise awareness: `await` reads
          // the user-owned then property again during assimilation.
          const settled = await result;
          return encodeResult(settled);
        } catch (error) {
          return javascriptFailure(error);
        }
      })();
    }
    return encodeResult(result);
  } catch (error) {
    return javascriptFailure(error);
  }
}"#;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CdpEvaluationLimits {
    pub source_bytes: usize,
    pub generated_script_bytes: usize,
    pub error_bytes: usize,
    pub value: EvaluationValueLimits,
    input_string_bytes: usize,
    inline_string_bytes: usize,
    chunk_utf16_units: usize,
}

impl CdpEvaluationLimits {
    pub const fn new(
        source_bytes: usize,
        generated_script_bytes: usize,
        error_bytes: usize,
        value: EvaluationValueLimits,
    ) -> Self {
        Self {
            source_bytes,
            generated_script_bytes,
            error_bytes,
            value,
            input_string_bytes: value.max_string_bytes(),
            inline_string_bytes: value.max_string_bytes(),
            chunk_utf16_units: 0,
        }
    }

    /// Bound only argument strings independently of results. All other input
    /// dimensions inherit the result limits, keeping this profile narrow.
    pub const fn with_input_string_bytes(mut self, input_string_bytes: usize) -> Self {
        self.input_string_bytes = input_string_bytes;
        self
    }

    pub fn input_value_limits(self) -> EvaluationValueLimits {
        EvaluationValueLimits::new(
            self.value.max_depth(),
            self.value.max_nodes(),
            self.value.max_entries(),
            self.input_string_bytes,
            self.value.max_binary_bytes(),
        )
        .expect("selected input string limit preserves validated value dimensions")
    }

    /// Select a bounded top-level string result reader. Ordinary graphs keep
    /// the smaller inline string bound; only one direct string may use the
    /// larger value limit through fixed-size CDP projections.
    pub fn with_chunked_string_results(
        mut self,
        inline_string_bytes: usize,
        chunk_utf16_units: usize,
    ) -> Result<Self, &'static str> {
        if inline_string_bytes == 0
            || inline_string_bytes > self.value.max_string_bytes()
            || chunk_utf16_units == 0
        {
            return Err("invalid chunked evaluation string limits");
        }
        self.inline_string_bytes = inline_string_bytes;
        self.chunk_utf16_units = chunk_utf16_units;
        Ok(self)
    }

    pub const fn inline_string_bytes(self) -> usize {
        self.inline_string_bytes
    }

    pub const fn chunk_utf16_units(self) -> usize {
        self.chunk_utf16_units
    }

    pub const fn chunked_string_results(self) -> bool {
        self.chunk_utf16_units != 0
    }

    /// Conservative host-side peak for input, generated script, CDP JSON,
    /// base64 staging, and the final portable value.
    pub fn maximum_retained_bytes_per_operation(self) -> u64 {
        let source = u64::try_from(self.source_bytes).unwrap_or(u64::MAX);
        let script = u64::try_from(self.generated_script_bytes).unwrap_or(u64::MAX);
        let error = u64::try_from(self.error_bytes).unwrap_or(u64::MAX);
        let strings = u64::try_from(self.inline_string_bytes).unwrap_or(u64::MAX);
        let input_strings = u64::try_from(self.input_string_bytes).unwrap_or(u64::MAX);
        let binary = u64::try_from(self.value.max_binary_bytes()).unwrap_or(u64::MAX);
        let entries = u64::try_from(self.value.max_entries()).unwrap_or(u64::MAX);
        source
            .saturating_mul(2)
            .saturating_add(script.saturating_mul(2))
            .saturating_add(error.saturating_mul(4))
            .saturating_add(strings.saturating_mul(8))
            .saturating_add(input_strings.saturating_sub(strings).saturating_mul(8))
            .saturating_add(binary.saturating_mul(6))
            .saturating_add(entries.saturating_mul(96))
            .saturating_add(64 * 1024)
    }

    pub fn maximum_chunked_string_retained_bytes(self) -> u64 {
        if !self.chunked_string_results() {
            return 0;
        }
        let strings = u64::try_from(self.value.max_string_bytes()).unwrap_or(u64::MAX);
        let chunks = u64::try_from(self.chunk_utf16_units).unwrap_or(u64::MAX);
        strings
            .saturating_mul(2)
            .saturating_add(chunks.saturating_mul(24))
            .saturating_add(64 * 1024)
    }
}

impl Default for CdpEvaluationLimits {
    fn default() -> Self {
        Self::new(
            1024 * 1024,
            16 * 1024 * 1024,
            64 * 1024,
            EvaluationValueLimits::new(32, 16 * 1024, 16 * 1024, 8 * 1024 * 1024, 8 * 1024 * 1024)
                .expect("default evaluation limits are valid"),
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CdpEvaluationProductionProfile {
    user_gesture: bool,
    scope: ResourceScope,
    limits: CdpEvaluationLimits,
    max_concurrent_operations: usize,
    max_context_routes: usize,
    max_native_id_bytes: usize,
    maximum_retained_bytes_per_operation: u64,
    maximum_chunked_string_retained_bytes: u64,
    maximum_route_retained_bytes: u64,
    requirements: ResourceRequirements,
}

impl CdpEvaluationProductionProfile {
    pub fn new(
        scope: ResourceScope,
        limits: CdpEvaluationLimits,
        max_concurrent_operations: usize,
    ) -> Self {
        let maximum_retained_bytes_per_operation = limits.maximum_retained_bytes_per_operation();
        let maximum_chunked_string_retained_bytes = limits.maximum_chunked_string_retained_bytes();
        let operations = u64::try_from(max_concurrent_operations).unwrap_or(u64::MAX);
        let bytes = maximum_retained_bytes_per_operation
            .saturating_mul(operations)
            .saturating_add(maximum_chunked_string_retained_bytes);
        let requirements = if max_concurrent_operations == 0 {
            ResourceRequirements::none()
        } else {
            ResourceRequirements::none()
                .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
                .with_metadata(
                    1,
                    max_concurrent_operations
                        .saturating_add(usize::from(maximum_chunked_string_retained_bytes > 0)),
                )
        };
        Self {
            scope,
            user_gesture: false,
            limits,
            max_concurrent_operations,
            max_context_routes: 0,
            max_native_id_bytes: 0,
            maximum_retained_bytes_per_operation,
            maximum_chunked_string_retained_bytes,
            maximum_route_retained_bytes: 0,
            requirements,
        }
    }

    /// Select Playwright-style transient activation only for user evaluations.
    /// Default profiles and internal CDP reads retain their original behavior.
    pub fn with_user_gesture(mut self, enabled: bool) -> Self {
        self.user_gesture = enabled;
        self
    }

    pub const fn uses_user_gesture(&self) -> bool {
        self.user_gesture
    }

    /// Adds the persistent route budget required to target Frame execution
    /// worlds. Five native strings conservatively cover the route identity,
    /// its unique context ID, and the target-local CDP client's copies.
    pub fn with_context_routes(
        mut self,
        max_context_routes: usize,
        max_native_id_bytes: usize,
    ) -> Self {
        self.max_context_routes = max_context_routes;
        self.max_native_id_bytes = max_native_id_bytes;
        self.maximum_route_retained_bytes = u64::try_from(max_context_routes)
            .unwrap_or(u64::MAX)
            .saturating_mul(5)
            .saturating_mul(u64::try_from(max_native_id_bytes).unwrap_or(u64::MAX));
        let operations = u64::try_from(self.max_concurrent_operations).unwrap_or(u64::MAX);
        let bytes = self
            .maximum_retained_bytes_per_operation
            .saturating_mul(operations)
            .saturating_add(self.maximum_chunked_string_retained_bytes)
            .saturating_add(self.maximum_route_retained_bytes);
        self.requirements = if bytes == 0 {
            ResourceRequirements::none()
        } else {
            ResourceRequirements::none()
                .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
                .with_metadata(
                    1,
                    self.max_concurrent_operations
                        .saturating_add(usize::from(self.maximum_route_retained_bytes > 0))
                        .saturating_add(usize::from(
                            self.maximum_chunked_string_retained_bytes > 0,
                        )),
                )
        };
        self
    }

    pub const fn resource_scope(self) -> ResourceScope {
        self.scope
    }
    pub const fn limits(self) -> CdpEvaluationLimits {
        self.limits
    }
    pub const fn max_concurrent_operations(self) -> usize {
        self.max_concurrent_operations
    }
    pub const fn max_context_routes(self) -> usize {
        self.max_context_routes
    }
    pub const fn max_native_id_bytes(self) -> usize {
        self.max_native_id_bytes
    }
    pub const fn maximum_retained_bytes_per_operation(self) -> u64 {
        self.maximum_retained_bytes_per_operation
    }
    pub const fn maximum_chunked_string_retained_bytes(self) -> u64 {
        self.maximum_chunked_string_retained_bytes
    }
    pub const fn maximum_route_retained_bytes(self) -> u64 {
        self.maximum_route_retained_bytes
    }
    pub const fn requirements(self) -> ResourceRequirements {
        self.requirements
    }
}

#[derive(Clone)]
struct EvaluationResourceBudget {
    ledger: ResourceLedger,
    profile: CdpEvaluationProductionProfile,
    active: Arc<AtomicUsize>,
    large_active: Arc<AtomicUsize>,
    next_group: Arc<AtomicU64>,
}

#[doc(hidden)]
pub struct EvaluationLease {
    _resource: Option<ResourceLease>,
    active: Arc<AtomicUsize>,
}

struct ChunkedStringLease {
    _resource: Option<ResourceLease>,
    active: Arc<AtomicUsize>,
}

impl Drop for ChunkedStringLease {
    fn drop(&mut self) {
        let previous = self.active.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(previous > 0);
    }
}

impl Drop for EvaluationLease {
    fn drop(&mut self) {
        let previous = self.active.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(previous > 0);
    }
}

#[derive(Clone)]
pub struct CdpEvaluationService {
    client: Arc<dyn CdpClient>,
    execution_context_id: Option<i64>,
    unique_context_id: Option<Arc<str>>,
    limits: CdpEvaluationLimits,
    resources: Option<EvaluationResourceBudget>,
}

#[derive(Clone)]
pub struct CdpEvaluationServiceFactory {
    limits: CdpEvaluationLimits,
    resources: EvaluationResourceBudget,
}

impl CdpEvaluationServiceFactory {
    pub fn new_with_production_resources(
        profile: CdpEvaluationProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ProductionResourceError> {
        let ledger = resources.claim(profile.requirements())?;
        Ok(Self {
            limits: profile.limits(),
            resources: EvaluationResourceBudget {
                ledger,
                profile,
                active: Arc::new(AtomicUsize::new(0)),
                large_active: Arc::new(AtomicUsize::new(0)),
                next_group: Arc::new(AtomicU64::new(1)),
            },
        })
    }

    pub fn bind(&self, client: Arc<dyn CdpClient>) -> CdpEvaluationService {
        CdpEvaluationService {
            client,
            execution_context_id: None,
            unique_context_id: None,
            limits: self.limits,
            resources: Some(self.resources.clone()),
        }
    }

    pub fn bind_unique_context(
        &self,
        client: Arc<dyn CdpClient>,
        mut unique_context_id: String,
    ) -> CdpEvaluationService {
        unique_context_id.shrink_to_fit();
        self.bind_shared_unique_context(client, Arc::from(unique_context_id))
    }

    /// Binds a route to an immutable context identity that may be shared by
    /// multiple internal services without allocating another string copy.
    pub fn bind_shared_unique_context(
        &self,
        client: Arc<dyn CdpClient>,
        unique_context_id: Arc<str>,
    ) -> CdpEvaluationService {
        CdpEvaluationService {
            client,
            execution_context_id: None,
            unique_context_id: Some(unique_context_id),
            limits: self.limits,
            resources: Some(self.resources.clone()),
        }
    }

    /// Binds both Chromium execution-context identities. The numeric ID is
    /// required by DOM adoption while the stable unique ID targets Runtime
    /// calls and detects world replacement.
    pub fn bind_execution_context(
        &self,
        client: Arc<dyn CdpClient>,
        execution_context_id: i64,
        mut unique_context_id: String,
    ) -> CdpEvaluationService {
        unique_context_id.shrink_to_fit();
        self.bind_shared_execution_context(
            client,
            execution_context_id,
            Arc::from(unique_context_id),
        )
    }

    /// Binds both execution-context identities while sharing the caller's
    /// already-retained stable identity allocation.
    pub fn bind_shared_execution_context(
        &self,
        client: Arc<dyn CdpClient>,
        execution_context_id: i64,
        unique_context_id: Arc<str>,
    ) -> CdpEvaluationService {
        CdpEvaluationService {
            client,
            execution_context_id: Some(execution_context_id),
            unique_context_id: Some(unique_context_id),
            limits: self.limits,
            resources: Some(self.resources.clone()),
        }
    }

    pub const fn profile(&self) -> CdpEvaluationProductionProfile {
        self.resources.profile
    }

    pub fn reserve_context_routes(&self) -> Result<Option<ResourceLease>, ResourceError> {
        let bytes = self.resources.profile.maximum_route_retained_bytes();
        if bytes == 0 {
            return Ok(None);
        }
        self.resources
            .ledger
            .reserve(
                self.resources.profile.resource_scope(),
                ResourceClass::BackendStateBytes,
                bytes,
            )
            .map(Some)
    }
}

impl CdpEvaluationService {
    pub fn new(client: Arc<dyn CdpClient>) -> Self {
        Self::with_limits(client, CdpEvaluationLimits::default())
    }

    pub const fn with_limits(client: Arc<dyn CdpClient>, limits: CdpEvaluationLimits) -> Self {
        Self {
            client,
            execution_context_id: None,
            unique_context_id: None,
            limits,
            resources: None,
        }
    }

    pub fn with_limits_in_unique_context(
        client: Arc<dyn CdpClient>,
        unique_context_id: impl Into<String>,
        limits: CdpEvaluationLimits,
    ) -> Self {
        Self {
            client,
            execution_context_id: None,
            unique_context_id: Some(Arc::from(unique_context_id.into().into_boxed_str())),
            limits,
            resources: None,
        }
    }

    pub fn with_limits_in_execution_context(
        client: Arc<dyn CdpClient>,
        execution_context_id: i64,
        unique_context_id: impl Into<String>,
        limits: CdpEvaluationLimits,
    ) -> Self {
        Self {
            client,
            execution_context_id: Some(execution_context_id),
            unique_context_id: Some(Arc::from(unique_context_id.into().into_boxed_str())),
            limits,
            resources: None,
        }
    }

    pub fn new_with_production_resources(
        client: Arc<dyn CdpClient>,
        profile: CdpEvaluationProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ProductionResourceError> {
        CdpEvaluationServiceFactory::new_with_production_resources(profile, resources)
            .map(|factory| factory.bind(client))
    }

    pub const fn limits(&self) -> CdpEvaluationLimits {
        self.limits
    }

    pub fn unique_context_id(&self) -> Option<&str> {
        self.unique_context_id.as_deref()
    }

    #[doc(hidden)]
    pub fn shared_unique_context_id(&self) -> Option<Arc<str>> {
        self.unique_context_id.as_ref().map(Arc::clone)
    }

    pub const fn execution_context_id(&self) -> Option<i64> {
        self.execution_context_id
    }

    pub fn retained_unique_context_bytes(&self) -> usize {
        self.unique_context_id
            .as_ref()
            .map_or(0, |value| value.len())
    }

    fn reserve(
        &self,
        params: &EvaluateParams,
    ) -> Result<Option<EvaluationLease>, OperationError<EvaluationErrorDetails>> {
        if params.source().len() > self.limits.source_bytes {
            return Err(input_limit_error(
                format!(
                    "evaluation source has {} bytes, limit is {}",
                    params.source().len(),
                    self.limits.source_bytes
                ),
                self.resources.is_some(),
            ));
        }
        if let Some(argument) = params.argument() {
            argument
                .validate(self.limits.input_value_limits())
                .map_err(|error| {
                    input_limit_error(
                        format!("invalid evaluation argument: {error}"),
                        self.resources.is_some(),
                    )
                })?;
        }
        let Some(resources) = &self.resources else {
            return Ok(None);
        };
        let limit = resources.profile.max_concurrent_operations();
        if resources
            .active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                (current < limit).then_some(current + 1)
            })
            .is_err()
        {
            return Err(resource_error(
                "evaluation operation concurrency limit was reached",
            ));
        }
        let resource = match resources.ledger.reserve(
            resources.profile.resource_scope(),
            ResourceClass::BackendStateBytes,
            resources.profile.maximum_retained_bytes_per_operation(),
        ) {
            Ok(resource) => resource,
            Err(error) => {
                let previous = resources.active.fetch_sub(1, Ordering::AcqRel);
                debug_assert!(previous > 0);
                return Err(map_resource_error(error));
            }
        };
        Ok(Some(EvaluationLease {
            _resource: Some(resource),
            active: resources.active.clone(),
        }))
    }

    #[doc(hidden)]
    pub fn reserve_for_handle(
        &self,
        params: &EvaluateParams,
    ) -> Result<Option<EvaluationLease>, OperationError<EvaluationErrorDetails>> {
        self.reserve(params)
    }

    #[doc(hidden)]
    pub fn shared_client(&self) -> Arc<dyn CdpClient> {
        Arc::clone(&self.client)
    }

    /// Returns the exact target-local CDP client bound to this evaluation route.
    pub fn client(&self) -> Arc<dyn CdpClient> {
        Arc::clone(&self.client)
    }

    fn reserve_chunked_string(
        &self,
    ) -> Result<Option<ChunkedStringLease>, OperationError<EvaluationErrorDetails>> {
        let Some(resources) = &self.resources else {
            return Ok(None);
        };
        if resources
            .large_active
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(resource_error(
                "chunked evaluation string capacity was reached",
            ));
        }
        let bytes = resources.profile.maximum_chunked_string_retained_bytes();
        let resource = match resources.ledger.reserve(
            resources.profile.resource_scope(),
            ResourceClass::BackendStateBytes,
            bytes,
        ) {
            Ok(resource) => resource,
            Err(error) => {
                resources.large_active.store(0, Ordering::Release);
                return Err(map_resource_error(error));
            }
        };
        Ok(Some(ChunkedStringLease {
            _resource: Some(resource),
            active: resources.large_active.clone(),
        }))
    }

    fn next_chunked_string_group(&self) -> Result<String, OperationError<EvaluationErrorDetails>> {
        let Some(resources) = &self.resources else {
            return Err(resource_error(
                "chunked evaluation strings require production resources",
            ));
        };
        let id = resources
            .next_group
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |id| id.checked_add(1))
            .map_err(|_| resource_error("chunked evaluation object-group IDs exhausted"))?;
        Ok(format!("pwright-evaluation-{id:016x}"))
    }

    async fn call_with_chunked_string_results(
        &self,
        script: String,
    ) -> Result<EvaluateResult, OperationError<EvaluationErrorDetails>> {
        const PLAN: &str = "function(){const value=this&&this.ok===true&&this.value;if(value&&value.t==='chunked-str')return{__pwrightChunkedString:true,utf16:value.v.length,bytes:value.b};return null;}";
        const READ: &str = "function(offset,size){const value=this.value.v;let end=Math.min(value.length,offset+size);if(end<value.length){const high=value.charCodeAt(end-1);const low=value.charCodeAt(end);if(high>=55296&&high<=56319&&low>=56320&&low<=57343)end+=1;}return{text:value.slice(offset,end),next:end};}";

        let unique_context_id = self.unique_context_id.as_deref().ok_or_else(|| {
            invalid_response("chunked evaluation requires an exact execution context")
        })?;
        let group = self.next_chunked_string_group()?;
        let declaration = format!("function(){{return {script};}}");
        if declaration.len() > self.limits.generated_script_bytes {
            return Err(input_limit_error(
                "chunked evaluation function exceeds generated-script limit".into(),
                self.resources.is_some(),
            ));
        }
        let initial = self
            .client
            .runtime_call_function_as_object_awaited_in_group_to_settlement(
                unique_context_id,
                &declaration,
                Vec::new(),
                &group,
                self.resources
                    .as_ref()
                    .is_some_and(|resources| resources.profile.user_gesture),
            )
            .await;
        // CDP only creates a retained group owner when its reply publishes a
        // RemoteObject ID. A plain inline primitive without exceptionDetails
        // has nothing to release. Returning before another CDP command lets
        // a timer-opened dialog publish after evaluation's initial reply.
        let inline_without_retained_object = initial
            .as_ref()
            .is_ok_and(inline_primitive_without_remote_object_id);
        let result = match initial {
            Err(error) => Err(map_cdp_error(error, self.limits.error_bytes)),
            Ok(initial) => self.read_chunked_string_result(initial, PLAN, READ).await,
        };
        if inline_without_retained_object {
            return result;
        }
        let cleanup = self
            .client
            .runtime_release_object_group_to_settlement(&group)
            .await
            .map_err(|error| map_cdp_error(error, self.limits.error_bytes));
        match (result, cleanup) {
            (Ok(value), Ok(())) => Ok(value),
            (Ok(_), Err(cleanup)) => Err(cleanup),
            (Err(primary), _) => Err(primary),
        }
    }

    async fn read_chunked_string_result(
        &self,
        initial: Value,
        plan_function: &str,
        read_function: &str,
    ) -> Result<EvaluateResult, OperationError<EvaluationErrorDetails>> {
        if initial
            .get("result")
            .and_then(|value| value.get("value"))
            .and_then(Value::as_str)
            .is_some()
        {
            return parse_response(initial, self.limits);
        }
        let object_id = initial
            .get("result")
            .and_then(|value| value.get("objectId"))
            .and_then(Value::as_str)
            .ok_or_else(|| invalid_response("chunked evaluation result has no objectId"))?;
        let plan = self
            .client
            .runtime_call_function_on(object_id, plan_function, Vec::new())
            .await
            .map_err(|error| map_cdp_error(error, self.limits.error_bytes))?;
        let value = plan
            .get("result")
            .and_then(|value| value.get("value"))
            .ok_or_else(|| invalid_response("chunked evaluation plan has no value"))?;
        let Some(marker) = value.as_object().filter(|value| {
            value.get("__pwrightChunkedString").and_then(Value::as_bool) == Some(true)
        }) else {
            return Err(invalid_response(
                "chunked evaluation retained result is not a large-string marker",
            ));
        };
        let utf16 = marker
            .get("utf16")
            .and_then(Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| invalid_response("chunked evaluation UTF-16 length is invalid"))?;
        let bytes = marker
            .get("bytes")
            .and_then(Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| invalid_response("chunked evaluation byte length is invalid"))?;
        if bytes <= self.limits.inline_string_bytes()
            || bytes > self.limits.value.max_string_bytes()
        {
            return Err(invalid_response(
                "chunked evaluation byte length exceeds its selected bounds",
            ));
        }
        let _lease = self.reserve_chunked_string()?;
        let mut output = String::new();
        output
            .try_reserve_exact(bytes)
            .map_err(|_| resource_error("chunked evaluation string allocation failed"))?;
        let mut offset = 0usize;
        while offset < utf16 {
            let response = self
                .client
                .runtime_call_function_on(
                    object_id,
                    read_function,
                    vec![
                        serde_json::json!({"value": offset}),
                        serde_json::json!({"value": self.limits.chunk_utf16_units()}),
                    ],
                )
                .await
                .map_err(|error| map_cdp_error(error, self.limits.error_bytes))?;
            let chunk = response
                .get("result")
                .and_then(|value| value.get("value"))
                .and_then(Value::as_object)
                .ok_or_else(|| invalid_response("chunked evaluation read has no object value"))?;
            let next = chunk
                .get("next")
                .and_then(Value::as_u64)
                .and_then(|value| usize::try_from(value).ok())
                .ok_or_else(|| invalid_response("chunked evaluation next offset is invalid"))?;
            let text = chunk
                .get("text")
                .and_then(Value::as_str)
                .ok_or_else(|| invalid_response("chunked evaluation read has no text"))?;
            if next <= offset || next > utf16 || output.len().saturating_add(text.len()) > bytes {
                return Err(invalid_response(
                    "chunked evaluation reader made invalid progress",
                ));
            }
            output.push_str(text);
            offset = next;
        }
        if output.len() != bytes {
            return Err(invalid_response(
                "chunked evaluation result byte length changed during transfer",
            ));
        }
        let value = EvaluationValue::String(output);
        value
            .validate(self.limits.value)
            .map_err(|error| invalid_response(&format!("invalid evaluation value: {error}")))?;
        Ok(EvaluateResult::new(value))
    }
}

fn inline_primitive_without_remote_object_id(response: &Value) -> bool {
    if response.get("exceptionDetails").is_some() {
        return false;
    }
    let Some(result) = response.get("result").and_then(Value::as_object) else {
        return false;
    };
    if result.get("type").and_then(Value::as_str) != Some("string")
        || result.get("value").and_then(Value::as_str).is_none()
    {
        return false;
    }
    // Scan the whole bounded reply shape so a future retained subresponse
    // cannot accidentally bypass awaited cleanup.
    let mut pending = vec![response];
    while let Some(value) = pending.pop() {
        match value {
            Value::Object(fields) => {
                if fields.contains_key("objectId") {
                    return false;
                }
                pending.extend(fields.values());
            }
            Value::Array(items) => pending.extend(items),
            _ => {}
        }
    }
    true
}

impl NativeBackendAccess<super::ChromiumCdpEvaluationBackend> for CdpEvaluationService {
    type Native = dyn CdpClient;

    fn native_backend(&self) -> &Self::Native {
        self.client.as_ref()
    }
}

impl Service<EvaluateParams> for CdpEvaluationService {
    type Response = EvaluateResult;
    type Error = OperationError<EvaluationErrorDetails>;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, params: EvaluateParams) -> Self::Future<'_> {
        let lease = match self.reserve(&params) {
            Ok(lease) => lease,
            Err(error) => return Box::pin(async move { Err(error) }),
        };
        let script = build_script(&params, self.limits, self.resources.is_some());
        drop(params);
        Box::pin(async move {
            let _lease = lease;
            let script = script?;
            if self.limits.chunked_string_results() {
                return self.call_with_chunked_string_results(script).await;
            }
            let response = if self
                .resources
                .as_ref()
                .is_some_and(|resources| resources.profile.user_gesture)
            {
                self.client
                    .runtime_evaluate_with_user_gesture(&script, self.unique_context_id.as_deref())
                    .await
            } else if let Some(unique_context_id) = &self.unique_context_id {
                self.client
                    .runtime_evaluate_in_unique_context(&script, unique_context_id.as_ref())
                    .await
            } else {
                self.client.runtime_evaluate(&script).await
            }
            .map_err(|error| map_cdp_error(error, self.limits.error_bytes))?;
            parse_response(response, self.limits)
        })
    }
}

fn build_script(
    params: &EvaluateParams,
    limits: CdpEvaluationLimits,
    production: bool,
) -> Result<String, OperationError<EvaluationErrorDetails>> {
    let source = serde_json::to_string(params.source()).map_err(json_build_error)?;
    let argument = params.argument().map(value_to_json);
    let argument = argument
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(json_build_error)?;
    let mode = match params.mode() {
        EvaluationMode::Expression => "expression",
        EvaluationMode::Function => "function",
    };
    let mut script = String::with_capacity(
        EVALUATION_RUNTIME
            .len()
            .saturating_add(source.len())
            .saturating_add(argument.as_ref().map_or(0, String::len))
            .saturating_add(256),
    );
    script.push('(');
    script.push_str(EVALUATION_RUNTIME);
    script.push_str(")({source:");
    script.push_str(&source);
    script.push_str(",mode:\"");
    script.push_str(mode);
    script.push_str("\",hasArgument:");
    script.push_str(if argument.is_some() { "true" } else { "false" });
    script.push_str(",argument:");
    script.push_str(argument.as_deref().unwrap_or("null"));
    script.push_str(",receiverSlot:null");
    script.push_str(",errorBytes:");
    script.push_str(&limits.error_bytes.to_string());
    script.push_str(",maxDepth:");
    script.push_str(&limits.value.max_depth().to_string());
    script.push_str(",maxNodes:");
    script.push_str(&limits.value.max_nodes().to_string());
    script.push_str(",maxEntries:");
    script.push_str(&limits.value.max_entries().to_string());
    script.push_str(",maxStringBytes:");
    script.push_str(&limits.value.max_string_bytes().to_string());
    script.push_str(",maxInlineStringBytes:");
    script.push_str(&limits.inline_string_bytes().to_string());
    script.push_str(",chunkUtf16Units:");
    script.push_str(&limits.chunk_utf16_units().to_string());
    script.push_str(",maxBinaryBytes:");
    script.push_str(&limits.value.max_binary_bytes().to_string());
    script.push_str("})");
    if script.len() > limits.generated_script_bytes {
        return Err(input_limit_error(
            format!(
                "generated evaluation script has {} bytes, limit is {}",
                script.len(),
                limits.generated_script_bytes
            ),
            production,
        ));
    }
    Ok(script)
}

pub(crate) fn value_to_json(value: &EvaluationValue) -> Value {
    let mut object = Map::new();
    match value {
        EvaluationValue::Undefined => tag(&mut object, "u"),
        EvaluationValue::Null => tag(&mut object, "null"),
        EvaluationValue::Bool(value) => {
            tag(&mut object, "bool");
            object.insert("v".into(), Value::Bool(*value));
        }
        EvaluationValue::Number(value) => {
            tag(&mut object, "num");
            object.insert("v".into(), Value::from(value.get()));
        }
        EvaluationValue::Nan => tag(&mut object, "nan"),
        EvaluationValue::Infinity => tag(&mut object, "inf"),
        EvaluationValue::NegativeInfinity => tag(&mut object, "-inf"),
        EvaluationValue::NegativeZero => tag(&mut object, "-0"),
        EvaluationValue::String(value) => {
            tag(&mut object, "str");
            string(&mut object, "v", value);
        }
        EvaluationValue::Bigint(value) => {
            tag(&mut object, "bigint");
            string(&mut object, "v", value);
        }
        EvaluationValue::Date(value) => {
            tag(&mut object, "date");
            string(&mut object, "v", value);
        }
        EvaluationValue::Url(value) => {
            tag(&mut object, "url");
            string(&mut object, "v", value);
        }
        EvaluationValue::Regexp { pattern, flags } => {
            tag(&mut object, "regexp");
            string(&mut object, "p", pattern);
            string(&mut object, "f", flags);
        }
        EvaluationValue::Error {
            name,
            message,
            stack,
        } => {
            tag(&mut object, "error");
            string(&mut object, "n", name);
            string(&mut object, "m", message);
            object.insert(
                "s".into(),
                stack
                    .as_ref()
                    .map_or(Value::Null, |value| Value::String(value.clone())),
            );
        }
        EvaluationValue::TypedArray { kind, bytes } => {
            tag(&mut object, "typed");
            string(&mut object, "k", typed_array_name(*kind));
            object.insert("b".into(), Value::String(BASE64.encode(bytes)));
        }
        EvaluationValue::Array { id, items } => {
            tag(&mut object, "array");
            id_value(&mut object, *id);
            object.insert(
                "v".into(),
                Value::Array(items.iter().map(value_to_json).collect()),
            );
        }
        EvaluationValue::Object { id, properties } => {
            tag(&mut object, "object");
            id_value(&mut object, *id);
            object.insert(
                "v".into(),
                Value::Array(
                    properties
                        .iter()
                        .map(|property| {
                            let mut encoded = Map::new();
                            string(&mut encoded, "k", &property.key);
                            encoded.insert("v".into(), value_to_json(&property.value));
                            Value::Object(encoded)
                        })
                        .collect(),
                ),
            );
        }
        EvaluationValue::Ref(id) => {
            tag(&mut object, "ref");
            object.insert("id".into(), Value::from(id.get()));
        }
    }
    Value::Object(object)
}

fn tag(object: &mut Map<String, Value>, value: &str) {
    string(object, "t", value);
}
fn string(object: &mut Map<String, Value>, key: &str, value: &str) {
    object.insert(key.into(), Value::String(value.to_owned()));
}
fn id_value(object: &mut Map<String, Value>, id: Option<NonZeroU32>) {
    object.insert(
        "id".into(),
        id.map_or(Value::Null, |id| Value::from(id.get())),
    );
}

#[doc(hidden)]
pub fn parse_response(
    response: Value,
    limits: CdpEvaluationLimits,
) -> Result<EvaluateResult, OperationError<EvaluationErrorDetails>> {
    const INLINE_PREFIX: &str = "__pwright_evaluation_json__";

    let remote = response
        .get("result")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid_response("Runtime.evaluate is missing result"))?;
    if remote.get("objectId").is_some() {
        return Err(invalid_response(
            "value evaluation returned a live CDP objectId",
        ));
    }
    let raw_envelope = remote
        .get("value")
        .ok_or_else(|| invalid_response("Runtime.evaluate is missing result.value"))?;
    let decoded_envelope;
    let envelope = if let Some(inline) = raw_envelope.as_str() {
        let inline = inline
            .strip_prefix(INLINE_PREFIX)
            .ok_or_else(|| invalid_response("evaluation inline result has an invalid prefix"))?;
        decoded_envelope = serde_json::from_str(inline).map_err(|error| {
            invalid_response(&format!("evaluation inline result is invalid: {error}"))
        })?;
        &decoded_envelope
    } else {
        raw_envelope
    };
    let object = envelope
        .as_object()
        .ok_or_else(|| invalid_response("evaluation envelope is not an object"))?;
    match required_bool(object, "ok")? {
        true => {
            let value = object
                .get("value")
                .ok_or_else(|| invalid_response("successful evaluation is missing value"))?;
            let mut budget = ParseBudget::default();
            let value = parse_value(value, limits.value, 0, &mut budget)?;
            value
                .validate(limits.value)
                .map_err(|error| invalid_response(&format!("invalid evaluation value: {error}")))?;
            Ok(EvaluateResult::new(value))
        }
        false => parse_failure(object, limits.error_bytes),
    }
}

fn parse_failure(
    object: &Map<String, Value>,
    error_bytes: usize,
) -> Result<EvaluateResult, OperationError<EvaluationErrorDetails>> {
    match required_str(object, "kind")? {
        "javascript" => {
            let error = required_object(object, "error")?;
            let name = bounded_string(error, "name", error_bytes)?;
            let message = bounded_string(error, "message", error_bytes)?;
            let stack = optional_bounded_string(error, "stack", error_bytes)?;
            Err(OperationError::new(
                FailureKind::Backend,
                format!("JavaScript {name}: {message}"),
                Some(EvaluationErrorDetails::Javascript {
                    name,
                    message,
                    stack,
                }),
            ))
        }
        "invalid-value" => {
            let message = bounded_string(object, "message", error_bytes)?;
            Err(OperationError::new(
                FailureKind::ProtocolViolation,
                message.clone(),
                Some(EvaluationErrorDetails::InvalidValue(message)),
            ))
        }
        "resource-exhausted" => {
            let message = bounded_string(object, "message", error_bytes)?;
            Err(OperationError::new(
                FailureKind::ResourceExhausted,
                message.clone(),
                Some(EvaluationErrorDetails::InvalidValue(message)),
            ))
        }
        _ => Err(invalid_response("evaluation failure has an unknown kind")),
    }
}

fn parse_value(
    value: &Value,
    limits: EvaluationValueLimits,
    depth: usize,
    budget: &mut ParseBudget,
) -> Result<EvaluationValue, OperationError<EvaluationErrorDetails>> {
    if depth > limits.max_depth() {
        return Err(invalid_response("evaluation value exceeds depth limit"));
    }
    let object = value
        .as_object()
        .ok_or_else(|| invalid_response("evaluation value is not an object"))?;
    budget.claim_node(limits)?;
    Ok(match required_str(object, "t")? {
        "u" => EvaluationValue::Undefined,
        "null" => EvaluationValue::Null,
        "bool" => EvaluationValue::Bool(required_bool(object, "v")?),
        "num" => {
            let number = object
                .get("v")
                .and_then(Value::as_f64)
                .ok_or_else(|| invalid_response("number value is missing or non-finite"))?;
            EvaluationValue::Number(
                EvaluationNumber::new(number)
                    .map_err(|_| invalid_response("number value is non-canonical"))?,
            )
        }
        "nan" => EvaluationValue::Nan,
        "inf" => EvaluationValue::Infinity,
        "-inf" => EvaluationValue::NegativeInfinity,
        "-0" => EvaluationValue::NegativeZero,
        "str" => EvaluationValue::String(budget.copy_string(object, "v", limits)?),
        "bigint" => EvaluationValue::Bigint(budget.copy_string(object, "v", limits)?),
        "date" => EvaluationValue::Date(budget.copy_string(object, "v", limits)?),
        "url" => EvaluationValue::Url(budget.copy_string(object, "v", limits)?),
        "regexp" => EvaluationValue::Regexp {
            pattern: budget.copy_string(object, "p", limits)?,
            flags: budget.copy_string(object, "f", limits)?,
        },
        "error" => EvaluationValue::Error {
            name: budget.copy_string(object, "n", limits)?,
            message: budget.copy_string(object, "m", limits)?,
            stack: budget.copy_optional_string(object, "s", limits)?,
        },
        "typed" => {
            let encoded = required_str(object, "b")?;
            let maximum_decoded = (encoded.len().saturating_add(3) / 4).saturating_mul(3);
            if maximum_decoded > limits.max_binary_bytes().saturating_add(2) {
                return Err(invalid_response(
                    "typed-array payload exceeds binary-byte limit",
                ));
            }
            let bytes = BASE64
                .decode(required_str(object, "b")?)
                .map_err(|_| invalid_response("typed-array payload is not canonical base64"))?;
            if BASE64.encode(&bytes) != encoded {
                return Err(invalid_response(
                    "typed-array payload is not canonical base64",
                ));
            }
            budget.claim_binary(bytes.len(), limits)?;
            EvaluationValue::TypedArray {
                kind: parse_typed_array(required_str(object, "k")?)?,
                bytes,
            }
        }
        "array" => {
            let values = required_array(object, "v")?;
            budget.claim_entries(values.len(), limits)?;
            EvaluationValue::Array {
                id: optional_id(object, "id")?,
                items: values
                    .iter()
                    .map(|value| parse_value(value, limits, depth + 1, budget))
                    .collect::<Result<_, _>>()?,
            }
        }
        "object" => {
            let values = required_array(object, "v")?;
            budget.claim_entries(values.len(), limits)?;
            let mut keys = BTreeSet::new();
            let properties = values
                .iter()
                .map(|property| {
                    let property = property
                        .as_object()
                        .ok_or_else(|| invalid_response("evaluation property is not an object"))?;
                    let key = budget.copy_string(property, "k", limits)?;
                    if !keys.insert(key.clone()) {
                        return Err(invalid_response(
                            "evaluation object contains a duplicate key",
                        ));
                    }
                    Ok(EvaluationProperty::new(
                        key,
                        parse_value(
                            property.get("v").ok_or_else(|| {
                                invalid_response("evaluation property is missing value")
                            })?,
                            limits,
                            depth + 1,
                            budget,
                        )?,
                    ))
                })
                .collect::<Result<_, OperationError<EvaluationErrorDetails>>>()?;
            EvaluationValue::Object {
                id: optional_id(object, "id")?,
                properties,
            }
        }
        "ref" => EvaluationValue::Ref(required_id(object, "id")?),
        _ => return Err(invalid_response("evaluation value has an unknown tag")),
    })
}

#[derive(Default)]
struct ParseBudget {
    nodes: usize,
    entries: usize,
    string_bytes: usize,
    binary_bytes: usize,
}

impl ParseBudget {
    fn claim_node(
        &mut self,
        limits: EvaluationValueLimits,
    ) -> Result<(), OperationError<EvaluationErrorDetails>> {
        self.nodes = self
            .nodes
            .checked_add(1)
            .ok_or_else(|| invalid_response("evaluation node count overflowed"))?;
        if self.nodes > limits.max_nodes() {
            return Err(invalid_response("evaluation value exceeds node limit"));
        }
        Ok(())
    }

    fn claim_entries(
        &mut self,
        additional: usize,
        limits: EvaluationValueLimits,
    ) -> Result<(), OperationError<EvaluationErrorDetails>> {
        self.entries = self
            .entries
            .checked_add(additional)
            .ok_or_else(|| invalid_response("evaluation entry count overflowed"))?;
        if self.entries > limits.max_entries() {
            return Err(invalid_response("evaluation value exceeds entry limit"));
        }
        Ok(())
    }

    fn claim_string(
        &mut self,
        value: &str,
        limits: EvaluationValueLimits,
    ) -> Result<(), OperationError<EvaluationErrorDetails>> {
        self.string_bytes = self
            .string_bytes
            .checked_add(value.len())
            .ok_or_else(|| invalid_response("evaluation string-byte count overflowed"))?;
        if self.string_bytes > limits.max_string_bytes() {
            return Err(invalid_response(
                "evaluation value exceeds string-byte limit",
            ));
        }
        Ok(())
    }

    fn claim_binary(
        &mut self,
        additional: usize,
        limits: EvaluationValueLimits,
    ) -> Result<(), OperationError<EvaluationErrorDetails>> {
        self.binary_bytes = self
            .binary_bytes
            .checked_add(additional)
            .ok_or_else(|| invalid_response("evaluation binary-byte count overflowed"))?;
        if self.binary_bytes > limits.max_binary_bytes() {
            return Err(invalid_response(
                "evaluation value exceeds binary-byte limit",
            ));
        }
        Ok(())
    }

    fn copy_string(
        &mut self,
        object: &Map<String, Value>,
        key: &str,
        limits: EvaluationValueLimits,
    ) -> Result<String, OperationError<EvaluationErrorDetails>> {
        let value = required_str(object, key)?;
        self.claim_string(value, limits)?;
        Ok(value.to_owned())
    }

    fn copy_optional_string(
        &mut self,
        object: &Map<String, Value>,
        key: &str,
        limits: EvaluationValueLimits,
    ) -> Result<Option<String>, OperationError<EvaluationErrorDetails>> {
        optional_str(object, key)?
            .map(|value| {
                self.claim_string(value, limits)?;
                Ok(value.to_owned())
            })
            .transpose()
    }
}

fn typed_array_name(kind: TypedArrayKind) -> &'static str {
    match kind {
        TypedArrayKind::Int8 => "Int8Array",
        TypedArrayKind::Uint8 => "Uint8Array",
        TypedArrayKind::Uint8Clamped => "Uint8ClampedArray",
        TypedArrayKind::Int16 => "Int16Array",
        TypedArrayKind::Uint16 => "Uint16Array",
        TypedArrayKind::Int32 => "Int32Array",
        TypedArrayKind::Uint32 => "Uint32Array",
        TypedArrayKind::Float32 => "Float32Array",
        TypedArrayKind::Float64 => "Float64Array",
        TypedArrayKind::Bigint64 => "BigInt64Array",
        TypedArrayKind::Biguint64 => "BigUint64Array",
    }
}

fn parse_typed_array(
    value: &str,
) -> Result<TypedArrayKind, OperationError<EvaluationErrorDetails>> {
    match value {
        "Int8Array" => Ok(TypedArrayKind::Int8),
        "Uint8Array" => Ok(TypedArrayKind::Uint8),
        "Uint8ClampedArray" => Ok(TypedArrayKind::Uint8Clamped),
        "Int16Array" => Ok(TypedArrayKind::Int16),
        "Uint16Array" => Ok(TypedArrayKind::Uint16),
        "Int32Array" => Ok(TypedArrayKind::Int32),
        "Uint32Array" => Ok(TypedArrayKind::Uint32),
        "Float32Array" => Ok(TypedArrayKind::Float32),
        "Float64Array" => Ok(TypedArrayKind::Float64),
        "BigInt64Array" => Ok(TypedArrayKind::Bigint64),
        "BigUint64Array" => Ok(TypedArrayKind::Biguint64),
        _ => Err(invalid_response(
            "evaluation value has an unknown typed-array kind",
        )),
    }
}

fn required_object<'a>(
    object: &'a Map<String, Value>,
    key: &str,
) -> Result<&'a Map<String, Value>, OperationError<EvaluationErrorDetails>> {
    object.get(key).and_then(Value::as_object).ok_or_else(|| {
        invalid_response(&format!(
            "evaluation envelope field {key:?} is not an object"
        ))
    })
}
fn required_array<'a>(
    object: &'a Map<String, Value>,
    key: &str,
) -> Result<&'a Vec<Value>, OperationError<EvaluationErrorDetails>> {
    object.get(key).and_then(Value::as_array).ok_or_else(|| {
        invalid_response(&format!(
            "evaluation envelope field {key:?} is not an array"
        ))
    })
}
fn required_str<'a>(
    object: &'a Map<String, Value>,
    key: &str,
) -> Result<&'a str, OperationError<EvaluationErrorDetails>> {
    object.get(key).and_then(Value::as_str).ok_or_else(|| {
        invalid_response(&format!(
            "evaluation envelope field {key:?} is not a string"
        ))
    })
}
fn required_bool(
    object: &Map<String, Value>,
    key: &str,
) -> Result<bool, OperationError<EvaluationErrorDetails>> {
    object.get(key).and_then(Value::as_bool).ok_or_else(|| {
        invalid_response(&format!(
            "evaluation envelope field {key:?} is not a boolean"
        ))
    })
}
fn optional_str<'a>(
    object: &'a Map<String, Value>,
    key: &str,
) -> Result<Option<&'a str>, OperationError<EvaluationErrorDetails>> {
    match object.get(key) {
        Some(Value::Null) | None => Ok(None),
        Some(Value::String(value)) => Ok(Some(value)),
        _ => Err(invalid_response(&format!(
            "evaluation envelope field {key:?} is not an optional string"
        ))),
    }
}
fn required_id(
    object: &Map<String, Value>,
    key: &str,
) -> Result<NonZeroU32, OperationError<EvaluationErrorDetails>> {
    let raw = object
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .and_then(NonZeroU32::new);
    raw.ok_or_else(|| {
        invalid_response(&format!(
            "evaluation envelope field {key:?} is not a nonzero u32"
        ))
    })
}
fn optional_id(
    object: &Map<String, Value>,
    key: &str,
) -> Result<Option<NonZeroU32>, OperationError<EvaluationErrorDetails>> {
    match object.get(key) {
        Some(Value::Null) | None => Ok(None),
        Some(_) => required_id(object, key).map(Some),
    }
}
fn bounded_string(
    object: &Map<String, Value>,
    key: &str,
    limit: usize,
) -> Result<String, OperationError<EvaluationErrorDetails>> {
    let value = required_str(object, key)?;
    if value.len() > limit {
        return Err(invalid_response(&format!(
            "evaluation error field {key:?} exceeds byte limit"
        )));
    }
    Ok(value.to_owned())
}
fn optional_bounded_string(
    object: &Map<String, Value>,
    key: &str,
    limit: usize,
) -> Result<Option<String>, OperationError<EvaluationErrorDetails>> {
    optional_str(object, key)?
        .map(|value| {
            if value.len() > limit {
                Err(invalid_response(&format!(
                    "evaluation error field {key:?} exceeds byte limit"
                )))
            } else {
                Ok(value.to_owned())
            }
        })
        .transpose()
}

fn map_cdp_error(error: CdpError, error_bytes: usize) -> OperationError<EvaluationErrorDetails> {
    let kind = super::classify_cdp_failure(&error);
    let message = error.to_string();
    let details = match error {
        CdpError::JsException(message) if message.len() <= error_bytes => {
            EvaluationErrorDetails::Javascript {
                name: "Error".to_owned(),
                message,
                stack: None,
            }
        }
        CdpError::JsException(_) => {
            return invalid_response("CDP JavaScript exception exceeds the configured byte limit");
        }
        _ => EvaluationErrorDetails::Backend(message.clone()),
    };
    OperationError::new(kind, message, Some(details))
}
fn json_build_error(error: serde_json::Error) -> OperationError<EvaluationErrorDetails> {
    invalid_response(&format!("could not construct evaluation script: {error}"))
}
fn invalid_response(message: &str) -> OperationError<EvaluationErrorDetails> {
    OperationError::new(
        FailureKind::ProtocolViolation,
        message,
        Some(EvaluationErrorDetails::InvalidBackendResponse(
            message.to_owned(),
        )),
    )
}
fn input_limit_error(message: String, production: bool) -> OperationError<EvaluationErrorDetails> {
    OperationError::new(
        if production {
            FailureKind::ResourceExhausted
        } else {
            FailureKind::ProtocolViolation
        },
        message.clone(),
        Some(EvaluationErrorDetails::InvalidValue(message)),
    )
}
fn resource_error(message: &str) -> OperationError<EvaluationErrorDetails> {
    OperationError::new(
        FailureKind::ResourceExhausted,
        message,
        Some(EvaluationErrorDetails::Backend(message.to_owned())),
    )
}
fn map_resource_error(error: ResourceError) -> OperationError<EvaluationErrorDetails> {
    resource_error(&error.to_string())
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::num::NonZeroU32;
    use std::sync::Arc;
    use std::task::{Context, Poll, Waker};

    use pwright_cap_evaluation::{
        EvaluateParams, EvaluationErrorDetails, EvaluationProperty, EvaluationValue,
    };
    use pwright_core::{
        FailureKind, ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceScope,
        Service,
    };
    use pwright_fake::FakeCdpClient;
    use serde_json::json;

    use super::{
        CdpEvaluationLimits, CdpEvaluationProductionProfile, CdpEvaluationService,
        CdpEvaluationServiceFactory,
    };

    fn production_profile(max_concurrent_operations: usize) -> CdpEvaluationProductionProfile {
        let limits = CdpEvaluationLimits::new(
            128,
            32 * 1024,
            128,
            pwright_cap_evaluation::EvaluationValueLimits::new(4, 16, 16, 256, 256).unwrap(),
        );
        CdpEvaluationProductionProfile::new(
            ResourceScope::extension(91, 1),
            limits,
            max_concurrent_operations,
        )
    }

    #[test]
    fn selected_large_input_adds_bounded_per_operation_retention() {
        // Arrange
        let output =
            pwright_cap_evaluation::EvaluationValueLimits::new(8, 64, 64, 256 * 1024, 1024)
                .unwrap();
        let input =
            pwright_cap_evaluation::EvaluationValueLimits::new(8, 64, 64, 3 * 1024 * 1024, 1024)
                .unwrap();
        let ordinary = CdpEvaluationLimits::new(32 * 1024, 2 * 1024 * 1024, 4096, output);

        // Act
        let selected = ordinary.with_input_string_bytes(input.max_string_bytes());

        // Assert
        assert_eq!(ordinary.input_value_limits(), output);
        assert_eq!(selected.value, output);
        assert_eq!(selected.input_value_limits(), input);
        assert_eq!(
            selected.maximum_retained_bytes_per_operation()
                - ordinary.maximum_retained_bytes_per_operation(),
            8 * (3 * 1024 * 1024 - 256 * 1024)
        );
    }

    #[tokio::test]
    async fn selected_large_input_reaches_cdp_without_raising_result_limit() {
        // Arrange
        let output =
            pwright_cap_evaluation::EvaluationValueLimits::new(8, 64, 64, 256 * 1024, 1024)
                .unwrap();
        let input =
            pwright_cap_evaluation::EvaluationValueLimits::new(8, 64, 64, 3 * 1024 * 1024, 1024)
                .unwrap();
        let limits = CdpEvaluationLimits::new(32 * 1024, 4 * 1024 * 1024, 4096, output)
            .with_input_string_bytes(input.max_string_bytes());
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_response(
            json!({"result":{"value":{"ok":true,"value":{"t":"num","v":2097152}}}}),
        );
        let service = CdpEvaluationService::with_limits(client.clone(), limits);
        let body = "0".repeat(2 * 1024 * 1024);

        // Act
        let result = service
            .call(EvaluateParams::function(
                "body => body.length",
                EvaluationValue::String(body.clone()),
            ))
            .await;

        // Assert
        assert!(result.is_ok());
        let calls = client.calls();
        assert_eq!(client.method_names(), ["Runtime.evaluate"]);
        let script = calls[0].1[0].as_str().unwrap();
        assert!(script.contains(&body));
        assert!(script.len() <= limits.generated_script_bytes);
        assert_eq!(limits.value.max_string_bytes(), 256 * 1024);
    }

    fn production_resources(profile: CdpEvaluationProductionProfile) -> ProductionResourceLedger {
        let bytes = profile
            .requirements()
            .class(ResourceClass::BackendStateBytes);
        let requirements = profile.requirements();
        let limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
            ResourceClass::BackendStateBytes,
            bytes.global,
            bytes.per_scope,
        );
        ProductionResourceLedger::new(limits, profile.requirements())
            .expect("evaluation production requirements should fit")
    }

    fn chunked_production_profile() -> CdpEvaluationProductionProfile {
        let limits = production_profile(1)
            .limits()
            .with_chunked_string_results(2, 4)
            .unwrap();
        CdpEvaluationProductionProfile::new(ResourceScope::extension(91, 2), limits, 1)
    }

    #[tokio::test]
    async fn evaluates_on_the_supplied_session_and_decodes_cycles_and_special_values() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_response(json!({"result": {"value": {
            "ok": true,
            "value": {"t":"object","id":1,"v":[
                {"k":"self","v":{"t":"ref","id":1}},
                {"k":"negativeZero","v":{"t":"-0"}},
                {"k":"bytes","v":{"t":"typed","k":"Uint16Array","b":"AQIDBA=="}}
            ]}
        }}}));
        let service = CdpEvaluationService::new(client.clone());

        // Act
        let result = service
            .call(EvaluateParams::expression("window.answer"))
            .await
            .expect("valid response");

        // Assert
        assert_eq!(client.method_names(), vec!["Runtime.evaluate"]);
        assert!(
            client.calls()[0].1[0]
                .as_str()
                .expect("script")
                .contains("window.answer")
        );
        assert_eq!(
            result.value,
            EvaluationValue::Object {
                id: NonZeroU32::new(1),
                properties: vec![
                    EvaluationProperty::new(
                        "self",
                        EvaluationValue::Ref(NonZeroU32::new(1).unwrap())
                    ),
                    EvaluationProperty::new("negativeZero", EvaluationValue::NegativeZero),
                    EvaluationProperty::new(
                        "bytes",
                        EvaluationValue::TypedArray {
                            kind: pwright_cap_evaluation::TypedArrayKind::Uint16,
                            bytes: vec![1, 2, 3, 4]
                        }
                    ),
                ],
            }
        );
    }

    #[tokio::test]
    async fn selected_user_gesture_keeps_the_exact_context_and_resource_contract() {
        // Arrange
        let baseline = production_profile(1);
        let profile = baseline.with_user_gesture(true);
        assert_eq!(profile.requirements(), baseline.requirements());
        let resources = production_resources(profile);
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_response(json!({"result":{"value":{"ok":true,"value":{"t":"null"}}}}));
        let factory =
            CdpEvaluationServiceFactory::new_with_production_resources(profile, &resources)
                .unwrap();
        let service = factory.bind_unique_context(client.clone(), "exact-world".to_owned());
        // Act
        let result = service.call(EvaluateParams::expression("null")).await;
        // Assert
        assert!(result.is_ok());
        assert_eq!(client.method_names(), ["Runtime.evaluate"]);
        assert_eq!(client.calls()[0].1[0]["userGesture"], true);
        assert_eq!(client.calls()[0].1[0]["uniqueContextId"], "exact-world");
    }

    #[tokio::test]
    async fn chunked_profile_returns_ordinary_values_in_the_initial_cdp_reply() {
        // Arrange
        let profile = chunked_production_profile();
        let resources = production_resources(profile);
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_response(json!({
            "result": {
                "type": "string",
                "value": "__pwright_evaluation_json__{\"ok\":true,\"value\":{\"t\":\"object\",\"id\":1,\"v\":[{\"k\":\"a\",\"v\":{\"t\":\"num\",\"v\":42}}]}}"
            }
        }));
        let factory =
            CdpEvaluationServiceFactory::new_with_production_resources(profile, &resources)
                .unwrap();
        let service = factory.bind_unique_context(client.clone(), "exact-world".to_owned());

        // Act
        let result = service
            .call(EvaluateParams::expression("({ a: 42 })"))
            .await
            .unwrap();

        // Assert
        assert_eq!(
            result.value,
            EvaluationValue::Object {
                id: NonZeroU32::new(1),
                properties: vec![EvaluationProperty::new(
                    "a",
                    EvaluationValue::Number(
                        pwright_cap_evaluation::EvaluationNumber::new(42.0).unwrap()
                    )
                )],
            }
        );
        assert_eq!(client.method_names(), ["Runtime.callFunctionOn"]);
        assert!(
            client.calls()[0].1[0]["functionDeclaration"]
                .as_str()
                .unwrap()
                .contains("serializeInline")
        );
    }

    #[tokio::test]
    async fn inline_primitive_finishes_when_late_dialog_blocks_group_release() {
        // Arrange: the release gate models a dialog that blocks later CDP commands.
        let profile = chunked_production_profile();
        let resources = production_resources(profile);
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_response(json!({
            "result": {
                "type": "string",
                "value": "__pwright_evaluation_json__{\"ok\":true,\"value\":{\"t\":\"u\"}}"
            }
        }));
        let gate = Arc::new(tokio::sync::Notify::new());
        client.set_release_object_group_gate(Some(gate));
        let factory =
            CdpEvaluationServiceFactory::new_with_production_resources(profile, &resources)
                .unwrap();
        let service = factory.bind_unique_context(client.clone(), "exact-world".to_owned());
        let mut operation = Box::pin(service.call(EvaluateParams::expression("undefined")));
        let mut context = Context::from_waker(Waker::noop());

        // Act
        let outcome = operation.as_mut().poll(&mut context);

        // Assert: no group-owned RemoteObject exists, so no later CDP release is needed.
        let Poll::Ready(result) = outcome else {
            panic!("inline primitive should not await a blocked group release");
        };
        assert_eq!(result.unwrap().value, EvaluationValue::Undefined);
        assert_eq!(client.method_names(), ["Runtime.callFunctionOn"]);
    }

    #[tokio::test]
    async fn inline_structured_javascript_error_is_preserved_without_group_release() {
        // Arrange
        let profile = chunked_production_profile();
        let resources = production_resources(profile);
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_response(json!({
            "result": {
                "type": "string",
                "value": "__pwright_evaluation_json__{\"ok\":false,\"kind\":\"javascript\",\"error\":{\"name\":\"TypeError\",\"message\":\"boom\",\"stack\":null}}"
            }
        }));
        let gate = Arc::new(tokio::sync::Notify::new());
        client.set_release_object_group_gate(Some(gate));
        let factory =
            CdpEvaluationServiceFactory::new_with_production_resources(profile, &resources)
                .unwrap();
        let service = factory.bind_unique_context(client.clone(), "exact-world".to_owned());
        let mut operation =
            Box::pin(service.call(EvaluateParams::expression("throw new TypeError()")));
        let mut context = Context::from_waker(Waker::noop());

        // Act
        let outcome = operation.as_mut().poll(&mut context);

        // Assert
        let Poll::Ready(result) = outcome else {
            panic!("inline structured error should not await a blocked group release");
        };
        let error = result.unwrap_err();
        assert!(matches!(
            error.details(),
            Some(EvaluationErrorDetails::Javascript { name, message, .. })
                if name == "TypeError" && message == "boom"
        ));
        assert_eq!(client.method_names(), ["Runtime.callFunctionOn"]);
    }

    #[tokio::test]
    async fn retained_result_waits_for_group_release_acknowledgement() {
        // Arrange
        let profile = chunked_production_profile();
        let resources = production_resources(profile);
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_responses([
            json!({"result":{"type":"object","objectId":"large-1"}}),
            json!({"result":{"value":{"__pwrightChunkedString":true,"utf16":3,"bytes":3}}}),
            json!({"result":{"value":{"text":"abc","next":3}}}),
        ]);
        let gate = Arc::new(tokio::sync::Notify::new());
        client.set_release_object_group_gate(Some(gate.clone()));
        let factory =
            CdpEvaluationServiceFactory::new_with_production_resources(profile, &resources)
                .unwrap();
        let service = factory.bind_unique_context(client.clone(), "exact-world".to_owned());
        let mut operation = Box::pin(service.call(EvaluateParams::expression("'abc'")));
        let mut context = Context::from_waker(Waker::noop());

        // Act
        let before_ack = operation.as_mut().poll(&mut context);
        gate.notify_one();
        let result = operation.await.unwrap();

        // Assert
        assert!(before_ack.is_pending());
        assert_eq!(result.value, EvaluationValue::String("abc".to_owned()));
        assert_eq!(
            client.method_names().last().map(String::as_str),
            Some("Runtime.releaseObjectGroup")
        );
    }

    #[tokio::test]
    async fn exception_remote_object_id_prevents_inline_cleanup_elision() {
        // Arrange
        let profile = chunked_production_profile();
        let resources = production_resources(profile);
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_response(json!({
            "result": {
                "type": "string",
                "value": "__pwright_evaluation_json__{\"ok\":true,\"value\":{\"t\":\"null\"}}"
            },
            "exceptionDetails": { "exception": { "type": "object", "objectId": "error-1" } }
        }));
        let gate = Arc::new(tokio::sync::Notify::new());
        client.set_release_object_group_gate(Some(gate.clone()));
        let factory =
            CdpEvaluationServiceFactory::new_with_production_resources(profile, &resources)
                .unwrap();
        let service = factory.bind_unique_context(client.clone(), "exact-world".to_owned());
        let mut operation = Box::pin(service.call(EvaluateParams::expression("null")));
        let mut context = Context::from_waker(Waker::noop());

        // Act
        let before_ack = operation.as_mut().poll(&mut context);
        gate.notify_one();
        let result = operation.await.unwrap();

        // Assert
        assert!(before_ack.is_pending());
        assert_eq!(result.value, EvaluationValue::Null);
        assert_eq!(
            client.method_names(),
            ["Runtime.callFunctionOn", "Runtime.releaseObjectGroup"]
        );
    }

    #[tokio::test]
    async fn id_free_cdp_exception_details_still_waits_for_group_release() {
        // Arrange: an ID-free CDP exception is not the inline JS-error envelope.
        let profile = chunked_production_profile();
        let resources = production_resources(profile);
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_response(json!({
            "result": {
                "type": "string",
                "value": "__pwright_evaluation_json__{\"ok\":true,\"value\":{\"t\":\"null\"}}"
            },
            "exceptionDetails": { "text": "uncaught", "lineNumber": 1, "columnNumber": 1 }
        }));
        let gate = Arc::new(tokio::sync::Notify::new());
        client.set_release_object_group_gate(Some(gate.clone()));
        let factory =
            CdpEvaluationServiceFactory::new_with_production_resources(profile, &resources)
                .unwrap();
        let service = factory.bind_unique_context(client.clone(), "exact-world".to_owned());
        let mut operation = Box::pin(service.call(EvaluateParams::expression("null")));
        let mut context = Context::from_waker(Waker::noop());

        // Act
        let before_ack = operation.as_mut().poll(&mut context);
        gate.notify_one();
        let result = operation.await.unwrap();

        // Assert
        assert!(before_ack.is_pending());
        assert_eq!(result.value, EvaluationValue::Null);
        assert_eq!(
            client.method_names(),
            ["Runtime.callFunctionOn", "Runtime.releaseObjectGroup"]
        );
    }

    #[tokio::test]
    async fn cancelling_retained_result_keeps_existing_no_detached_cleanup_gate() {
        // Arrange
        let profile = chunked_production_profile();
        let resources = production_resources(profile);
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_responses([
            json!({"result":{"type":"object","objectId":"large-1"}}),
            json!({"result":{"value":{"__pwrightChunkedString":true,"utf16":3,"bytes":3}}}),
            json!({"result":{"value":{"text":"abc","next":3}}}),
        ]);
        let gate = Arc::new(tokio::sync::Notify::new());
        client.set_release_object_group_gate(Some(gate.clone()));
        let factory =
            CdpEvaluationServiceFactory::new_with_production_resources(profile, &resources)
                .unwrap();
        let service = factory.bind_unique_context(client.clone(), "exact-world".to_owned());
        let baseline_leases = resources.metrics().unwrap().active_leases;
        let mut operation = Box::pin(service.call(EvaluateParams::expression("'abc'")));
        let mut context = Context::from_waker(Waker::noop());
        assert!(operation.as_mut().poll(&mut context).is_pending());
        let calls_before_cancel = client.method_names();

        // Act
        drop(operation);
        gate.notify_one();
        tokio::task::yield_now().await;

        // Assert: this is a regression gate against introducing fire-and-forget,
        // not a fix for the broader remote-handle cancellation leak question.
        assert_eq!(resources.metrics().unwrap().active_leases, baseline_leases);
        assert_eq!(client.method_names(), calls_before_cancel);
    }

    #[tokio::test]
    async fn chunked_profile_reads_only_a_selected_large_top_level_string() {
        // Arrange
        let profile = chunked_production_profile();
        let resources = production_resources(profile);
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_responses([
            json!({"result":{"type":"object","objectId":"large-1"}}),
            json!({"result":{"value":{"__pwrightChunkedString":true,"utf16":3,"bytes":3}}}),
            json!({"result":{"value":{"text":"abc","next":3}}}),
        ]);
        let factory =
            CdpEvaluationServiceFactory::new_with_production_resources(profile, &resources)
                .unwrap();
        let service = factory.bind_unique_context(client.clone(), "exact-world".to_owned());

        // Act
        let result = service
            .call(EvaluateParams::expression("'abc'"))
            .await
            .unwrap();

        // Assert
        assert_eq!(result.value, EvaluationValue::String("abc".to_owned()));
        assert_eq!(
            client.method_names(),
            [
                "Runtime.callFunctionOn",
                "Runtime.callFunctionOn",
                "Runtime.callFunctionOn",
                "Runtime.releaseObjectGroup",
            ]
        );
    }

    #[tokio::test]
    async fn production_service_targets_one_unique_execution_context() {
        // Arrange
        let profile = production_profile(1).with_context_routes(1, 64);
        let requirements = profile.requirements();
        let bytes = requirements.class(ResourceClass::BackendStateBytes);
        let resources = ProductionResourceLedger::new(
            ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
                ResourceClass::BackendStateBytes,
                bytes.global,
                bytes.per_scope,
            ),
            requirements,
        )
        .expect("context-routed evaluation resources should fit");
        let factory =
            CdpEvaluationServiceFactory::new_with_production_resources(profile, &resources)
                .expect("factory should claim its complete profile");
        let _routes = factory
            .reserve_context_routes()
            .expect("route storage should reserve")
            .expect("one configured route needs a lease");
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_response(
            json!({"result":{"value":{"ok":true,"value":{"t":"str","v":"child"}}}}),
        );
        let service = factory.bind_execution_context(client.clone(), 71, "unique-child".to_owned());

        // Act
        let result = service
            .call(EvaluateParams::expression("document.title"))
            .await
            .expect("context-targeted evaluation should decode");

        // Assert
        assert_eq!(result.value, EvaluationValue::String("child".to_owned()));
        assert_eq!(service.execution_context_id(), Some(71));
        assert_eq!(service.unique_context_id(), Some("unique-child"));
        assert_eq!(client.calls()[0].1[0]["uniqueContextId"], "unique-child");
    }

    #[tokio::test]
    async fn omitted_function_argument_is_passed_as_javascript_undefined() {
        // Arrange
        let argument_client = Arc::new(FakeCdpClient::from_html(""));
        argument_client
            .set_evaluate_response(json!({"result":{"value":{"ok":true,"value":{"t":"u"}}}}));
        let no_argument_client = Arc::new(FakeCdpClient::from_html(""));
        no_argument_client
            .set_evaluate_response(json!({"result":{"value":{"ok":true,"value":{"t":"u"}}}}));

        // Act
        CdpEvaluationService::new(argument_client.clone())
            .call(EvaluateParams::function("x => x", EvaluationValue::Null))
            .await
            .unwrap();
        CdpEvaluationService::new(no_argument_client.clone())
            .call(EvaluateParams::function_without_argument("() => 1"))
            .await
            .unwrap();

        // Assert
        assert!(
            argument_client.calls()[0].1[0]
                .as_str()
                .unwrap()
                .contains("hasArgument:true")
        );
        assert!(
            no_argument_client.calls()[0].1[0]
                .as_str()
                .unwrap()
                .contains("hasArgument:false")
        );
        assert!(
            no_argument_client.calls()[0].1[0]
                .as_str()
                .unwrap()
                .contains("return fn(argument)")
        );
        assert!(
            no_argument_client.calls()[0].1[0]
                .as_str()
                .unwrap()
                .contains("const result = evaluator.evaluate()")
        );
    }

    #[tokio::test]
    async fn generated_runtime_classifies_browser_references_before_property_traversal() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_response(
            json!({"result":{"value":{"ok":true,"value":{"t":"str","v":"ref: <Window>"}}}}),
        );
        let service = CdpEvaluationService::new(client.clone());

        // Act
        let result = service
            .call(EvaluateParams::function_without_argument(
                "() => window.open('about:blank')",
            ))
            .await
            .expect("browser reference should use the value serializer");

        // Assert
        assert_eq!(
            result.value,
            EvaluationValue::String("ref: <Window>".to_owned())
        );
        let calls = client.calls();
        let script = calls[0].1[0]
            .as_str()
            .expect("Runtime.evaluate should receive a script");
        let browser_reference = script
            .find("value instanceof globalThis.Window")
            .expect("Window references should be classified");
        let property_traversal = script
            .find("for (const key in value)")
            .expect("ordinary objects should still be traversed");
        assert!(browser_reference < property_traversal);
        assert!(script.contains("ref: <Document>"));
        assert!(script.contains("ref: <Node>"));
        assert!(script.contains("item = value[key]"));
        assert!(script.contains("catch (_)"));
    }

    #[tokio::test]
    async fn maps_structured_javascript_failures_without_a_handle() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_response(json!({"result":{"value":{"ok":false,"kind":"javascript","error":{"name":"TypeError","message":"boom","stack":"stack"}}}}));
        let service = CdpEvaluationService::new(client);

        // Act
        let error = service
            .call(EvaluateParams::expression("fail()"))
            .await
            .unwrap_err();

        // Assert
        assert_eq!(error.kind(), FailureKind::Backend);
        assert!(
            matches!(error.details(), Some(EvaluationErrorDetails::Javascript { name, message, .. }) if name == "TypeError" && message == "boom")
        );
    }

    #[tokio::test]
    async fn rejects_source_before_calling_cdp() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let defaults = CdpEvaluationLimits::default();
        let limits = CdpEvaluationLimits::new(
            3,
            defaults.generated_script_bytes,
            defaults.error_bytes,
            defaults.value,
        );
        let service = CdpEvaluationService::with_limits(client.clone(), limits);

        // Act
        let error = service
            .call(EvaluateParams::expression("1234"))
            .await
            .unwrap_err();

        // Assert
        assert_eq!(error.kind(), FailureKind::ProtocolViolation);
        assert!(client.method_names().is_empty());
    }

    #[tokio::test]
    async fn rejects_live_object_ids_and_unknown_references() {
        // Arrange
        let object_client = Arc::new(FakeCdpClient::from_html(""));
        object_client.set_evaluate_response(
            json!({"result":{"objectId":"remote-1","value":{"ok":true,"value":{"t":"null"}}}}),
        );
        let reference_client = Arc::new(FakeCdpClient::from_html(""));
        reference_client.set_evaluate_response(
            json!({"result":{"value":{"ok":true,"value":{"t":"ref","id":7}}}}),
        );

        // Act
        let object_error = CdpEvaluationService::new(object_client)
            .call(EvaluateParams::expression("document.body"))
            .await
            .unwrap_err();
        let reference_error = CdpEvaluationService::new(reference_client)
            .call(EvaluateParams::expression("({})"))
            .await
            .unwrap_err();

        // Assert
        assert_eq!(object_error.kind(), FailureKind::ProtocolViolation);
        assert_eq!(reference_error.kind(), FailureKind::ProtocolViolation);
    }

    #[tokio::test]
    async fn unpolled_production_call_owns_capacity_and_drop_releases_it() {
        // Arrange
        let profile = production_profile(1);
        let resources = production_resources(profile);
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_response(json!({"result":{"value":{"ok":true,"value":{"t":"null"}}}}));
        let service = CdpEvaluationService::new_with_production_resources(
            client.clone(),
            profile,
            &resources,
        )
        .expect("declared evaluation should construct");

        // Act
        let first = service.call(EvaluateParams::expression("1"));
        let rejected = service
            .call(EvaluateParams::expression("2"))
            .await
            .expect_err("second call should fail at synchronous admission");
        drop(first);
        let completed = service
            .call(EvaluateParams::expression("3"))
            .await
            .expect("dropping the first reservation should free capacity");

        // Assert
        assert_eq!(rejected.kind(), FailureKind::ResourceExhausted);
        assert_eq!(completed.value, EvaluationValue::Null);
        assert_eq!(client.method_names(), vec!["Runtime.evaluate"]);
    }

    #[test]
    fn production_factory_requires_its_exact_resource_declaration() {
        // Arrange
        let profile = production_profile(1);
        let undeclared = ProductionResourceLedger::new(
            ResourceLimits::deny_all(1, 1),
            pwright_core::ResourceRequirements::none(),
        )
        .expect("empty production ledger should build");

        // Act
        let result =
            CdpEvaluationServiceFactory::new_with_production_resources(profile, &undeclared);

        // Assert
        assert!(result.is_err());
    }
}
