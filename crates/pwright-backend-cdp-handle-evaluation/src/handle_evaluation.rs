//! Retained JavaScript-handle evaluation over one exact CDP execution world.

use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::num::NonZeroU32;
use std::pin::Pin;
use std::sync::Arc;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use pwright_browser_model::{ELEMENT_HANDLE_OBJECT_KIND, JS_HANDLE_OBJECT_KIND};
use pwright_cap_evaluation::{
    EvaluationErrorDetails, EvaluationMode, EvaluationNumber, EvaluationValue,
};
use pwright_cap_handle_evaluation::{
    EvaluateHandleParams, HandleEvaluationErrorDetails, HandleEvaluationValue,
};
use pwright_cdp::CdpClient;
use pwright_cdp::connection::CdpError;
use pwright_core::{FailureKind, ObjectKindTag, OperationError, Service};
use serde_json::{Map, Value};

use crate::CdpHandleEvaluationProgram;
use pwright_backend_cdp_evaluation::{CdpEvaluationLimits, CdpEvaluationService, EvaluationLease};

pub(crate) mod acquisition;
#[cfg(test)]
mod acquisition_live;

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
  if (c.mode === 'expression') return await globalThis.eval(c.source);
  const fn = globalThis.eval('(' + c.source + ')');
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
    /// Retain one already-returned CDP `RemoteObject` without another browser
    /// round trip. Event producers remain responsible for compensating the
    /// native `objectId` when this validation fails before ownership transfers.
    pub fn from_remote_object(
        client: Arc<dyn CdpClient>,
        remote: &Value,
        limits: CdpHandleEvaluationLimits,
    ) -> Result<Self, OperationError<HandleEvaluationErrorDetails>> {
        parse_remote_object(client, remote, limits)
    }

    pub(crate) fn element_reference(
        client: Arc<dyn CdpClient>,
        object_id: &str,
        max_native_bytes: usize,
    ) -> Result<Self, OperationError<HandleEvaluationErrorDetails>> {
        if object_id.is_empty() || object_id.len() > max_native_bytes {
            return Err(invalid_value(
                "native element reference is empty or oversized".into(),
            ));
        }
        Ok(Self {
            client,
            kind: CdpHandleKind::Element,
            object_id: Some(Arc::from(object_id)),
            primitive_argument: None,
            native_argument_bytes: object_id.len(),
            preview: Arc::from(""),
        })
    }

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

    /// Reconstruct a retained primitive without another browser round trip.
    /// Object Handles return `None`; their JSON projection still executes in
    /// the exact source world. Keeping explicit null here also avoids CDP's
    /// generated `Option<Value>` model collapsing `{ value: null }` into an
    /// absent CallArgument field (which CDP interprets as `undefined`).
    pub fn primitive_value(
        &self,
    ) -> Result<Option<EvaluationValue>, OperationError<HandleEvaluationErrorDetails>> {
        if self.object_id.is_some() {
            return Ok(None);
        }
        let Some(argument) = self.primitive_argument.as_ref() else {
            return Err(invalid_response(
                "primitive Handle omitted its retained CDP call argument",
            ));
        };
        if let Some(value) = argument.get("value") {
            let value = match value {
                Value::Null => EvaluationValue::Null,
                Value::Bool(value) => EvaluationValue::Bool(*value),
                Value::Number(value) => EvaluationValue::Number(
                    EvaluationNumber::new(value.as_f64().ok_or_else(|| {
                        invalid_response("primitive Handle number is not representable as f64")
                    })?)
                    .map_err(|error| invalid_response(&error.to_string()))?,
                ),
                Value::String(value) => EvaluationValue::String(value.clone()),
                Value::Array(_) | Value::Object(_) => {
                    return Err(invalid_response(
                        "primitive Handle contains a non-primitive CDP value",
                    ));
                }
            };
            return Ok(Some(value));
        }
        if let Some(value) = argument.get("unserializableValue").and_then(Value::as_str) {
            let value = match value {
                "NaN" => EvaluationValue::Nan,
                "Infinity" => EvaluationValue::Infinity,
                "-Infinity" => EvaluationValue::NegativeInfinity,
                "-0" => EvaluationValue::NegativeZero,
                value if value.ends_with('n') => {
                    EvaluationValue::Bigint(value[..value.len() - 1].to_owned())
                }
                _ => {
                    return Err(invalid_response(
                        "primitive Handle contains an unknown unserializable CDP value",
                    ));
                }
            };
            return Ok(Some(value));
        }
        if argument.as_object().is_some_and(serde_json::Map::is_empty) {
            return Ok(Some(EvaluationValue::Undefined));
        }
        Err(invalid_response(
            "primitive Handle call argument has an invalid shape",
        ))
    }

    /// Whether this retained result is exactly the JavaScript `null`
    /// primitive. Object Handles and every other primitive return `false`.
    /// This keeps optional-ElementHandle classification inside the backend
    /// type that owns the retained CDP representation.
    pub fn is_null_primitive(&self) -> Result<bool, OperationError<HandleEvaluationErrorDetails>> {
        self.primitive_value()
            .map(|value| matches!(value, Some(EvaluationValue::Null)))
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
    params: CdpHandleEvaluationProgram,
    receiver: Option<(pwright_core::ObjectId, CdpEvaluatedHandle)>,
    native_receiver: Option<CdpEvaluatedHandle>,
    handles: Vec<(pwright_core::ObjectId, CdpEvaluatedHandle)>,
}

