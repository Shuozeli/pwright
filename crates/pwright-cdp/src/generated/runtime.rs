//! CDP `Runtime` domain — generated from protocol JSON.
//!
//! Runtime domain exposes JavaScript runtime by means of remote evaluation and mirror objects.
//! Evaluation results are returned as mirror object that expose object type, string representation
//! and unique identifier that can be used for further object reference. Original objects are
//! maintained in memory unless they are either explicitly released or are released along with the
//! other objects in their object group.

#![allow(clippy::doc_markdown)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Unique script identifier.
pub type ScriptId = String;

/// Represents options for serialization. Overrides `generatePreview` and `returnByValue`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializationOptions {
    pub serialization: String,
    /// Deep serialization depth. Default is full depth. Respected only in `deep` serialization mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_depth: Option<i64>,
    /// Embedder-specific parameters. For example if connected to V8 in Chrome these control DOM
    /// serialization via `maxNodeDepth: integer` and `includeShadowTree: "none" | "open" | "all"`.
    /// Values can be only of type string or integer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additional_parameters: Option<Value>,
}

/// Represents deep serialized value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeepSerializedValue {
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_id: Option<String>,
    /// Set if value reference met more then once during serialization. In such
    /// case, value is provided only to one of the serialized values. Unique
    /// per value in the scope of one CDP call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weak_local_object_reference: Option<i64>,
}

/// Unique object identifier.
pub type RemoteObjectId = String;

/// Primitive value which cannot be JSON-stringified. Includes values `-0`, `NaN`, `Infinity`,
/// `-Infinity`, and bigint literals.
pub type UnserializableValue = String;

/// Mirror object referencing original JavaScript object.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteObject {
    /// Object type.
    #[serde(rename = "type")]
    pub r#type: String,
    /// Object subtype hint. Specified for `object` type values only.
    /// NOTE: If you change anything here, make sure to also update
    /// `subtype` in `ObjectPreview` and `PropertyPreview` below.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtype: Option<String>,
    /// Object class (constructor) name. Specified for `object` type values only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,
    /// Remote object value in case of primitive values or JSON values (if it was requested).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    /// Primitive value which can not be JSON-stringified does not have `value`, but gets this
    /// property.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unserializable_value: Option<UnserializableValue>,
    /// String representation of the object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Deep serialized value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deep_serialized_value: Option<DeepSerializedValue>,
    /// Unique object identifier (for non-primitive values).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_id: Option<RemoteObjectId>,
    /// Preview containing abbreviated property values. Specified for `object` type values only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<ObjectPreview>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_preview: Option<CustomPreview>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomPreview {
    /// The JSON-stringified result of formatter.header(object, config) call.
    /// It contains json ML array that represents RemoteObject.
    pub header: String,
    /// If formatter returns true as a result of formatter.hasBody call then bodyGetterId will
    /// contain RemoteObjectId for the function that returns result of formatter.body(object, config) call.
    /// The result value is json ML array.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_getter_id: Option<RemoteObjectId>,
}

/// Object containing abbreviated remote object value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectPreview {
    /// Object type.
    #[serde(rename = "type")]
    pub r#type: String,
    /// Object subtype hint. Specified for `object` type values only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtype: Option<String>,
    /// String representation of the object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// True iff some of the properties or entries of the original object did not fit.
    pub overflow: bool,
    /// List of the properties.
    pub properties: Vec<PropertyPreview>,
    /// List of the entries. Specified for `map` and `set` subtype values only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entries: Option<Vec<EntryPreview>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyPreview {
    /// Property name.
    pub name: String,
    /// Object type. Accessor means that the property itself is an accessor property.
    #[serde(rename = "type")]
    pub r#type: String,
    /// User-friendly property value string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Nested value preview.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_preview: Option<ObjectPreview>,
    /// Object subtype hint. Specified for `object` type values only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtype: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryPreview {
    /// Preview of the key. Specified for map-like collection entries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<ObjectPreview>,
    /// Preview of the value.
    pub value: ObjectPreview,
}

