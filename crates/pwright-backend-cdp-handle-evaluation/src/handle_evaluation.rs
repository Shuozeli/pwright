//! Retained JavaScript-handle evaluation over one exact CDP execution world.

use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::num::NonZeroU32;
use std::pin::Pin;
use std::sync::Arc;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use pwright_browser_model::{ELEMENT_HANDLE_OBJECT_KIND, JS_HANDLE_OBJECT_KIND};
use pwright_cap_evaluation::{EvaluationErrorDetails, EvaluationMode};
use pwright_cap_handle_evaluation::{
    EvaluateHandleParams, HandleEvaluationErrorDetails, HandleEvaluationValue,
};
use pwright_cdp::CdpClient;
use pwright_cdp::connection::CdpError;
use pwright_core::{FailureKind, ObjectKindTag, OperationError, Service};
use serde_json::{Map, Value};

use pwright_backend_cdp_evaluation::{CdpEvaluationLimits, CdpEvaluationService};

const HANDLE_RUNTIME: &str = r#"async function(c) {
  'use strict';
  const handles = Array.prototype.slice.call(arguments, 1);
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
  const refs = new Map();
  const decode = value => {
    switch (value.t) {
      case 'u': return undefined;
      case 'null': return null;
      case 'bool': case 'num': case 'str': return value.v;
      case 'nan': return NaN;
      case 'inf': return Infinity;
      case '-inf': return -Infinity;
      case '-0': return -0;
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
        if (value.id !== null) refs.set(value.id, result);
        for (const item of value.v) result.push(decode(item));
        return result;
      }
      case 'object': {
        const result = {};
        if (value.id !== null) refs.set(value.id, result);
        for (const property of value.v) result[property.k] = decode(property.v);
        return result;
      }
      case 'ref': {
        if (!refs.has(value.id)) throw new TypeError('unknown evaluation reference');
        return refs.get(value.id);
      }
      case 'handle': {
        if (!Number.isInteger(value.i) || value.i < 0 || value.i >= handles.length)
          throw new TypeError('unknown Handle argument slot');
        return handles[value.i];
      }
      default: throw new TypeError('unknown evaluation value tag');
    }
  };
  const argument = c.hasArgument ? decode(c.argument) : undefined;
  if (c.mode === 'expression') return await eval(c.source);
  const fn = eval('(' + c.source + ')');
  if (typeof fn !== 'function') throw new TypeError('evaluation source is not a function');
  if (c.receiverSlot === null) return await fn(argument);
  if (!Number.isInteger(c.receiverSlot) || c.receiverSlot < 0 || c.receiverSlot >= handles.length)
    throw new TypeError('unknown Handle receiver slot');
  return await fn(handles[c.receiverSlot], argument);
}"#;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CdpHandleEvaluationLimits {
    evaluation: CdpEvaluationLimits,
    native_id_bytes: usize,
    preview_bytes: usize,
}

impl CdpHandleEvaluationLimits {
    pub const fn new(
        evaluation: CdpEvaluationLimits,
        native_id_bytes: usize,
        preview_bytes: usize,
    ) -> Self {
        Self {
            evaluation,
            native_id_bytes,
            preview_bytes,
        }
    }

    pub const fn evaluation(self) -> CdpEvaluationLimits {
        self.evaluation
    }

    pub const fn native_id_bytes(self) -> usize {
        self.native_id_bytes
    }

    pub const fn preview_bytes(self) -> usize {
        self.preview_bytes
    }
}