impl CdpHandleEvaluationRequest {
    pub const fn new(
        params: EvaluateHandleParams,
        handles: Vec<(pwright_core::ObjectId, CdpEvaluatedHandle)>,
    ) -> Self {
        Self {
            params: CdpHandleEvaluationProgram::from_portable(params),
            receiver: None,
            native_receiver: None,
            handles,
        }
    }

    pub const fn new_on_handle(
        params: EvaluateHandleParams,
        receiver: (pwright_core::ObjectId, CdpEvaluatedHandle),
        handles: Vec<(pwright_core::ObjectId, CdpEvaluatedHandle)>,
    ) -> Self {
        Self {
            params: CdpHandleEvaluationProgram::from_portable(params),
            receiver: Some(receiver),
            native_receiver: None,
            handles,
        }
    }

    /// Portable validation projection. For native expression arguments this is
    /// not the executable mode; use program() to inspect invocation semantics.
    pub const fn params(&self) -> &EvaluateHandleParams {
        self.params.portable_projection()
    }

    pub const fn program(&self) -> &CdpHandleEvaluationProgram {
        &self.params
    }

    pub const fn new_program(
        params: CdpHandleEvaluationProgram,
        receiver: Option<(pwright_core::ObjectId, CdpEvaluatedHandle)>,
        handles: Vec<(pwright_core::ObjectId, CdpEvaluatedHandle)>,
    ) -> Self {
        Self {
            params,
            receiver,
            native_receiver: None,
            handles,
        }
    }

    /// Temporary backend-owned receiver with no portable Handle registration.
    /// Its composing owner must retain and release the native object.
    pub fn new_on_native_handle(
        params: EvaluateHandleParams,
        receiver: CdpEvaluatedHandle,
    ) -> Self {
        Self {
            params: CdpHandleEvaluationProgram::from_portable(params),
            receiver: None,
            native_receiver: Some(receiver),
            handles: Vec::new(),
        }
    }

    fn receiver_handle(&self) -> Option<&CdpEvaluatedHandle> {
        self.receiver
            .as_ref()
            .map(|(_, handle)| handle)
            .or(self.native_receiver.as_ref())
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
        self.evaluate_with(request, false, |handle, _lease| handle)
    }