/// Object property descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyDescriptor {
    /// Property name or symbol description.
    pub name: String,
    /// The value associated with the property.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<RemoteObject>,
    /// True if the value associated with the property may be changed (data descriptors only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub writable: Option<bool>,
    /// A function which serves as a getter for the property, or `undefined` if there is no getter
    /// (accessor descriptors only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub get: Option<RemoteObject>,
    /// A function which serves as a setter for the property, or `undefined` if there is no setter
    /// (accessor descriptors only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set: Option<RemoteObject>,
    /// True if the type of this property descriptor may be changed and if the property may be
    /// deleted from the corresponding object.
    pub configurable: bool,
    /// True if this property shows up during enumeration of the properties on the corresponding
    /// object.
    pub enumerable: bool,
    /// True if the result was thrown during the evaluation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub was_thrown: Option<bool>,
    /// True if the property is owned for the object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_own: Option<bool>,
    /// Property symbol object, if the property is of the `symbol` type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<RemoteObject>,
}

/// Object internal property descriptor. This property isn't normally visible in JavaScript code.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalPropertyDescriptor {
    /// Conventional property name.
    pub name: String,
    /// The value associated with the property.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<RemoteObject>,
}

/// Object private field descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivatePropertyDescriptor {
    /// Private property name.
    pub name: String,
    /// The value associated with the private property.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<RemoteObject>,
    /// A function which serves as a getter for the private property,
    /// or `undefined` if there is no getter (accessor descriptors only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub get: Option<RemoteObject>,
    /// A function which serves as a setter for the private property,
    /// or `undefined` if there is no setter (accessor descriptors only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set: Option<RemoteObject>,
}

/// Represents function call argument. Either remote object id `objectId`, primitive `value`,
/// unserializable primitive value or neither of (for undefined) them should be specified.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CallArgument {
    /// Primitive value or serializable javascript object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    /// Primitive value which can not be JSON-stringified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unserializable_value: Option<UnserializableValue>,
    /// Remote object handle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_id: Option<RemoteObjectId>,
}

/// Id of an execution context.
pub type ExecutionContextId = i64;

/// Description of an isolated world.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionContextDescription {
    /// Unique id of the execution context. It can be used to specify in which execution context
    /// script evaluation should be performed.
    pub id: ExecutionContextId,
    /// Execution context origin.
    pub origin: String,
    /// Human readable name describing given context.
    pub name: String,
    /// A system-unique execution context identifier. Unlike the id, this is unique across
    /// multiple processes, so can be reliably used to identify specific context while backend
    /// performs a cross-process navigation.
    pub unique_id: String,
    /// Embedder-specific auxiliary data likely matching {isDefault: boolean, type: 'default'|'isolated'|'worker', frameId: string}
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aux_data: Option<Value>,
}

/// Detailed information about exception (or error) that was thrown during script compilation or
/// execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionDetails {
    /// Exception id.
    pub exception_id: i64,
    /// Exception text, which should be used together with exception object when available.
    pub text: String,
    /// Line number of the exception location (0-based).
    pub line_number: i64,
    /// Column number of the exception location (0-based).
    pub column_number: i64,
    /// Script ID of the exception location.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script_id: Option<ScriptId>,
    /// URL of the exception location, to be used when the script was not reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// JavaScript stack trace if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack_trace: Option<StackTrace>,
    /// Exception object if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exception: Option<RemoteObject>,
    /// Identifier of the context where exception happened.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_context_id: Option<ExecutionContextId>,
    /// Dictionary with entries of meta data that the client associated
    /// with this exception, such as information about associated network
    /// requests, etc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exception_meta_data: Option<Value>,
}

/// Number of milliseconds since epoch.
pub type Timestamp = f64;

/// Number of milliseconds.
pub type TimeDelta = f64;

/// Stack entry for runtime errors and assertions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallFrame {
    /// JavaScript function name.
    pub function_name: String,
    /// JavaScript script id.
    pub script_id: ScriptId,
    /// JavaScript script name or url.
    pub url: String,
    /// JavaScript script line number (0-based).
    pub line_number: i64,
    /// JavaScript script column number (0-based).
    pub column_number: i64,
}