impl Default for CdpHandleEvaluationLimits {
    fn default() -> Self {
        Self::new(CdpEvaluationLimits::default(), 4096, 4096)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CdpHandleKind {
    Js,
    Element,
}

impl CdpHandleKind {
    pub const fn object_kind(self) -> ObjectKindTag {
        match self {
            Self::Js => JS_HANDLE_OBJECT_KIND.tag,
            Self::Element => ELEMENT_HANDLE_OBJECT_KIND.tag,
        }
    }
}

/// A backend-local retained handle. Native IDs never enter the portable wire.
#[derive(Clone)]
pub struct CdpEvaluatedHandle {
    client: Arc<dyn CdpClient>,
    kind: CdpHandleKind,
    object_id: Option<Arc<str>>,
    primitive_argument: Option<Value>,
    native_argument_bytes: usize,
    preview: Arc<str>,
}

impl Debug for CdpEvaluatedHandle {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CdpEvaluatedHandle")
            .field("kind", &self.kind)
            .field("object_id", &self.object_id)
            .field("primitive_argument", &self.primitive_argument)
            .field("preview", &self.preview)
            .finish_non_exhaustive()
    }
}

impl CdpEvaluatedHandle {
    pub const fn kind(&self) -> CdpHandleKind {
        self.kind
    }

    pub const fn object_kind(&self) -> ObjectKindTag {
        self.kind.object_kind()
    }

    pub fn object_id(&self) -> Option<&str> {
        self.object_id.as_deref()
    }

    pub fn preview(&self) -> &str {
        &self.preview
    }

    pub const fn native_argument_bytes(&self) -> usize {
        self.native_argument_bytes
    }

    pub fn call_argument(&self) -> Value {
        self.object_id.as_ref().map_or_else(
            || {
                self.primitive_argument
                    .clone()
                    .unwrap_or_else(|| serde_json::json!({}))
            },
            |object_id| serde_json::json!({ "objectId": object_id.as_ref() }),
        )
    }

    pub fn client(&self) -> Arc<dyn CdpClient> {
        Arc::clone(&self.client)
    }

    pub fn belongs_to_client(&self, client: &Arc<dyn CdpClient>) -> bool {
        Arc::ptr_eq(&self.client, client)
    }

    pub async fn release(&self) -> Result<(), CdpError> {
        let Some(object_id) = &self.object_id else {
            return Ok(());
        };
        self.client.runtime_release_object(object_id).await
    }
}

/// One portable request plus the exact backend-local Handles resolved by the
/// Chromium composition. Entries are ordered by first appearance in the value
/// graph and must match `EvaluateHandleParams::referenced_handles` exactly.
#[derive(Clone)]
pub struct CdpHandleEvaluationRequest {
    params: EvaluateHandleParams,
    receiver: Option<(pwright_core::ObjectId, CdpEvaluatedHandle)>,
    handles: Vec<(pwright_core::ObjectId, CdpEvaluatedHandle)>,
}

impl CdpHandleEvaluationRequest {
    pub const fn new(
        params: EvaluateHandleParams,
        handles: Vec<(pwright_core::ObjectId, CdpEvaluatedHandle)>,
    ) -> Self {
        Self {
            params,
            receiver: None,
            handles,
        }
    }

    pub const fn new_on_handle(
        params: EvaluateHandleParams,
        receiver: (pwright_core::ObjectId, CdpEvaluatedHandle),
        handles: Vec<(pwright_core::ObjectId, CdpEvaluatedHandle)>,
    ) -> Self {
        Self {
            params,
            receiver: Some(receiver),
            handles,
        }
    }

    pub const fn params(&self) -> &EvaluateHandleParams {
        &self.params
    }

    pub fn handles(&self) -> &[(pwright_core::ObjectId, CdpEvaluatedHandle)] {
        &self.handles
    }

    pub fn receiver(&self) -> Option<&(pwright_core::ObjectId, CdpEvaluatedHandle)> {
        self.receiver.as_ref()
    }
}

#[derive(Clone)]
pub struct CdpHandleEvaluationService {
    evaluation: CdpEvaluationService,
    limits: CdpHandleEvaluationLimits,
}

impl CdpHandleEvaluationService {
    pub fn new(client: Arc<dyn CdpClient>) -> Self {
        Self::with_limits(client, CdpHandleEvaluationLimits::default())
    }

    pub fn with_limits(client: Arc<dyn CdpClient>, limits: CdpHandleEvaluationLimits) -> Self {
        Self {
            evaluation: CdpEvaluationService::with_limits(client, limits.evaluation),
            limits,
        }
    }