    /// Property pipelines keep the original admission through carrier reading
    /// and compensation instead of reserving a second concurrent operation.
    pub(crate) fn call_retaining_lease(
        &self,
        request: CdpHandleEvaluationRequest,
    ) -> CdpHandleEvaluationFuture<'_, (CdpEvaluatedHandle, Option<EvaluationLease>)> {
        self.evaluate_with(request, false, |handle, lease| (handle, lease))
    }

    /// A temporary owner lookup must never dispose its borrowed receiver when
    /// a corrupt acquisition response reuses that native identity. The single
    /// protected ID is bounded and copied only under the evaluation admission.
    pub(crate) fn call_retaining_lease_protecting_receiver(
        &self,
        request: CdpHandleEvaluationRequest,
    ) -> CdpHandleEvaluationFuture<'_, (CdpEvaluatedHandle, Option<EvaluationLease>)> {
        self.evaluate_with(request, true, |handle, lease| (handle, lease))
    }

    fn evaluate_with<'a, T: Send + 'a>(
        &'a self,
        request: CdpHandleEvaluationRequest,
        protect_receiver: bool,
        finish: impl FnOnce(CdpEvaluatedHandle, Option<EvaluationLease>) -> T + Send + 'a,
    ) -> CdpHandleEvaluationFuture<'a, T> {
        let params = &request.params;
        let value_params = match params.value_projection(self.limits.evaluation.value) {
            Ok(params) => params,
            Err(error) => return Box::pin(async move { Err(invalid_value(error.to_string())) }),
        };
        let lease = match self.evaluation.reserve_for_handle(&value_params) {
            Ok(lease) => lease,
            Err(error) => return Box::pin(async move { Err(map_evaluation_error(error)) }),
        };
        let protected_receiver = if protect_receiver {
            let Some(id) = request
                .receiver_handle()
                .and_then(CdpEvaluatedHandle::object_id)
                .filter(|id| !id.is_empty() && id.len() <= self.limits.native_id_bytes())
            else {
                let error = acquisition::bounded_protocol_error(
                    "protected acquisition requires a bounded native receiver identity",
                    self.limits.evaluation.error_bytes,
                );
                return Box::pin(async move { Err(error) });
            };
            Some(id.to_owned())
        } else {
            None
        };
        let invocation =
            build_invocation(&request, self.limits, self.unique_context_id().is_some());
        drop(value_params);
        drop(request);
        Box::pin(async move {
            let invocation = invocation?;
            let response = match (self.unique_context_id(), invocation) {
                (Some(unique_context_id), HandleInvocation::Call { arguments }) => {
                    self.evaluation
                        .shared_client()
                        .runtime_call_function_as_object_in_unique_context_to_settlement(
                            unique_context_id,
                            HANDLE_RUNTIME,
                            arguments,
                        )
                        .await
                }
                (None, HandleInvocation::Evaluate { script }) => {
                    self.evaluation
                        .shared_client()
                        .runtime_evaluate_as_object_to_settlement(&script)
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
            let handle = match protected_receiver.as_deref() {
                Some(receiver) => {
                    acquisition::finish_protecting_receiver(
                        self.evaluation.shared_client(),
                        response,
                        self.limits,
                        receiver,
                    )
                    .await?
                }
                None => {
                    acquisition::finish(self.evaluation.shared_client(), response, self.limits)
                        .await?
                }
            };
            Ok(finish(handle, lease))
        })
    }
}

type CdpHandleEvaluationFuture<'a, T = CdpEvaluatedHandle> = Pin<
    Box<dyn Future<Output = Result<T, OperationError<HandleEvaluationErrorDetails>>> + Send + 'a>,
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
    let slot_offset = usize::from(request.receiver_handle().is_some());
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
        "receiverSlot": request.receiver_handle().map(|_| 0),
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
                .saturating_add(usize::from(request.receiver_handle().is_some())),
        );
        arguments.push(serde_json::json!({ "value": config }));
        if let Some(receiver) = request.receiver_handle() {
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
    if request.receiver_handle().is_some() || !request.handles.is_empty() {
        return Err(invalid_value(
            "Handle arguments require an exact CDP execution context".to_owned(),
        ));
    }
    build_script(&request.params, argument.as_ref(), limits)
        .map(|script| HandleInvocation::Evaluate { script })
}

fn build_script(
    params: &CdpHandleEvaluationProgram,
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
    parse_borrowed_response(client, &response, limits)
}

fn parse_borrowed_response(
    client: Arc<dyn CdpClient>,
    response: &Value,
    limits: CdpHandleEvaluationLimits,
) -> Result<CdpEvaluatedHandle, OperationError<HandleEvaluationErrorDetails>> {
    let remote = response
        .get("result")
        .ok_or_else(|| invalid_response("CDP handle evaluation omitted result RemoteObject"))?;
    parse_remote_object(client, remote, limits)
}