/// Call frames for assertions or error messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackTrace {
    /// String label of this stack trace. For async traces this may be a name of the function that
    /// initiated the async call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// JavaScript function name.
    pub call_frames: Vec<CallFrame>,
    /// Asynchronous JavaScript stack trace that preceded this stack, if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<Box<StackTrace>>,
    /// Asynchronous JavaScript stack trace that preceded this stack, if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<StackTraceId>,
}

/// Unique identifier of current debugger.
pub type UniqueDebuggerId = String;

/// If `debuggerId` is set stack trace comes from another debugger and can be resolved there. This
/// allows to track cross-debugger calls. See `Runtime.StackTrace` and `Debugger.paused` for usages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackTraceId {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debugger_id: Option<UniqueDebuggerId>,
}

/// Parameters for `Runtime.awaitPromise`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AwaitPromiseParams {
    /// Identifier of the promise.
    pub promise_object_id: RemoteObjectId,
    /// Whether the result is expected to be a JSON object that should be sent by value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_by_value: Option<bool>,
    /// Whether preview should be generated for the result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_preview: Option<bool>,
}

/// Return type for `Runtime.awaitPromise`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AwaitPromiseReturns {
    /// Promise result. Will contain rejected value if promise was rejected.
    pub result: RemoteObject,
    /// Exception details if stack strace is available.
    #[serde(default)]
    pub exception_details: Option<ExceptionDetails>,
}

/// Parameters for `Runtime.callFunctionOn`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CallFunctionOnParams {
    /// Declaration of the function to call.
    pub function_declaration: String,
    /// Identifier of the object to call function on. Either objectId or executionContextId should
    /// be specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<RemoteObjectId>,
    /// Call arguments. All call arguments must belong to the same JavaScript world as the target
    /// object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<CallArgument>>,
    /// In silent mode exceptions thrown during evaluation are not reported and do not pause
    /// execution. Overrides `setPauseOnException` state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub silent: Option<bool>,
    /// Whether the result is expected to be a JSON object which should be sent by value.
    /// Can be overriden by `serializationOptions`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_by_value: Option<bool>,
    /// Whether preview should be generated for the result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_preview: Option<bool>,
    /// Whether execution should be treated as initiated by user in the UI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_gesture: Option<bool>,
    /// Whether execution should `await` for resulting value and return once awaited promise is
    /// resolved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub await_promise: Option<bool>,
    /// Specifies execution context which global object will be used to call function on. Either
    /// executionContextId or objectId should be specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_context_id: Option<ExecutionContextId>,
    /// Symbolic group name that can be used to release multiple objects. If objectGroup is not
    /// specified and objectId is, objectGroup will be inherited from object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_group: Option<String>,
    /// Whether to throw an exception if side effect cannot be ruled out during evaluation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub throw_on_side_effect: Option<bool>,
    /// An alternative way to specify the execution context to call function on.
    /// Compared to contextId that may be reused across processes, this is guaranteed to be
    /// system-unique, so it can be used to prevent accidental function call
    /// in context different than intended (e.g. as a result of navigation across process
    /// boundaries).
    /// This is mutually exclusive with `executionContextId`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique_context_id: Option<String>,
    /// Specifies the result serialization. If provided, overrides
    /// `generatePreview` and `returnByValue`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serialization_options: Option<SerializationOptions>,
}

/// Return type for `Runtime.callFunctionOn`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallFunctionOnReturns {
    /// Call result.
    pub result: RemoteObject,
    /// Exception details.
    #[serde(default)]
    pub exception_details: Option<ExceptionDetails>,
}

/// Parameters for `Runtime.compileScript`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CompileScriptParams {
    /// Expression to compile.
    pub expression: String,
    /// Source url to be set for the script.
    #[serde(rename = "sourceURL")]
    pub source_url: String,
    /// Specifies whether the compiled script should be persisted.
    pub persist_script: bool,
    /// Specifies in which execution context to perform script run. If the parameter is omitted the
    /// evaluation will be performed in the context of the inspected page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_context_id: Option<ExecutionContextId>,
}

/// Return type for `Runtime.compileScript`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileScriptReturns {
    /// Id of the script.
    #[serde(default)]
    pub script_id: Option<ScriptId>,
    /// Exception details.
    #[serde(default)]
    pub exception_details: Option<ExceptionDetails>,
}