    pub fn from_evaluation(
        evaluation: CdpEvaluationService,
        native_id_bytes: usize,
        preview_bytes: usize,
    ) -> Self {
        let limits =
            CdpHandleEvaluationLimits::new(evaluation.limits(), native_id_bytes, preview_bytes);
        Self { evaluation, limits }
    }

    pub const fn limits(&self) -> CdpHandleEvaluationLimits {
        self.limits
    }

    pub fn unique_context_id(&self) -> Option<&str> {
        self.evaluation.unique_context_id()
    }
}

impl CdpHandleEvaluationService {
    fn evaluate(&self, request: CdpHandleEvaluationRequest) -> CdpHandleEvaluationFuture<'_> {
        let params = &request.params;
        let value_params = match params.value_projection(self.limits.evaluation.value) {
            Ok(params) => params,
            Err(error) => return Box::pin(async move { Err(invalid_value(error.to_string())) }),
        };
        let lease = match self.evaluation.reserve_for_handle(&value_params) {
            Ok(lease) => lease,
            Err(error) => return Box::pin(async move { Err(map_evaluation_error(error)) }),
        };
        let invocation =
            build_invocation(&request, self.limits, self.unique_context_id().is_some());
        drop(value_params);
        drop(request);
        Box::pin(async move {
            let _lease = lease;
            let invocation = invocation?;
            let response = match (self.unique_context_id(), invocation) {
                (Some(unique_context_id), HandleInvocation::Call { arguments }) => {
                    self.evaluation
                        .shared_client()
                        .runtime_call_function_as_object_in_unique_context(
                            unique_context_id,
                            HANDLE_RUNTIME,
                            arguments,
                        )
                        .await
                }
                (None, HandleInvocation::Evaluate { script }) => {
                    self.evaluation
                        .shared_client()
                        .runtime_evaluate_as_object(&script)
                        .await
                }
                (None, HandleInvocation::Call { .. }) => {
                    return Err(invalid_value(
                        "Handle arguments require an exact CDP execution context".to_owned(),
                    ));
                }
                (Some(_), HandleInvocation::Evaluate { .. }) => {
                    unreachable!("bound Handle evaluation always uses Runtime.callFunctionOn")
                }
            }
            .map_err(|error| map_cdp_error(error, self.limits.evaluation.error_bytes))?;
            parse_response(self.evaluation.shared_client(), response, self.limits)
        })
    }
}

type CdpHandleEvaluationFuture<'a> = Pin<
    Box<
        dyn Future<
                Output = Result<CdpEvaluatedHandle, OperationError<HandleEvaluationErrorDetails>>,
            > + Send
            + 'a,
    >,
>;

impl Service<EvaluateHandleParams> for CdpHandleEvaluationService {
    type Response = CdpEvaluatedHandle;
    type Error = OperationError<HandleEvaluationErrorDetails>;
    type Future<'a> = CdpHandleEvaluationFuture<'a>;

    fn call(&self, params: EvaluateHandleParams) -> Self::Future<'_> {
        self.evaluate(CdpHandleEvaluationRequest::new(params, Vec::new()))
    }
}

impl Service<CdpHandleEvaluationRequest> for CdpHandleEvaluationService {
    type Response = CdpEvaluatedHandle;
    type Error = OperationError<HandleEvaluationErrorDetails>;
    type Future<'a> = CdpHandleEvaluationFuture<'a>;

    fn call(&self, request: CdpHandleEvaluationRequest) -> Self::Future<'_> {
        self.evaluate(request)
    }
}

pub(crate) enum HandleInvocation {
    Call { arguments: Vec<Value> },
    Evaluate { script: String },
}