fn parse_remote_object(
    client: Arc<dyn CdpClient>,
    remote: &Value,
    limits: CdpHandleEvaluationLimits,
) -> Result<CdpEvaluatedHandle, OperationError<HandleEvaluationErrorDetails>> {
    let remote = remote
        .as_object()
        .ok_or_else(|| invalid_response("CDP RemoteObject is not an object"))?;
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

    use super::{
        CdpEvaluatedHandle, CdpHandleEvaluationLimits, CdpHandleEvaluationRequest,
        CdpHandleEvaluationService, CdpHandleKind,
    };
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

    #[test]
    fn primitive_handle_reconstructs_explicit_null_without_cdp_call_argument_round_trip() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let client_dyn: Arc<dyn pwright_cdp::CdpClient> = client;
        let handle = super::parse_response(
            client_dyn,
            serde_json::json!({
                "result": { "type": "object", "subtype": "null", "value": null }
            }),
            Default::default(),
        )
        .expect("valid primitive null Handle");

        // Act
        let value = handle.primitive_value().expect("valid retained primitive");

        // Assert
        assert_eq!(value, Some(pwright_cap_evaluation::EvaluationValue::Null));
    }

    #[test]
    fn existing_remote_object_becomes_a_retained_handle_without_browser_io() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let client_dyn: Arc<dyn pwright_cdp::CdpClient> = client.clone();
        let remote = serde_json::json!({
            "type": "object",
            "description": "Object",
            "objectId": "console-argument-1"
        });

        // Act
        let handle = CdpEvaluatedHandle::from_remote_object(
            client_dyn,
            &remote,
            CdpHandleEvaluationLimits::default(),
        )
        .expect("event RemoteObject should validate");

        // Assert
        assert_eq!(handle.kind(), CdpHandleKind::Js);
        assert_eq!(handle.object_id(), Some("console-argument-1"));
        assert_eq!(handle.preview(), "Object");
        assert!(client.calls().is_empty());
    }

    #[test]
    fn existing_remote_primitive_preserves_its_json_value_without_browser_io() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let client_dyn: Arc<dyn pwright_cdp::CdpClient> = client.clone();
        let remote = serde_json::json!({
            "type": "number",
            "value": 5,
            "description": "5"
        });

        // Act
        let handle = CdpEvaluatedHandle::from_remote_object(
            client_dyn,
            &remote,
            CdpHandleEvaluationLimits::default(),
        )
        .expect("event primitive should validate");
        let value = handle.primitive_value().expect("retained primitive");

        // Assert
        assert_eq!(
            value,
            Some(pwright_cap_evaluation::EvaluationValue::Number(
                pwright_cap_evaluation::EvaluationNumber::new(5.0).unwrap()
            ))
        );
        assert!(client.calls().is_empty());
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
        assert_eq!(error.kind(), FailureKind::Cleanup);
    }

    #[tokio::test]
    async fn native_expression_keeps_reference_slots_and_exact_execution_mode() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_responses([
            serde_json::json!({"result": {"type": "object", "objectId": "native-input"}}),
            serde_json::json!({"result": {"type": "number", "value": 42}}),
        ]);
        let evaluation = CdpEvaluationService::with_limits_in_unique_context(
            client.clone(),
            "world-9",
            Default::default(),
        );
        let service = CdpHandleEvaluationService::from_evaluation(evaluation, 64, 64);
        let input = service
            .call(EvaluateHandleParams::expression("({})"))
            .await
            .unwrap();
        let request = CdpHandleEvaluationRequest::new_program(
            crate::CdpHandleEvaluationProgram::expression(
                "40 + 2".into(),
                HandleEvaluationValue::Handle(ObjectId::new(41)),
            ),
            None,
            vec![(ObjectId::new(41), input)],
        );

        // Act
        let output = service.call(request).await.unwrap();

        // Assert
        assert_eq!(output.object_id(), None);
        let calls = client.calls();
        let config = &calls[1].1[0]["arguments"][0]["value"];
        assert_eq!(config["mode"], "expression");
        assert_eq!(config["source"], "40 + 2");
        assert_eq!(config["hasArgument"], true);
        assert_eq!(config["argument"]["t"], "handle");
        assert_eq!(calls[1].1[0]["arguments"][1]["objectId"], "native-input");
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

#[cfg(test)]
mod acquisition_limits_tests;
#[cfg(test)]
mod acquisition_tests;