/// Parameters for `Runtime.evaluate`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateParams {
    /// Expression to evaluate.
    pub expression: String,
    /// Symbolic group name that can be used to release multiple objects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_group: Option<String>,
    /// Determines whether Command Line API should be available during the evaluation.
    #[serde(rename = "includeCommandLineAPI")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_command_line_api: Option<bool>,
    /// In silent mode exceptions thrown during evaluation are not reported and do not pause
    /// execution. Overrides `setPauseOnException` state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub silent: Option<bool>,
    /// Specifies in which execution context to perform evaluation. If the parameter is omitted the
    /// evaluation will be performed in the context of the inspected page.
    /// This is mutually exclusive with `uniqueContextId`, which offers an
    /// alternative way to identify the execution context that is more reliable
    /// in a multi-process environment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_id: Option<ExecutionContextId>,
    /// Whether the result is expected to be a JSON object that should be sent by value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_by_value: Option<bool>,
    /// Whether preview should be generated for the result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_preview: Option<bool>,
    /// Whether execution should be treated as initiated by user in the UI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_gesture: Option<bool>,
    /// Whether execution should `await` for resulting value and return once awaited promise is
    /// resolved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub await_promise: Option<bool>,
    /// Whether to throw an exception if side effect cannot be ruled out during evaluation.
    /// This implies `disableBreaks` below.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub throw_on_side_effect: Option<bool>,
    /// Terminate execution after timing out (number of milliseconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<TimeDelta>,
    /// Disable breakpoints during execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_breaks: Option<bool>,
    /// Setting this flag to true enables `let` re-declaration and top-level `await`.
    /// Note that `let` variables can only be re-declared if they originate from
    /// `replMode` themselves.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repl_mode: Option<bool>,
    /// The Content Security Policy (CSP) for the target might block 'unsafe-eval'
    /// which includes eval(), Function(), setTimeout() and setInterval()
    /// when called with non-callable arguments. This flag bypasses CSP for this
    /// evaluation and allows unsafe-eval. Defaults to true.
    #[serde(rename = "allowUnsafeEvalBlockedByCSP")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_unsafe_eval_blocked_by_csp: Option<bool>,
    /// An alternative way to specify the execution context to evaluate in.
    /// Compared to contextId that may be reused across processes, this is guaranteed to be
    /// system-unique, so it can be used to prevent accidental evaluation of the expression
    /// in context different than intended (e.g. as a result of navigation across process
    /// boundaries).
    /// This is mutually exclusive with `contextId`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique_context_id: Option<String>,
    /// Specifies the result serialization. If provided, overrides
    /// `generatePreview` and `returnByValue`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serialization_options: Option<SerializationOptions>,
}

/// Return type for `Runtime.evaluate`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateReturns {
    /// Evaluation result.
    pub result: RemoteObject,
    /// Exception details.
    #[serde(default)]
    pub exception_details: Option<ExceptionDetails>,
}

/// Return type for `Runtime.getIsolateId`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetIsolateIdReturns {
    /// The isolate id.
    pub id: String,
}

/// Return type for `Runtime.getHeapUsage`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetHeapUsageReturns {
    /// Used JavaScript heap size in bytes.
    pub used_size: f64,
    /// Allocated JavaScript heap size in bytes.
    pub total_size: f64,
    /// Used size in bytes in the embedder's garbage-collected heap.
    pub embedder_heap_used_size: f64,
    /// Size in bytes of backing storage for array buffers and external strings.
    pub backing_storage_size: f64,
}

/// Parameters for `Runtime.getProperties`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPropertiesParams {
    /// Identifier of the object to return properties for.
    pub object_id: RemoteObjectId,
    /// If true, returns properties belonging only to the element itself, not to its prototype
    /// chain.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub own_properties: Option<bool>,
    /// If true, returns accessor properties (with getter/setter) only; internal properties are not
    /// returned either.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessor_properties_only: Option<bool>,
    /// Whether preview should be generated for the results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_preview: Option<bool>,
    /// If true, returns non-indexed properties only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub non_indexed_properties_only: Option<bool>,
}

