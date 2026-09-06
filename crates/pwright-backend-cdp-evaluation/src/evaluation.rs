//! Bounded, value-only JavaScript evaluation over one target-local CDP session.

use std::collections::BTreeSet;
use std::future::Future;
use std::num::NonZeroU32;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

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
pub const EVALUATION_RUNTIME: &str = r#"async function(c) {
  'use strict';
  const handles = Array.prototype.slice.call(arguments, 1);
  const hasOwn = Object.prototype.hasOwnProperty;
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
    if (state.strings > c.maxStringBytes) throwLimit('evaluation string-byte limit exceeded');
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
      claimString(value);
      return { t: 'str', v: value };
    }
    if (typeof value === 'bigint') {
      const text = value.toString();
      claimString(text);
      return { t: 'bigint', v: text };
    }
    if (typeof value === 'symbol' || typeof value === 'function') return { t: 'u' };
    if (value instanceof Date) {
      const text = value.toISOString();
      claimString(text);
      return { t: 'date', v: text };
    }
    if (typeof URL !== 'undefined' && value instanceof URL) {
      const text = value.href;
      claimString(text);
      return { t: 'url', v: text };
    }
    if (value instanceof RegExp) {
      claimString(value.source);
      claimString(value.flags);
      return { t: 'regexp', p: value.source, f: value.flags };
    }
    if (value instanceof Error) {
      const record = errorRecord(value);
      claimString(record.name);
      claimString(record.message);
      if (record.stack !== null) claimString(record.stack);
      return { t: 'error', n: record.name, m: record.message, s: record.stack };
    }
    if (ArrayBuffer.isView(value) && !(value instanceof DataView)) {
      const kind = value.constructor && value.constructor.name;
      if (!hasOwn.call(typedConstructors, kind)) throw new TypeError('unsupported typed array kind');
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
      if (!hasOwn.call(value, key)) continue;
      claimEntry();
      claimString(key);
      properties.push({ k: key, v: encode(value[key], depth + 1) });
    }
    return { t: 'object', id, v: properties };
  };
  let argument;
  if (c.hasArgument) {
    try {
      argument = decode(c.argument);
    } catch (error) {
      return { ok: false, kind: 'invalid-value', message: clipped(error && error.message !== undefined ? error.message : error) };
    }
  }
  let result;
  try {
    if (c.mode === 'expression') result = await eval(c.source);
    else {
      const fn = eval('(' + c.source + ')');
      if (typeof fn !== 'function') throw new TypeError('evaluation source is not a function');
      if (c.receiverSlot === null) result = await fn(argument);
      else {
        if (!Number.isInteger(c.receiverSlot) || c.receiverSlot < 0 || c.receiverSlot >= handles.length)
          throw new TypeError('unknown Handle receiver slot');
        result = await fn(handles[c.receiverSlot], argument);
      }
    }
  } catch (error) {
    return { ok: false, kind: 'javascript', error: errorRecord(error) };
  }
  try {
    return { ok: true, value: encode(result, 0) };
  } catch (error) {
    return { ok: false, kind: error && error.__pwrightLimit ? 'resource-exhausted' : 'invalid-value', message: clipped(error && error.message !== undefined ? error.message : error) };
  }
}"#;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CdpEvaluationLimits {
    pub source_bytes: usize,
    pub generated_script_bytes: usize,
    pub error_bytes: usize,
    pub value: EvaluationValueLimits,
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
        }
    }

    /// Conservative host-side peak for input, generated script, CDP JSON,
    /// base64 staging, and the final portable value.
    pub fn maximum_retained_bytes_per_operation(self) -> u64 {
        let source = u64::try_from(self.source_bytes).unwrap_or(u64::MAX);
        let script = u64::try_from(self.generated_script_bytes).unwrap_or(u64::MAX);
        let error = u64::try_from(self.error_bytes).unwrap_or(u64::MAX);
        let strings = u64::try_from(self.value.max_string_bytes()).unwrap_or(u64::MAX);
        let binary = u64::try_from(self.value.max_binary_bytes()).unwrap_or(u64::MAX);
        let entries = u64::try_from(self.value.max_entries()).unwrap_or(u64::MAX);
        source
            .saturating_mul(2)
            .saturating_add(script.saturating_mul(2))
            .saturating_add(error.saturating_mul(4))
            .saturating_add(strings.saturating_mul(8))
            .saturating_add(binary.saturating_mul(6))
            .saturating_add(entries.saturating_mul(96))
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
    scope: ResourceScope,
    limits: CdpEvaluationLimits,
    max_concurrent_operations: usize,
    max_context_routes: usize,
    max_native_id_bytes: usize,
    maximum_retained_bytes_per_operation: u64,
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
        let operations = u64::try_from(max_concurrent_operations).unwrap_or(u64::MAX);
        let bytes = maximum_retained_bytes_per_operation.saturating_mul(operations);
        let requirements = if max_concurrent_operations == 0 {
            ResourceRequirements::none()
        } else {
            ResourceRequirements::none()
                .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
                .with_metadata(1, max_concurrent_operations)
        };
        Self {
            scope,
            limits,
            max_concurrent_operations,
            max_context_routes: 0,
            max_native_id_bytes: 0,
            maximum_retained_bytes_per_operation,
            maximum_route_retained_bytes: 0,
            requirements,
        }
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
            .saturating_add(self.maximum_route_retained_bytes);
        self.requirements = if bytes == 0 {
            ResourceRequirements::none()
        } else {
            ResourceRequirements::none()
                .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
                .with_metadata(
                    1,
                    self.max_concurrent_operations
                        .saturating_add(usize::from(self.maximum_route_retained_bytes > 0)),
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
}

#[doc(hidden)]
pub struct EvaluationLease {
    _resource: Option<ResourceLease>,
    active: Arc<AtomicUsize>,
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
            argument.validate(self.limits.value).map_err(|error| {
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
            let response = if let Some(unique_context_id) = &self.unique_context_id {
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
    let remote = response
        .get("result")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid_response("Runtime.evaluate is missing result"))?;
    if remote.get("objectId").is_some() {
        return Err(invalid_response(
            "value evaluation returned a live CDP objectId",
        ));
    }
    let envelope = remote
        .get("value")
        .ok_or_else(|| invalid_response("Runtime.evaluate is missing result.value"))?;
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
    use std::num::NonZeroU32;
    use std::sync::Arc;

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

    fn production_resources(profile: CdpEvaluationProductionProfile) -> ProductionResourceLedger {
        let bytes = profile
            .requirements()
            .class(ResourceClass::BackendStateBytes);
        let limits = ResourceLimits::deny_all(1, profile.max_concurrent_operations()).with_class(
            ResourceClass::BackendStateBytes,
            bytes.global,
            bytes.per_scope,
        );
        ProductionResourceLedger::new(limits, profile.requirements())
            .expect("evaluation production requirements should fit")
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
                .contains("result = await fn(argument)")
        );
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