pub(crate) fn build_invocation(
    request: &CdpHandleEvaluationRequest,
    limits: CdpHandleEvaluationLimits,
    exact_context: bool,
) -> Result<HandleInvocation, OperationError<HandleEvaluationErrorDetails>> {
    let expected = request
        .params
        .referenced_handles(limits.evaluation.value)
        .map_err(|error| invalid_value(error.to_string()))?;
    if expected.len() != request.handles.len()
        || expected
            .iter()
            .zip(&request.handles)
            .any(|(expected, (actual, _))| expected != actual)
    {
        return Err(invalid_value(
            "resolved native Handles do not match the portable argument graph".to_owned(),
        ));
    }
    if request.params.argument().is_some() && request.params.mode() == EvaluationMode::Expression {
        return Err(invalid_value(
            "expression evaluation cannot carry an argument".to_owned(),
        ));
    }
    let slot_offset = usize::from(request.receiver.is_some());
    let slots = request
        .handles
        .iter()
        .enumerate()
        .map(|(index, (object_id, _))| (*object_id, index.saturating_add(slot_offset)))
        .collect::<std::collections::BTreeMap<_, _>>();
    let argument = request
        .params
        .argument()
        .map(|value| handle_value_to_json(value, &slots))
        .transpose()?;
    let mode = match request.params.mode() {
        EvaluationMode::Expression => "expression",
        EvaluationMode::Function => "function",
    };
    let config = serde_json::json!({
        "source": request.params.source(),
        "mode": mode,
        "hasArgument": argument.is_some(),
        "argument": argument,
        "receiverSlot": request.receiver.as_ref().map(|_| 0),
        "errorBytes": limits.evaluation.error_bytes,
        "maxDepth": limits.evaluation.value.max_depth(),
        "maxNodes": limits.evaluation.value.max_nodes(),
        "maxEntries": limits.evaluation.value.max_entries(),
        "maxStringBytes": limits.evaluation.value.max_string_bytes(),
        "maxBinaryBytes": limits.evaluation.value.max_binary_bytes(),
    });
    let config_bytes = serde_json::to_vec(&config).map_err(json_build_error)?;
    if HANDLE_RUNTIME.len().saturating_add(config_bytes.len())
        > limits.evaluation.generated_script_bytes
    {
        return Err(invalid_value(format!(
            "generated handle-evaluation invocation has {} bytes, limit is {}",
            HANDLE_RUNTIME.len().saturating_add(config_bytes.len()),
            limits.evaluation.generated_script_bytes
        )));
    }
    if exact_context {
        let mut arguments = Vec::with_capacity(
            request
                .handles
                .len()
                .saturating_add(1)
                .saturating_add(usize::from(request.receiver.is_some())),
        );
        arguments.push(serde_json::json!({ "value": config }));
        if let Some((_, receiver)) = &request.receiver {
            arguments.push(receiver.call_argument());
        }
        arguments.extend(
            request
                .handles
                .iter()
                .map(|(_, handle)| handle.call_argument()),
        );
        return Ok(HandleInvocation::Call { arguments });
    }
    if request.receiver.is_some() || !request.handles.is_empty() {
        return Err(invalid_value(
            "Handle arguments require an exact CDP execution context".to_owned(),
        ));
    }
    build_script(&request.params, argument.as_ref(), limits)
        .map(|script| HandleInvocation::Evaluate { script })
}

fn build_script(
    params: &EvaluateHandleParams,
    argument: Option<&Value>,
    limits: CdpHandleEvaluationLimits,
) -> Result<String, OperationError<HandleEvaluationErrorDetails>> {
    let source = serde_json::to_string(params.source()).map_err(json_build_error)?;
    let argument = argument
        .map(serde_json::to_string)
        .transpose()
        .map_err(json_build_error)?;
    let mode = match params.mode() {
        EvaluationMode::Expression => "expression",
        EvaluationMode::Function => "function",
    };
    let mut script = String::with_capacity(
        HANDLE_RUNTIME
            .len()
            .saturating_add(source.len())
            .saturating_add(argument.as_ref().map_or(0, String::len))
            .saturating_add(96),
    );
    script.push('(');
    script.push_str(HANDLE_RUNTIME);
    script.push_str(")({source:");
    script.push_str(&source);
    script.push_str(",mode:\"");
    script.push_str(mode);
    script.push_str("\",hasArgument:");
    script.push_str(if argument.is_some() { "true" } else { "false" });
    script.push_str(",argument:");
    script.push_str(argument.as_deref().unwrap_or("null"));
    script.push_str(",receiverSlot:null");
    script.push_str("})");
    if script.len() > limits.evaluation.generated_script_bytes {
        return Err(invalid_value(format!(
            "generated handle-evaluation script has {} bytes, limit is {}",
            script.len(),
            limits.evaluation.generated_script_bytes
        )));
    }
    Ok(script)
}