/// Return type for `Runtime.getProperties`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPropertiesReturns {
    /// Object properties.
    pub result: Vec<PropertyDescriptor>,
    /// Internal object properties (only of the element itself).
    #[serde(default)]
    pub internal_properties: Option<Vec<InternalPropertyDescriptor>>,
    /// Object private properties.
    #[serde(default)]
    pub private_properties: Option<Vec<PrivatePropertyDescriptor>>,
    /// Exception details.
    #[serde(default)]
    pub exception_details: Option<ExceptionDetails>,
}

/// Parameters for `Runtime.globalLexicalScopeNames`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GlobalLexicalScopeNamesParams {
    /// Specifies in which execution context to lookup global scope variables.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_context_id: Option<ExecutionContextId>,
}

/// Return type for `Runtime.globalLexicalScopeNames`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalLexicalScopeNamesReturns {
    pub names: Vec<String>,
}

/// Parameters for `Runtime.queryObjects`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryObjectsParams {
    /// Identifier of the prototype to return objects for.
    pub prototype_object_id: RemoteObjectId,
    /// Symbolic group name that can be used to release the results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_group: Option<String>,
}

/// Return type for `Runtime.queryObjects`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryObjectsReturns {
    /// Array with objects.
    pub objects: RemoteObject,
}

/// Parameters for `Runtime.releaseObject`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseObjectParams {
    /// Identifier of the object to release.
    pub object_id: RemoteObjectId,
}

/// Parameters for `Runtime.releaseObjectGroup`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseObjectGroupParams {
    /// Symbolic object group name.
    pub object_group: String,
}

/// Parameters for `Runtime.runScript`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunScriptParams {
    /// Id of the script to run.
    pub script_id: ScriptId,
    /// Specifies in which execution context to perform script run. If the parameter is omitted the
    /// evaluation will be performed in the context of the inspected page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_context_id: Option<ExecutionContextId>,
    /// Symbolic group name that can be used to release multiple objects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_group: Option<String>,
    /// In silent mode exceptions thrown during evaluation are not reported and do not pause
    /// execution. Overrides `setPauseOnException` state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub silent: Option<bool>,
    /// Determines whether Command Line API should be available during the evaluation.
    #[serde(rename = "includeCommandLineAPI")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_command_line_api: Option<bool>,
    /// Whether the result is expected to be a JSON object which should be sent by value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_by_value: Option<bool>,
    /// Whether preview should be generated for the result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_preview: Option<bool>,
    /// Whether execution should `await` for resulting value and return once awaited promise is
    /// resolved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub await_promise: Option<bool>,
}

/// Return type for `Runtime.runScript`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunScriptReturns {
    /// Run result.
    pub result: RemoteObject,
    /// Exception details.
    #[serde(default)]
    pub exception_details: Option<ExceptionDetails>,
}

/// Parameters for `Runtime.setAsyncCallStackDepth`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetAsyncCallStackDepthParams {
    /// Maximum depth of async call stacks. Setting to `0` will effectively disable collecting async
    /// call stacks (default).
    pub max_depth: i64,
}

/// Parameters for `Runtime.setCustomObjectFormatterEnabled`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetCustomObjectFormatterEnabledParams {
    pub enabled: bool,
}

/// Parameters for `Runtime.setMaxCallStackSizeToCapture`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetMaxCallStackSizeToCaptureParams {
    pub size: i64,
}

/// Parameters for `Runtime.addBinding`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AddBindingParams {
    pub name: String,
    /// If specified, the binding would only be exposed to the specified
    /// execution context. If omitted and `executionContextName` is not set,
    /// the binding is exposed to all execution contexts of the target.
    /// This parameter is mutually exclusive with `executionContextName`.
    /// Deprecated in favor of `executionContextName` due to an unclear use case
    /// and bugs in implementation (crbug.com/1169639). `executionContextId` will be
    /// removed in the future.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_context_id: Option<ExecutionContextId>,
    /// If specified, the binding is exposed to the executionContext with
    /// matching name, even for contexts created after the binding is added.
    /// See also `ExecutionContext.name` and `worldName` parameter to
    /// `Page.addScriptToEvaluateOnNewDocument`.
    /// This parameter is mutually exclusive with `executionContextId`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_context_name: Option<String>,
}

