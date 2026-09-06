//! Handle-local value evaluation and bounded property enumeration.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use pwright_cap_evaluation::{EvaluationNumber, EvaluationValue};
use pwright_cap_handle_evaluation::{
    EvaluateHandleParams, HandleEvaluationErrorDetails, HandleEvaluationValue,
};
use pwright_core::{FailureKind, OperationError, Service};
use serde_json::Value;

use crate::handle_evaluation::{
    CdpEvaluatedHandle, CdpHandleEvaluationLimits, CdpHandleEvaluationRequest,
    CdpHandleEvaluationService, HandleInvocation, build_invocation, invalid_response,
    invalid_value, map_cdp_error, map_evaluation_error, parse_response as parse_handle_response,
};
use pwright_backend_cdp_evaluation::{CdpEvaluationService, EVALUATION_RUNTIME, parse_response};

const PROPERTY_SNAPSHOT_SOURCE: &str = r#"(target, maximum) => {
  'use strict';
  const result = Object.create(null);
  const object = Object(target);
  let count = 0;
  for (const name of Object.keys(object)) {
    const descriptor = Object.getOwnPropertyDescriptor(object, name);
    if (!descriptor || !Object.prototype.hasOwnProperty.call(descriptor, 'value')) continue;
    result[name] = descriptor.value;
    count += 1;
    if (count > maximum) break;
  }
  return result;
}"#;

#[derive(Clone)]
pub struct CdpHandleValueEvaluationService {
    evaluation: CdpEvaluationService,
    limits: CdpHandleEvaluationLimits,
}

impl CdpHandleValueEvaluationService {
    pub fn from_evaluation(
        evaluation: CdpEvaluationService,
        native_id_bytes: usize,
        preview_bytes: usize,
    ) -> Self {
        Self {
            limits: CdpHandleEvaluationLimits::new(
                evaluation.limits(),
                native_id_bytes,
                preview_bytes,
            ),
            evaluation,
        }
    }
}