pub(crate) fn handle_value_to_json(
    value: &HandleEvaluationValue,
    slots: &std::collections::BTreeMap<pwright_core::ObjectId, usize>,
) -> Result<Value, OperationError<HandleEvaluationErrorDetails>> {
    let mut object = Map::new();
    match value {
        HandleEvaluationValue::Undefined => insert_tag(&mut object, "u"),
        HandleEvaluationValue::Null => insert_tag(&mut object, "null"),
        HandleEvaluationValue::Bool(value) => {
            insert_tag(&mut object, "bool");
            object.insert("v".into(), Value::Bool(*value));
        }
        HandleEvaluationValue::Number(value) => {
            insert_tag(&mut object, "num");
            object.insert("v".into(), Value::from(value.get()));
        }
        HandleEvaluationValue::Nan => insert_tag(&mut object, "nan"),
        HandleEvaluationValue::Infinity => insert_tag(&mut object, "inf"),
        HandleEvaluationValue::NegativeInfinity => insert_tag(&mut object, "-inf"),
        HandleEvaluationValue::NegativeZero => insert_tag(&mut object, "-0"),
        HandleEvaluationValue::String(value) => {
            insert_tag(&mut object, "str");
            insert_string(&mut object, "v", value);
        }
        HandleEvaluationValue::Bigint(value) => {
            insert_tag(&mut object, "bigint");
            insert_string(&mut object, "v", value);
        }
        HandleEvaluationValue::Date(value) => {
            insert_tag(&mut object, "date");
            insert_string(&mut object, "v", value);
        }
        HandleEvaluationValue::Url(value) => {
            insert_tag(&mut object, "url");
            insert_string(&mut object, "v", value);
        }
        HandleEvaluationValue::Regexp { pattern, flags } => {
            insert_tag(&mut object, "regexp");
            insert_string(&mut object, "p", pattern);
            insert_string(&mut object, "f", flags);
        }
        HandleEvaluationValue::Error {
            name,
            message,
            stack,
        } => {
            insert_tag(&mut object, "error");
            insert_string(&mut object, "n", name);
            insert_string(&mut object, "m", message);
            object.insert(
                "s".into(),
                stack
                    .as_ref()
                    .map_or(Value::Null, |value| Value::String(value.clone())),
            );
        }
        HandleEvaluationValue::TypedArray { kind, bytes } => {
            insert_tag(&mut object, "typed");
            insert_string(&mut object, "k", typed_array_name(*kind));
            object.insert("b".into(), Value::String(BASE64.encode(bytes)));
        }
        HandleEvaluationValue::Array { id, items } => {
            insert_tag(&mut object, "array");
            insert_id(&mut object, *id);
            object.insert(
                "v".into(),
                Value::Array(
                    items
                        .iter()
                        .map(|item| handle_value_to_json(item, slots))
                        .collect::<Result<_, _>>()?,
                ),
            );
        }
        HandleEvaluationValue::Object { id, properties } => {
            insert_tag(&mut object, "object");
            insert_id(&mut object, *id);
            object.insert(
                "v".into(),
                Value::Array(
                    properties
                        .iter()
                        .map(|property| {
                            let mut encoded = Map::new();
                            insert_string(&mut encoded, "k", &property.key);
                            encoded
                                .insert("v".into(), handle_value_to_json(&property.value, slots)?);
                            Ok(Value::Object(encoded))
                        })
                        .collect::<Result<_, OperationError<HandleEvaluationErrorDetails>>>()?,
                ),
            );
        }
        HandleEvaluationValue::Ref(id) => {
            insert_tag(&mut object, "ref");
            object.insert("id".into(), Value::from(id.get()));
        }
        HandleEvaluationValue::Handle(object_id) => {
            let slot = slots.get(object_id).copied().ok_or_else(|| {
                invalid_value(format!(
                    "Handle `{object_id}` has no resolved native argument"
                ))
            })?;
            insert_tag(&mut object, "handle");
            object.insert("i".into(), Value::from(slot));
        }
    }
    Ok(Value::Object(object))
}