/// Parameters for `Runtime.removeBinding`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RemoveBindingParams {
    pub name: String,
}

/// Parameters for `Runtime.getExceptionDetails`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetExceptionDetailsParams {
    /// The error object for which to resolve the exception details.
    pub error_object_id: RemoteObjectId,
}

/// Return type for `Runtime.getExceptionDetails`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetExceptionDetailsReturns {
    #[serde(default)]
    pub exception_details: Option<ExceptionDetails>,
}

/// Event payload for `Runtime.bindingCalled`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BindingCalledEvent {
    pub name: String,
    pub payload: String,
    /// Identifier of the context where the call was made.
    pub execution_context_id: ExecutionContextId,
}

/// Event payload for `Runtime.consoleAPICalled`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsoleAPICalledEvent {
    /// Type of the call.
    #[serde(rename = "type")]
    pub r#type: String,
    /// Call arguments.
    pub args: Vec<RemoteObject>,
    /// Identifier of the context where the call was made.
    pub execution_context_id: ExecutionContextId,
    /// Call timestamp.
    pub timestamp: Timestamp,
    /// Stack trace captured when the call was made. The async stack chain is automatically reported for
    /// the following call types: `assert`, `error`, `trace`, `warning`. For other types the async call
    /// chain can be retrieved using `Debugger.getStackTrace` and `stackTrace.parentId` field.
    #[serde(default)]
    pub stack_trace: Option<StackTrace>,
    /// Console context descriptor for calls on non-default console context (not console.*):
    /// 'anonymous#unique-logger-id' for call on unnamed context, 'name#unique-logger-id' for call
    /// on named context.
    #[serde(default)]
    pub context: Option<String>,
}

/// Event payload for `Runtime.exceptionRevoked`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionRevokedEvent {
    /// Reason describing why exception was revoked.
    pub reason: String,
    /// The id of revoked exception, as reported in `exceptionThrown`.
    pub exception_id: i64,
}

/// Event payload for `Runtime.exceptionThrown`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionThrownEvent {
    /// Timestamp of the exception.
    pub timestamp: Timestamp,
    pub exception_details: ExceptionDetails,
}

/// Event payload for `Runtime.executionContextCreated`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionContextCreatedEvent {
    /// A newly created execution context.
    pub context: ExecutionContextDescription,
}

/// Event payload for `Runtime.executionContextDestroyed`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionContextDestroyedEvent {
    /// Id of the destroyed context
    pub execution_context_id: ExecutionContextId,
    /// Unique Id of the destroyed context
    pub execution_context_unique_id: String,
}

/// Event payload for `Runtime.inspectRequested`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectRequestedEvent {
    pub object: RemoteObject,
    pub hints: Value,
    /// Identifier of the context where the call was made.
    #[serde(default)]
    pub execution_context_id: Option<ExecutionContextId>,
}

// ── Methods ──
//
// These are the typed method signatures for Runtime.* commands.
// Integration into CdpSession is done in pwright-cdp.

// Add handler to promise with given promise object id.
// async fn runtime_await_promise(&self, params: AwaitPromiseParams) -> Result<AwaitPromiseReturns>
// CDP: "Runtime.awaitPromise"

// Calls function with given declaration on the given object. Object group of the result is
// inherited from the target object.
// async fn runtime_call_function_on(&self, params: CallFunctionOnParams) -> Result<CallFunctionOnReturns>
// CDP: "Runtime.callFunctionOn"

// Compiles expression.
// async fn runtime_compile_script(&self, params: CompileScriptParams) -> Result<CompileScriptReturns>
// CDP: "Runtime.compileScript"

// Disables reporting of execution contexts creation.
// async fn runtime_disable(&self) -> Result<()>
// CDP: "Runtime.disable"

// Discards collected exceptions and console API calls.
// async fn runtime_discard_console_entries(&self) -> Result<()>
// CDP: "Runtime.discardConsoleEntries"