impl Service<CdpHandleEvaluationRequest> for CdpHandleValueEvaluationService {
    type Response = EvaluationValue;
    type Error = OperationError<HandleEvaluationErrorDetails>;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, request: CdpHandleEvaluationRequest) -> Self::Future<'_> {
        let Some(unique_context_id) = self.evaluation.unique_context_id() else {
            return Box::pin(async {
                Err(invalid_value(
                    "Handle-local value evaluation requires an exact CDP execution context"
                        .to_owned(),
                ))
            });
        };
        let value_params = match request
            .params()
            .value_projection(self.limits.evaluation().value)
        {
            Ok(params) => params,
            Err(error) => return Box::pin(async move { Err(invalid_value(error.to_string())) }),
        };
        let lease = match self.evaluation.reserve_for_handle(&value_params) {
            Ok(lease) => lease,
            Err(error) => return Box::pin(async move { Err(map_evaluation_error(error)) }),
        };
        let invocation = build_invocation(&request, self.limits, true);
        drop(value_params);
        drop(request);
        Box::pin(async move {
            let _lease = lease;
            let arguments = match invocation? {
                HandleInvocation::Call { arguments } => arguments,
                HandleInvocation::Evaluate { .. } => {
                    return Err(invalid_response(
                        "Handle-local value invocation did not target an exact context",
                    ));
                }
            };
            let encoded_arguments = serde_json::to_vec(&arguments).map_err(|error| {
                invalid_response(&format!("could not encode Handle-local arguments: {error}"))
            })?;
            let generated_bytes = EVALUATION_RUNTIME
                .len()
                .saturating_add(encoded_arguments.len());
            if generated_bytes > self.limits.evaluation().generated_script_bytes {
                return Err(invalid_value(format!(
                    "generated Handle-local value invocation has {generated_bytes} bytes, limit is {}",
                    self.limits.evaluation().generated_script_bytes
                )));
            }
            let response = self
                .evaluation
                .shared_client()
                .runtime_call_function_by_value_in_unique_context(
                    unique_context_id,
                    EVALUATION_RUNTIME,
                    arguments,
                )
                .await
                .map_err(|error| map_cdp_error(error, self.limits.evaluation().error_bytes))?;
            parse_response(response, self.limits.evaluation())
                .map(|result| result.into_value())
                .map_err(map_evaluation_error)
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CdpHandlePropertyLimits {
    max_properties: usize,
    property_name_bytes: usize,
}

impl CdpHandlePropertyLimits {
    pub fn new(
        max_properties: usize,
        property_name_bytes: usize,
    ) -> Result<Self, CdpHandlePropertyLimitError> {
        if max_properties > u32::MAX as usize {
            return Err(CdpHandlePropertyLimitError::PropertyCountTooLarge {
                actual: max_properties,
                maximum: u32::MAX as usize,
            });
        }
        Ok(Self {
            max_properties,
            property_name_bytes,
        })
    }

    pub const fn max_properties(self) -> usize {
        self.max_properties
    }

    pub const fn property_name_bytes(self) -> usize {
        self.property_name_bytes
    }
}

impl Default for CdpHandlePropertyLimits {
    fn default() -> Self {
        Self::new(1024, 16 * 1024).expect("default Handle property limits are valid")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CdpHandlePropertyLimitError {
    PropertyCountTooLarge { actual: usize, maximum: usize },
}

impl Display for CdpHandlePropertyLimitError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::PropertyCountTooLarge { actual, maximum } => write!(
                formatter,
                "Handle property limit {actual} exceeds supported maximum {maximum}"
            ),
        }
    }
}

impl Error for CdpHandlePropertyLimitError {}

#[derive(Clone)]
pub struct CdpHandlePropertiesService {
    handle_evaluation: CdpHandleEvaluationService,
    limits: CdpHandleEvaluationLimits,
    property_limits: CdpHandlePropertyLimits,
}

impl CdpHandlePropertiesService {
    pub fn from_evaluation(
        evaluation: CdpEvaluationService,
        native_id_bytes: usize,
        preview_bytes: usize,
        property_limits: CdpHandlePropertyLimits,
    ) -> Self {
        let limits =
            CdpHandleEvaluationLimits::new(evaluation.limits(), native_id_bytes, preview_bytes);
        Self {
            handle_evaluation: CdpHandleEvaluationService::from_evaluation(
                evaluation,
                native_id_bytes,
                preview_bytes,
            ),
            limits,
            property_limits,
        }
    }

    pub async fn call_on_handle(
        &self,
        target_id: pwright_core::ObjectId,
        target: CdpEvaluatedHandle,
    ) -> Result<Vec<(String, CdpEvaluatedHandle)>, OperationError<HandleEvaluationErrorDetails>>
    {
        if target.object_id().is_none() {
            return Ok(Vec::new());
        }
        let maximum = EvaluationNumber::new(self.property_limits.max_properties as f64)
            .map_err(|error| invalid_value(error.to_string()))?;
        let params = EvaluateHandleParams::function(
            PROPERTY_SNAPSHOT_SOURCE,
            HandleEvaluationValue::Number(maximum),
        );
        let temporary = self
            .handle_evaluation
            .call(CdpHandleEvaluationRequest::new_on_handle(
                params,
                (target_id, target),
                Vec::new(),
            ))
            .await?;
        let Some(temporary_id) = temporary.object_id().map(str::to_owned) else {
            return Err(invalid_response(
                "Handle property snapshot did not return a retained object",
            ));
        };
        let client = temporary.client();
        let response = client.runtime_get_properties(&temporary_id).await;
        let response = match response {
            Ok(response) => response,
            Err(error) => {
                let primary = map_cdp_error(error, self.limits.evaluation().error_bytes);
                return Err(with_cleanup(primary, temporary.release().await.err()));
            }
        };
        let parsed = parse_properties(
            Arc::clone(&client),
            &response,
            self.limits,
            self.property_limits,
        );
        match parsed {
            Ok(properties) => {
                if let Err(cleanup) = temporary.release().await {
                    let cleanup_error =
                        map_cdp_error(cleanup, self.limits.evaluation().error_bytes);
                    let release_error = release_handles(&properties).await.err();
                    return Err(with_cleanup(cleanup_error, release_error));
                }
                Ok(properties)
            }
            Err(primary) => {
                let raw_cleanup = release_remote_values(Arc::clone(&client), &response)
                    .await
                    .err();
                let temporary_cleanup = temporary.release().await.err();
                let primary = with_cleanup(primary, raw_cleanup);
                Err(with_cleanup(primary, temporary_cleanup))
            }
        }
    }
}

fn parse_properties(
    client: Arc<dyn pwright_cdp::CdpClient>,
    response: &Value,
    limits: CdpHandleEvaluationLimits,
    property_limits: CdpHandlePropertyLimits,
) -> Result<Vec<(String, CdpEvaluatedHandle)>, OperationError<HandleEvaluationErrorDetails>> {
    let descriptors = response
        .get("result")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid_response("Runtime.getProperties omitted its result array"))?;
    if descriptors.len() > property_limits.max_properties {
        return Err(OperationError::new(
            FailureKind::ResourceExhausted,
            format!(
                "Handle has more than {} own enumerable data properties",
                property_limits.max_properties
            ),
            Some(HandleEvaluationErrorDetails::InvalidValue(
                "Handle property-count limit exceeded".to_owned(),
            )),
        ));
    }
    let mut names = BTreeSet::new();
    let mut properties = Vec::with_capacity(descriptors.len());
    for descriptor in descriptors {
        if descriptor.get("enumerable").and_then(Value::as_bool) != Some(true) {
            continue;
        }
        let name = descriptor
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid_response("CDP property descriptor omitted its name"))?;
        if name.len() > property_limits.property_name_bytes {
            return Err(invalid_response(
                "CDP property descriptor name exceeds the configured byte limit",
            ));
        }
        if !names.insert(name.to_owned()) {
            return Err(invalid_response(
                "CDP property descriptor list contains a duplicate name",
            ));
        }
        let remote = descriptor
            .get("value")
            .ok_or_else(|| invalid_response("enumerable data property omitted its value"))?;
        let native = parse_handle_response(
            Arc::clone(&client),
            serde_json::json!({ "result": remote }),
            limits,
        )?;
        properties.push((name.to_owned(), native));
    }
    Ok(properties)
}