fn insert_tag(object: &mut Map<String, Value>, value: &str) {
    insert_string(object, "t", value);
}

fn insert_string(object: &mut Map<String, Value>, key: &str, value: &str) {
    object.insert(key.into(), Value::String(value.to_owned()));
}

fn insert_id(object: &mut Map<String, Value>, id: Option<NonZeroU32>) {
    object.insert(
        "id".into(),
        id.map_or(Value::Null, |id| Value::from(id.get())),
    );
}

const fn typed_array_name(kind: pwright_cap_evaluation::TypedArrayKind) -> &'static str {
    match kind {
        pwright_cap_evaluation::TypedArrayKind::Int8 => "Int8Array",
        pwright_cap_evaluation::TypedArrayKind::Uint8 => "Uint8Array",
        pwright_cap_evaluation::TypedArrayKind::Uint8Clamped => "Uint8ClampedArray",
        pwright_cap_evaluation::TypedArrayKind::Int16 => "Int16Array",
        pwright_cap_evaluation::TypedArrayKind::Uint16 => "Uint16Array",
        pwright_cap_evaluation::TypedArrayKind::Int32 => "Int32Array",
        pwright_cap_evaluation::TypedArrayKind::Uint32 => "Uint32Array",
        pwright_cap_evaluation::TypedArrayKind::Float32 => "Float32Array",
        pwright_cap_evaluation::TypedArrayKind::Float64 => "Float64Array",
        pwright_cap_evaluation::TypedArrayKind::Bigint64 => "BigInt64Array",
        pwright_cap_evaluation::TypedArrayKind::Biguint64 => "BigUint64Array",
    }
}

pub(crate) fn parse_response(
    client: Arc<dyn CdpClient>,
    response: Value,
    limits: CdpHandleEvaluationLimits,
) -> Result<CdpEvaluatedHandle, OperationError<HandleEvaluationErrorDetails>> {
    let remote = response
        .get("result")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid_response("CDP handle evaluation omitted result RemoteObject"))?;
    let remote_type = remote
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_response("CDP handle evaluation result omitted type"))?;
    let object_id: Option<Arc<str>> = remote
        .get("objectId")
        .map(|value| {
            let value = value
                .as_str()
                .ok_or_else(|| invalid_response("CDP RemoteObject objectId is not a string"))?;
            if value.is_empty() {
                return Err(invalid_response("CDP RemoteObject objectId is empty"));
            }
            if value.len() > limits.native_id_bytes {
                return Err(invalid_response(
                    "CDP RemoteObject objectId exceeds the configured byte limit",
                ));
            }
            Ok(Arc::from(value))
        })
        .transpose()?;
    let is_node = remote.get("subtype").and_then(Value::as_str) == Some("node");
    if is_node && object_id.is_none() {
        return Err(invalid_response(
            "CDP node RemoteObject does not carry an objectId",
        ));
    }
    let preview = remote
        .get("description")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| {
            remote
                .get("unserializableValue")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .or_else(|| {
            remote.get("value").map(|value| {
                value
                    .as_str()
                    .map_or_else(|| value.to_string(), str::to_owned)
            })
        })
        .unwrap_or_else(|| remote_type.to_owned());
    let preview = clip_utf8(preview, limits.preview_bytes);
    let primitive_argument = if object_id.is_some() {
        None
    } else if remote_type == "undefined" {
        Some(serde_json::json!({}))
    } else if let Some(value) = remote.get("unserializableValue") {
        let value = value.as_str().ok_or_else(|| {
            invalid_response("CDP RemoteObject unserializableValue is not a string")
        })?;
        Some(serde_json::json!({ "unserializableValue": value }))
    } else if let Some(value) = remote.get("value") {
        Some(serde_json::json!({ "value": value }))
    } else {
        return Err(invalid_response(
            "CDP primitive RemoteObject cannot be reconstructed as a call argument",
        ));
    };
    let native_argument_bytes = object_id.as_ref().map_or_else(
        || {
            primitive_argument
                .as_ref()
                .map(serde_json::to_vec)
                .transpose()
                .map(|value| value.map_or(0, |value| value.len()))
                .map_err(json_build_error)
        },
        |object_id| Ok(object_id.len()),
    )?;
    if native_argument_bytes > limits.native_id_bytes {
        return Err(invalid_response(
            "CDP RemoteObject call argument exceeds the configured native byte limit",
        ));
    }
    Ok(CdpEvaluatedHandle {
        client,
        kind: if is_node {
            CdpHandleKind::Element
        } else {
            CdpHandleKind::Js
        },
        object_id,
        primitive_argument,
        native_argument_bytes,
        preview: Arc::from(preview),
    })
}

