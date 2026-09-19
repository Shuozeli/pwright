use super::*;
use pwright_backend_cdp_input::type_text;
use pwright_cap_locator::{INPUT_TEXT_MAX_BYTES, InputText, PressSequentiallyParams};

impl Service<PressSequentiallyParams> for CdpLocatorService {
    type Response = ();
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, params: PressSequentiallyParams) -> Self::Future<'_> {
        // Reject retained capacity before boxing even without a production ledger.
        if params.text.capacity() > INPUT_TEXT_MAX_BYTES {
            drop(params);
            return Box::pin(async {
                Err(selector_resource_error(
                    "sequential text capacity exceeds 1048576 bytes",
                ))
            });
        }
        let resources = match self.reserve_locator_action(&params.selector, Some(&params.text)) {
            Ok(resources) => resources,
            Err(error) => {
                drop(params);
                return Box::pin(async move { Err(error) });
            }
        };
        Box::pin(async move {
            let _resources = resources;
            let selector_label = params.selector.to_string();
            let script = press_focus_script(&params.selector)?;
            self.ensure_selector_runtime().await?;
            let mut transaction = self.keyboard_transaction().await?;
            let action = async {
                let is_mac = loop {
                    self.runtime_counters
                        .focus_evaluations
                        .fetch_add(1, Ordering::Relaxed);
                    let response = self.evaluate(&script).await.map_err(map_cdp_error)?;
                    match parse_press_focus_attempt(response, &selector_label)? {
                        PressFocusAttempt::Done { is_mac } => break is_mac,
                        PressFocusAttempt::Retry => tokio::time::sleep(self.retry_delay).await,
                        PressFocusAttempt::RuntimeMissing => {
                            self.runtime_counters
                                .missing_after_navigation
                                .fetch_add(1, Ordering::Relaxed);
                            self.install_selector_runtime().await?;
                        }
                    }
                };
                self.handoff_keyboard_lifetime()?;
                type_text(
                    &mut transaction,
                    InputText::try_from(params.text.as_str()).expect("capacity prevalidated"),
                    is_mac,
                    || async {
                        if let Some(delay) = params.delay.filter(|delay| !delay.is_zero()) {
                            tokio::time::sleep(delay).await;
                        }
                        Ok(())
                    },
                )
                .await
                .map_err(map_cdp_error)
            }
            .await;
            transaction
                .finish()
                .await
                .map_err(|error| map_cdp_error(error).reclassify(FailureKind::Cleanup))?;
            // No navigation barrier and no retry after partial text dispatch.
            action
        })
    }
}

macro_rules! cancellation {
    ($command:ty) => {
        impl CommandCancellation<$command> for CdpLocatorService {
            type Future<'a>
                = Pin<
                Box<
                    dyn Future<Output = Result<(), OperationError<LocatorErrorDetails>>>
                        + Send
                        + 'a,
                >,
            >
            where
                Self: 'a;
            fn cancel(
                &self,
                _context: OperationContext,
                _reason: CancellationReason,
            ) -> Self::Future<'_> {
                Box::pin(async move {
                    self.input
                        .cleanup_pending(self.client.as_ref())
                        .await
                        .map_err(map_cdp_error)
                })
            }
        }
    };
}
cancellation!(pwright_cap_locator::PressSequentially);
cancellation!(locator_v2::PressSequentially);

#[cfg(test)]
mod tests {
    use super::*;
    use pwright_cap_locator::PressSequentially;
    use pwright_core::{CallId, CommandSpec, ObjectId};
    use pwright_fake::FakeCdpClient;
    use serde_json::json;
    use std::task::{Context, Waker};

    fn prepared() -> (Arc<FakeCdpClient>, CdpLocatorService) {
        let client = Arc::new(FakeCdpClient::from_html("<input>"));
        client.set_evaluate_responses([
            json!({"result": {"value": true}}),
            json!({"result": {"value": {"status": "done", "isMac": false}}}),
        ]);
        let service = CdpLocatorService::new(client.clone());
        (client, service)
    }

    #[tokio::test]
    async fn strict_focus_precedes_literal_input_without_navigation_preparation() {
        // Arrange
        let (client, service) = prepared();
        // Act
        service
            .call(PressSequentiallyParams::new(
                Selector::css("input").unwrap(),
                "a中",
                None,
            ))
            .await
            .unwrap();
        // Assert
        assert_eq!(
            client.method_names(),
            [
                "Runtime.evaluate",
                "Runtime.evaluate",
                "Input.dispatchKeyEvent",
                "Input.dispatchKeyEvent",
                "Input.insertText"
            ]
        );
        let calls = client.calls();
        let focus = calls[1].1[0].as_str().unwrap();
        assert!(focus.contains("element.setSelectionRange(0, 0)"));
        assert!(!focus.contains("scrollIntoView"));
        assert_eq!(calls[4].1[0], "中");
    }

    #[tokio::test]
    async fn excess_text_capacity_is_rejected_without_browser_io_or_ledger() {
        // Arrange
        let (client, service) = prepared();
        let params = PressSequentiallyParams::new(
            Selector::test_id("input"),
            String::with_capacity(INPUT_TEXT_MAX_BYTES + 1),
            None,
        );
        // Act
        let outcome = service.call(params).await;
        // Assert
        assert!(outcome.is_err());
        assert!(client.calls().is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn cancelled_sequential_delay_uses_the_typed_cleanup_hook_without_retyping() {
        // Arrange
        let (client, service) = prepared();
        let mut operation = service.call(PressSequentiallyParams::new(
            Selector::css("input").unwrap(),
            "ab",
            Some(Duration::from_secs(30)),
        ));
        let pending = operation
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()));
        assert!(pending.is_pending());
        drop(operation);
        let context = OperationContext::new(
            CallId::new(1),
            ObjectId::new(2),
            PressSequentially::METHOD,
            None,
        );
        // Act
        let result = <CdpLocatorService as CommandCancellation<PressSequentially>>::cancel(
            &service,
            context,
            CancellationReason::DeadlineExceeded,
        )
        .await;
        // Assert
        assert!(result.is_ok());
        let calls = client.calls();
        assert_eq!(calls.len(), 4);
        assert_eq!(calls[2].1[0]["key"], "a");
        assert_eq!(calls[3].1[0]["type"], "keyUp");
        assert_eq!(calls[3].1[0]["key"], "a");
    }
}