async fn release_remote_values(
    client: Arc<dyn pwright_cdp::CdpClient>,
    response: &Value,
) -> Result<(), pwright_cdp::connection::CdpError> {
    let Some(descriptors) = response.get("result").and_then(Value::as_array) else {
        return Ok(());
    };
    let mut first_error = None;
    for object_id in descriptors.iter().filter_map(|descriptor| {
        descriptor
            .get("value")
            .and_then(|value| value.get("objectId"))
            .and_then(Value::as_str)
    }) {
        if let Err(error) = client.runtime_release_object(object_id).await
            && first_error.is_none()
        {
            first_error = Some(error);
        }
    }
    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

async fn release_handles(
    properties: &[(String, CdpEvaluatedHandle)],
) -> Result<(), pwright_cdp::connection::CdpError> {
    let mut first_error = None;
    for (_, handle) in properties {
        if let Err(error) = handle.release().await
            && first_error.is_none()
        {
            first_error = Some(error);
        }
    }
    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn with_cleanup<E: Display>(
    primary: OperationError<HandleEvaluationErrorDetails>,
    cleanup: Option<E>,
) -> OperationError<HandleEvaluationErrorDetails> {
    let Some(cleanup) = cleanup else {
        return primary;
    };
    let cleanup = cleanup.to_string();
    OperationError::new(
        FailureKind::Cleanup,
        format!("{}; cleanup failed: {cleanup}", primary.message()),
        Some(HandleEvaluationErrorDetails::Cleanup {
            primary: primary.message().to_owned(),
            cleanup,
        }),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_cap_evaluation::EvaluationValue;
    use pwright_cap_handle_evaluation::{EvaluateHandleParams, HandleEvaluationValue};
    use pwright_core::{FailureKind, ObjectId, Service};
    use pwright_fake::FakeCdpClient;

    use super::{
        CdpHandlePropertiesService, CdpHandlePropertyLimits, CdpHandleValueEvaluationService,
    };
    use pwright_backend_cdp_evaluation::{CdpEvaluationLimits, CdpEvaluationService};

    use crate::{CdpHandleEvaluationLimits, CdpHandleEvaluationRequest};

    fn object_handle(fake: Arc<FakeCdpClient>, object_id: &str) -> crate::CdpEvaluatedHandle {
        let client: Arc<dyn pwright_cdp::CdpClient> = fake;
        crate::handle_evaluation::parse_response(
            client,
            serde_json::json!({
                "result": {
                    "type": "object",
                    "objectId": object_id,
                    "description": "Object"
                }
            }),
            CdpHandleEvaluationLimits::default(),
        )
        .unwrap()
    }

    fn primitive_handle(fake: Arc<FakeCdpClient>) -> crate::CdpEvaluatedHandle {
        let client: Arc<dyn pwright_cdp::CdpClient> = fake;
        crate::handle_evaluation::parse_response(
            client,
            serde_json::json!({"result": {"type": "number", "value": 7}}),
            CdpHandleEvaluationLimits::default(),
        )
        .unwrap()
    }

    fn evaluation(fake: Arc<FakeCdpClient>) -> CdpEvaluationService {
        let client: Arc<dyn pwright_cdp::CdpClient> = fake;
        CdpEvaluationService::with_limits_in_unique_context(
            client,
            "world-1",
            CdpEvaluationLimits::default(),
        )
    }

    #[tokio::test]
    async fn handle_value_evaluation_passes_target_first_and_parses_bounded_value() {
        // Arrange
        let fake = Arc::new(FakeCdpClient::from_html("<main></main>"));
        fake.set_evaluate_response(serde_json::json!({
            "result": {"value": {"ok": true, "value": {"t": "str", "v": "done"}}}
        }));
        let target = object_handle(Arc::clone(&fake), "target-1");
        let service = CdpHandleValueEvaluationService::from_evaluation(
            evaluation(Arc::clone(&fake)),
            4096,
            4096,
        );
        let request = CdpHandleEvaluationRequest::new_on_handle(
            EvaluateHandleParams::function(
                "(target, suffix) => target.name + suffix",
                HandleEvaluationValue::String("!".to_owned()),
            ),
            (ObjectId::new(9), target),
            Vec::new(),
        );

        // Act
        let value = service.call(request).await.unwrap();

        // Assert
        assert_eq!(value, EvaluationValue::String("done".to_owned()));
        let calls = fake.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "Runtime.callFunctionOn");
        assert_eq!(calls[0].1[0]["returnByValue"], true);
        assert_eq!(calls[0].1[0]["arguments"][0]["value"]["receiverSlot"], 0);
        assert_eq!(calls[0].1[0]["arguments"][1]["objectId"], "target-1");
    }

    #[tokio::test]
    async fn get_properties_returns_ordered_data_handles_and_releases_snapshot() {
        // Arrange
        let fake = Arc::new(FakeCdpClient::from_html("<main></main>"));
        fake.set_evaluate_responses([
            serde_json::json!({
                "result": {
                    "type": "object",
                    "objectId": "snapshot-1",
                    "description": "Object"
                }
            }),
            serde_json::json!({
                "result": [
                    {
                        "name": "first",
                        "enumerable": true,
                        "value": {"type": "number", "value": 1}
                    },
                    {
                        "name": "second",
                        "enumerable": true,
                        "value": {
                            "type": "object",
                            "subtype": "node",
                            "objectId": "node-2",
                            "description": "button"
                        }
                    }
                ]
            }),
        ]);
        let target = object_handle(Arc::clone(&fake), "target-1");
        let service = CdpHandlePropertiesService::from_evaluation(
            evaluation(Arc::clone(&fake)),
            4096,
            4096,
            CdpHandlePropertyLimits::new(2, 32).unwrap(),
        );

        // Act
        let properties = service
            .call_on_handle(ObjectId::new(9), target)
            .await
            .unwrap();

        // Assert
        assert_eq!(properties.len(), 2);
        assert_eq!(properties[0].0, "first");
        assert_eq!(properties[1].0, "second");
        assert_eq!(properties[1].1.kind(), crate::CdpHandleKind::Element);
        assert_eq!(
            fake.method_names(),
            [
                "Runtime.callFunctionOn",
                "Runtime.getProperties",
                "Runtime.releaseObject"
            ]
        );
        assert_eq!(fake.calls()[2].1[0], serde_json::json!("snapshot-1"));
    }

    #[tokio::test]
    async fn primitive_get_properties_is_empty_without_native_io() {
        // Arrange
        let fake = Arc::new(FakeCdpClient::from_html("<main></main>"));
        let target = primitive_handle(Arc::clone(&fake));
        let service = CdpHandlePropertiesService::from_evaluation(
            evaluation(Arc::clone(&fake)),
            4096,
            4096,
            CdpHandlePropertyLimits::new(2, 32).unwrap(),
        );

        // Act
        let properties = service
            .call_on_handle(ObjectId::new(9), target)
            .await
            .unwrap();

        // Assert
        assert!(properties.is_empty());
        assert!(fake.calls().is_empty());
    }

    #[tokio::test]
    async fn property_overflow_releases_every_native_value_and_snapshot() {
        // Arrange
        let fake = Arc::new(FakeCdpClient::from_html("<main></main>"));
        fake.set_evaluate_responses([
            serde_json::json!({
                "result": {"type": "object", "objectId": "snapshot-1"}
            }),
            serde_json::json!({
                "result": [
                    {"name": "one", "enumerable": true, "value": {"type": "object", "objectId": "value-1"}},
                    {"name": "two", "enumerable": true, "value": {"type": "object", "objectId": "value-2"}}
                ]
            }),
        ]);
        let target = object_handle(Arc::clone(&fake), "target-1");
        let service = CdpHandlePropertiesService::from_evaluation(
            evaluation(Arc::clone(&fake)),
            4096,
            4096,
            CdpHandlePropertyLimits::new(1, 32).unwrap(),
        );

        // Act
        let error = service
            .call_on_handle(ObjectId::new(9), target)
            .await
            .unwrap_err();

        // Assert
        assert_eq!(error.kind(), FailureKind::ResourceExhausted);
        assert_eq!(
            fake.method_names(),
            [
                "Runtime.callFunctionOn",
                "Runtime.getProperties",
                "Runtime.releaseObject",
                "Runtime.releaseObject",
                "Runtime.releaseObject"
            ]
        );
        assert_eq!(fake.calls()[2].1[0], serde_json::json!("value-1"));
        assert_eq!(fake.calls()[3].1[0], serde_json::json!("value-2"));
        assert_eq!(fake.calls()[4].1[0], serde_json::json!("snapshot-1"));
    }
}