fn clip_utf8(mut value: String, limit: usize) -> String {
    if value.len() <= limit {
        return value;
    }
    let mut end = limit;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value.truncate(end);
    value
}

pub(crate) fn map_evaluation_error(
    error: OperationError<EvaluationErrorDetails>,
) -> OperationError<HandleEvaluationErrorDetails> {
    let details = error.details().map(|details| match details {
        EvaluationErrorDetails::Javascript {
            name,
            message,
            stack,
        } => HandleEvaluationErrorDetails::Javascript {
            name: name.clone(),
            message: message.clone(),
            stack: stack.clone(),
        },
        EvaluationErrorDetails::InvalidValue(message) => {
            HandleEvaluationErrorDetails::InvalidValue(message.clone())
        }
        EvaluationErrorDetails::InvalidBackendResponse(message) => {
            HandleEvaluationErrorDetails::InvalidBackendResponse(message.clone())
        }
        EvaluationErrorDetails::Backend(message) => {
            HandleEvaluationErrorDetails::Backend(message.clone())
        }
    });
    OperationError::new(error.kind(), error.message(), details)
}

pub(crate) fn map_cdp_error(
    error: CdpError,
    error_bytes: usize,
) -> OperationError<HandleEvaluationErrorDetails> {
    let kind = pwright_backend_cdp_common::classify_cdp_failure(&error);
    let message = error.to_string();
    let details = match error {
        CdpError::JsException(message) if message.len() <= error_bytes => {
            HandleEvaluationErrorDetails::Javascript {
                name: "Error".to_owned(),
                message,
                stack: None,
            }
        }
        CdpError::JsException(_) => {
            return invalid_response("CDP JavaScript exception exceeds the configured byte limit");
        }
        _ => HandleEvaluationErrorDetails::Backend(message.clone()),
    };
    OperationError::new(kind, message, Some(details))
}

fn json_build_error(error: serde_json::Error) -> OperationError<HandleEvaluationErrorDetails> {
    invalid_response(&format!(
        "could not construct handle-evaluation script: {error}"
    ))
}

pub(crate) fn invalid_response(message: &str) -> OperationError<HandleEvaluationErrorDetails> {
    OperationError::new(
        FailureKind::ProtocolViolation,
        message,
        Some(HandleEvaluationErrorDetails::InvalidBackendResponse(
            message.to_owned(),
        )),
    )
}