// Enables reporting of execution contexts creation by means of `executionContextCreated` event.
// When the reporting gets enabled the event will be sent immediately for each existing execution
// context.
// async fn runtime_enable(&self) -> Result<()>
// CDP: "Runtime.enable"

// Evaluates expression on global object.
// async fn runtime_evaluate(&self, params: EvaluateParams) -> Result<EvaluateReturns>
// CDP: "Runtime.evaluate"

// Returns the isolate id.
// async fn runtime_get_isolate_id(&self) -> Result<GetIsolateIdReturns>
// CDP: "Runtime.getIsolateId"

// Returns the JavaScript heap usage.
// It is the total usage of the corresponding isolate not scoped to a particular Runtime.
// async fn runtime_get_heap_usage(&self) -> Result<GetHeapUsageReturns>
// CDP: "Runtime.getHeapUsage"

// Returns properties of a given object. Object group of the result is inherited from the target
// object.
// async fn runtime_get_properties(&self, params: GetPropertiesParams) -> Result<GetPropertiesReturns>
// CDP: "Runtime.getProperties"

// Returns all let, const and class variables from global scope.
// async fn runtime_global_lexical_scope_names(&self, params: GlobalLexicalScopeNamesParams) -> Result<GlobalLexicalScopeNamesReturns>
// CDP: "Runtime.globalLexicalScopeNames"

// async fn runtime_query_objects(&self, params: QueryObjectsParams) -> Result<QueryObjectsReturns>
// CDP: "Runtime.queryObjects"

// Releases remote object with given id.
// async fn runtime_release_object(&self, params: ReleaseObjectParams) -> Result<()>
// CDP: "Runtime.releaseObject"

// Releases all remote objects that belong to a given group.
// async fn runtime_release_object_group(&self, params: ReleaseObjectGroupParams) -> Result<()>
// CDP: "Runtime.releaseObjectGroup"

// Tells inspected instance to run if it was waiting for debugger to attach.
// async fn runtime_run_if_waiting_for_debugger(&self) -> Result<()>
// CDP: "Runtime.runIfWaitingForDebugger"

// Runs script with given id in a given context.
// async fn runtime_run_script(&self, params: RunScriptParams) -> Result<RunScriptReturns>
// CDP: "Runtime.runScript"

// Enables or disables async call stacks tracking.
// async fn runtime_set_async_call_stack_depth(&self, params: SetAsyncCallStackDepthParams) -> Result<()>
// CDP: "Runtime.setAsyncCallStackDepth"

// async fn runtime_set_custom_object_formatter_enabled(&self, params: SetCustomObjectFormatterEnabledParams) -> Result<()>
// CDP: "Runtime.setCustomObjectFormatterEnabled"

// async fn runtime_set_max_call_stack_size_to_capture(&self, params: SetMaxCallStackSizeToCaptureParams) -> Result<()>
// CDP: "Runtime.setMaxCallStackSizeToCapture"

// Terminate current or next JavaScript execution.
// Will cancel the termination when the outer-most script execution ends.
// async fn runtime_terminate_execution(&self) -> Result<()>
// CDP: "Runtime.terminateExecution"

// If executionContextId is empty, adds binding with the given name on the
// global objects of all inspected contexts, including those created later,
// bindings survive reloads.
// Binding function takes exactly one argument, this argument should be string,
// in case of any other input, function throws an exception.
// Each binding function call produces Runtime.bindingCalled notification.
// async fn runtime_add_binding(&self, params: AddBindingParams) -> Result<()>
// CDP: "Runtime.addBinding"

// This method does not remove binding function from global object but
// unsubscribes current runtime agent from Runtime.bindingCalled notifications.
// async fn runtime_remove_binding(&self, params: RemoveBindingParams) -> Result<()>
// CDP: "Runtime.removeBinding"

// This method tries to lookup and populate exception details for a
// JavaScript Error object.
// Note that the stackTrace portion of the resulting exceptionDetails will
// only be populated if the Runtime domain was enabled at the time when the
// Error was thrown.
// async fn runtime_get_exception_details(&self, params: GetExceptionDetailsParams) -> Result<GetExceptionDetailsReturns>
// CDP: "Runtime.getExceptionDetails"
