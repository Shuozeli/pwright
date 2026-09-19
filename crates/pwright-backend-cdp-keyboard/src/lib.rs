//! Optional typed Keyboard backend with caller-owned scheduling and Page state.
use pwright_backend_cdp_input::{
    CdpInputCoordinator, CdpInputTransaction, CdpKeyDownInterceptor, KeyTokens, press_keys,
    type_text,
};
use pwright_cap_keyboard::*;
use pwright_cdp::{CdpClient, connection::CdpError};
use pwright_core::{
    CancellationReason, CommandCancellation, FailureKind, OperationContext, OperationError,
    ProductionResourceError, ProductionResourceLedger, Service,
};
use pwright_input_model::InputText;
use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

mod errors;
mod manifest;
mod profile;
use errors::{key_error, native_error, resource_error};
pub use manifest::*;
use profile::Budget;
pub use profile::{
    CdpKeyboardProductionProfile, CdpKeyboardProfileError, KEYBOARD_ERROR_MAX_BYTES,
};

pub type KeyboardResult = Result<(), OperationError<KeyboardErrorDetails>>;
pub type KeyboardFuture<'a> = Pin<Box<dyn Future<Output = KeyboardResult> + Send + 'a>>;

/// The embedding host supplies its clock/executor. This leaf never starts one.
pub trait CdpKeyboardDelay: Send + Sync {
    fn wait(
        &self,
        duration: Duration,
    ) -> Pin<Box<dyn Future<Output = Result<(), CdpError>> + Send + '_>>;
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyboardPlatform {
    Mac,
    Other,
}
impl KeyboardPlatform {
    const fn is_mac(self) -> bool {
        matches!(self, Self::Mac)
    }
}

#[derive(Clone)]
pub struct CdpKeyboardServiceFactory {
    budget: Budget,
    delay: Arc<dyn CdpKeyboardDelay>,
    platform: KeyboardPlatform,
}
impl CdpKeyboardServiceFactory {
    pub fn new_with_production_resources(
        profile: CdpKeyboardProductionProfile,
        resources: &ProductionResourceLedger,
        delay: Arc<dyn CdpKeyboardDelay>,
        platform: KeyboardPlatform,
    ) -> Result<Self, ProductionResourceError> {
        Ok(Self {
            budget: Budget::new(profile, resources)?,
            delay,
            platform,
        })
    }
    pub fn active_operations(&self) -> usize {
        self.budget.active()
    }
    pub fn profile(&self) -> CdpKeyboardProductionProfile {
        self.budget.profile
    }
    /// The caller must inject the exact coordinator shared with this Page's
    /// Locator/Mouse routes. No private replacement input state is constructed.
    pub fn bind(
        &self,
        client: Arc<dyn CdpClient>,
        input: CdpInputCoordinator,
    ) -> CdpKeyboardServices {
        self.bind_with_key_down_interceptor(client, input, None)
    }
    /// Select a separately admitted/retired provider on each transaction, without
    /// installing it in shared Page state or creating a native-client wrapper.
    /// Selected interception requires the interleaved input policy.
    pub fn bind_with_key_down_interceptor(
        &self,
        client: Arc<dyn CdpClient>,
        input: CdpInputCoordinator,
        interceptor: Option<Arc<dyn CdpKeyDownInterceptor>>,
    ) -> CdpKeyboardServices {
        let page = Arc::new(PageInput {
            client,
            input,
            budget: self.budget.clone(),
            delay: self.delay.clone(),
            platform: self.platform,
            interceptor,
        });
        CdpKeyboardServices {
            keyboard_down: CdpKeyboardDownService(page.clone()),
            keyboard_up: CdpKeyboardUpService(page.clone()),
            keyboard_press: CdpKeyboardPressService(page.clone()),
            keyboard_insert_text: CdpKeyboardInsertTextService(page.clone()),
            keyboard_type: CdpKeyboardTypeService(page),
        }
    }
}

struct PageInput {
    client: Arc<dyn CdpClient>,
    input: CdpInputCoordinator,
    budget: Budget,
    delay: Arc<dyn CdpKeyboardDelay>,
    platform: KeyboardPlatform,
    interceptor: Option<Arc<dyn CdpKeyDownInterceptor>>,
}
impl PageInput {
    async fn wait(&self, delay: Option<Duration>) -> Result<(), CdpError> {
        if let Some(delay) = delay.filter(|delay| !delay.is_zero()) {
            self.delay.wait(delay).await?;
        }
        Ok(())
    }
    async fn down(
        &self,
        transaction: &mut CdpInputTransaction,
        params: &DownParams,
    ) -> KeyboardResult {
        transaction
            .key_down_named(params.key.as_str(), self.platform.is_mac())
            .await
            .map_err(key_error)
    }
    async fn up(&self, transaction: &mut CdpInputTransaction, params: &UpParams) -> KeyboardResult {
        transaction
            .key_up_named(params.key.as_str(), self.platform.is_mac())
            .await
            .map_err(key_error)
    }
    async fn press(
        &self,
        transaction: &mut CdpInputTransaction,
        params: &PressParams,
    ) -> KeyboardResult {
        let keys = KeyTokens::parse_raw_keyboard(params.key.as_str())
            .map_err(|_| resource_error("Keyboard chord exceeds the 16-token limit"))?;
        press_keys(transaction, &keys, self.platform.is_mac(), || {
            self.wait(params.delay)
        })
        .await
        .map_err(key_error)
    }
    async fn insert(
        &self,
        transaction: &mut CdpInputTransaction,
        params: &InsertTextParams,
    ) -> KeyboardResult {
        transaction
            .insert_text(
                InputText::try_from(params.text.as_str())
                    .expect("portable text enforces input bound"),
            )
            .await
            .map_err(native_error)
    }
    async fn type_text(
        &self,
        transaction: &mut CdpInputTransaction,
        params: &TypeParams,
    ) -> KeyboardResult {
        type_text(
            transaction,
            InputText::try_from(params.text.as_str()).expect("portable text enforces input bound"),
            self.platform.is_mac(),
            || self.wait(params.delay),
        )
        .await
        .map_err(native_error)
    }
}

/// One optional service per command, all sharing the same injected Page state.
#[derive(Clone)]
pub struct CdpKeyboardServices {
    pub keyboard_down: CdpKeyboardDownService,
    pub keyboard_up: CdpKeyboardUpService,
    pub keyboard_press: CdpKeyboardPressService,
    pub keyboard_insert_text: CdpKeyboardInsertTextService,
    pub keyboard_type: CdpKeyboardTypeService,
}
impl CdpKeyboardServices {
    /// Shared Page binding allocation payload, including the Arc counters.
    /// Inline service handles are owned/accounted by the embedding route.
    /// Allocator metadata, the injected client and input coordinator are excluded.
    pub const fn shared_binding_bytes() -> usize {
        std::mem::size_of::<PageInput>() + 2 * std::mem::size_of::<usize>()
    }
}
macro_rules! keyboard_service {
    ($service:ident, $command:ty, $params:ty, $argument:ident, $execute:ident) => {
        #[derive(Clone)]
        pub struct $service(Arc<PageInput>);
        impl $service {
            pub fn shares_input_state_with(&self, input: &CdpInputCoordinator) -> bool {
                self.0.input.shares_state_with(input)
            }
        }
        impl Service<$params> for $service {
            type Response = ();
            type Error = OperationError<KeyboardErrorDetails>;
            type Future<'a> = KeyboardFuture<'a>;
            fn call(&self, params: $params) -> Self::Future<'_> {
                let capacity = params.$argument.capacity();
                let admitted = match self.0.budget.reserve(params, capacity) {
                    Ok(admitted) => admitted,
                    Err(error) => return Box::pin(async move { Err(error) }),
                };
                Box::pin(async move {
                    let mut transaction = self
                        .0
                        .input
                        .begin_transaction(self.0.client.clone(), None)
                        .await
                        .map_err(native_error)?;
                    if let Some(interceptor) = &self.0.interceptor {
                        transaction
                            .set_key_down_interceptor(interceptor.clone())
                            .map_err(native_error)?;
                    }
                    let outcome = self.0.$execute(&mut transaction, &admitted.params).await;
                    let cleanup = transaction.finish().await;
                    drop(admitted);
                    if let Err(error) = cleanup {
                        return Err(native_error(error).reclassify(FailureKind::Cleanup));
                    }
                    outcome
                })
            }
        }
        impl CommandCancellation<$command> for $service {
            type Future<'a> = KeyboardFuture<'a>;
            fn cancel(
                &self,
                _context: OperationContext,
                _reason: CancellationReason,
            ) -> Self::Future<'_> {
                // Policy-selected key recovery is not a native-call retirement receipt.
                Box::pin(async move {
                    self.0
                        .input
                        .cleanup_pending(self.0.client.as_ref())
                        .await
                        .map_err(native_error)
                })
            }
        }
        impl pwright_browser_backend::NativeBackendAccess<ChromiumCdpKeyboardBackend> for $service {
            type Native = dyn CdpClient;
            fn native_backend(&self) -> &Self::Native {
                self.0.client.as_ref()
            }
        }
    };
}
keyboard_service!(CdpKeyboardDownService, KeyboardDown, DownParams, key, down);
keyboard_service!(CdpKeyboardUpService, KeyboardUp, UpParams, key, up);
keyboard_service!(
    CdpKeyboardPressService,
    KeyboardPress,
    PressParams,
    key,
    press
);
keyboard_service!(
    CdpKeyboardInsertTextService,
    KeyboardInsertText,
    InsertTextParams,
    text,
    insert
);
keyboard_service!(
    CdpKeyboardTypeService,
    KeyboardType,
    TypeParams,
    text,
    type_text
);

#[cfg(test)]
mod tests;