pub(crate) fn invalid_value(message: String) -> OperationError<HandleEvaluationErrorDetails> {
    OperationError::new(
        FailureKind::ProtocolViolation,
        message.clone(),
        Some(HandleEvaluationErrorDetails::InvalidValue(message)),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_cap_handle_evaluation::{EvaluateHandleParams, HandleEvaluationValue};
    use pwright_core::{FailureKind, ObjectId, Service};
    use pwright_fake::FakeCdpClient;

    use super::{CdpHandleEvaluationRequest, CdpHandleEvaluationService, CdpHandleKind};
    use pwright_backend_cdp_evaluation::CdpEvaluationService;

    #[tokio::test]
    async fn element_handle_uses_exact_context_and_retains_native_id() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_response(serde_json::json!({
            "result": {
                "type": "object",
                "subtype": "node",
                "className": "HTMLBodyElement",
                "description": "body",
                "objectId": "node-7"
            }
        }));
        let evaluation = CdpEvaluationService::with_limits_in_unique_context(
            client.clone(),
            "world-7",
            Default::default(),
        );
        let service = CdpHandleEvaluationService::from_evaluation(evaluation, 64, 64);

        // Act
        let handle = service
            .call(EvaluateHandleParams::expression("document.body"))
            .await
            .unwrap();

        // Assert
        assert_eq!(handle.kind(), CdpHandleKind::Element);
        assert_eq!(handle.object_id(), Some("node-7"));
        assert_eq!(handle.preview(), "body");
        assert_eq!(client.method_names(), ["Runtime.callFunctionOn"]);
        assert_eq!(client.calls()[0].1[0]["uniqueContextId"], "world-7");
        assert_eq!(client.calls()[0].1[0]["returnByValue"], false);
    }

    #[tokio::test]
    async fn primitive_handle_has_no_native_release_call() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_response(serde_json::json!({
            "result": { "type": "number", "value": 42, "description": "42" }
        }));
        let service = CdpHandleEvaluationService::new(client.clone());

        // Act
        let handle = service
            .call(EvaluateHandleParams::expression("42"))
            .await
            .unwrap();
        handle.release().await.unwrap();

        // Assert
        assert_eq!(handle.kind(), CdpHandleKind::Js);
        assert_eq!(handle.object_id(), None);
        assert_eq!(client.method_names(), ["Runtime.evaluate"]);
    }

    #[tokio::test]
    async fn malformed_node_without_object_id_is_rejected() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_response(serde_json::json!({
            "result": { "type": "object", "subtype": "node" }
        }));
        let service = CdpHandleEvaluationService::new(client);

        // Act
        let error = service
            .call(EvaluateHandleParams::expression("document.body"))
            .await
            .unwrap_err();

        // Assert
        assert_eq!(error.kind(), FailureKind::ProtocolViolation);
    }

    #[tokio::test]
    async fn portable_handle_leaf_becomes_a_native_cdp_call_argument() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_responses([
            serde_json::json!({
                "result": {
                    "type": "object",
                    "description": "Object",
                    "objectId": "native-input"
                }
            }),
            serde_json::json!({
                "result": {
                    "type": "object",
                    "description": "Object",
                    "objectId": "native-output"
                }
            }),
        ]);
        let evaluation = CdpEvaluationService::with_limits_in_unique_context(
            client.clone(),
            "world-9",
            Default::default(),
        );
        let service = CdpHandleEvaluationService::from_evaluation(evaluation, 64, 64);
        let input = service
            .call(EvaluateHandleParams::expression("({ value: 7 })"))
            .await
            .unwrap();
        let request = CdpHandleEvaluationRequest::new(
            EvaluateHandleParams::function(
                "value => ({ copied: value.value })",
                HandleEvaluationValue::Handle(ObjectId::new(41)),
            ),
            vec![(ObjectId::new(41), input)],
        );

        // Act
        let output = service.call(request).await.unwrap();

        // Assert
        assert_eq!(output.object_id(), Some("native-output"));
        let calls = client.calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[1].1[0]["arguments"][1]["objectId"], "native-input");
        assert_eq!(
            calls[1].1[0]["arguments"][0]["value"]["argument"]["t"],
            "handle"
        );
        assert_eq!(calls[1].1[0]["arguments"][0]["value"]["argument"]["i"], 0);
    }
}
