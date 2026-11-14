//! Optional composable server bindings for the locator capability.
//!
//! Applications supply one service per command and choose their own backend,
//! executor, transport, and connection supervisor. This crate starts no task,
//! runtime, browser, or Node.js process.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use pwright_cap_locator::LOCATOR_CAPABILITY_ID;
use pwright_core::ProtocolVersion;
use pwright_server_wire::{
    ConcurrentServerFrameHandler, Either, ServerCapabilityInventory,
    ServerCapabilityInventoryError, ServerCapabilitySource, ServerFrameDisposition,
    ServerHandlerChainError,
};
use pwright_transport::{ServerHello, WireCapabilityId, WireFrame};

#[doc(hidden)]
#[rustfmt::skip]
mod generated_server;

pub use generated_server::*;

#[doc(hidden)]
#[rustfmt::skip]
mod generated_server_v2;

/// Generated locator 2.x handler composition.
pub mod v2 {
    pub use super::generated_server_v2::*;
}

/// Locator handler table selected before the first application frame is
/// decoded. Minor versions share the table for their negotiated major.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocatorServerMajor {
    V1,
    V2,
}

/// Exactly one locator handler table selected for an accepted connection.
/// The unselected table is not retained or polled.
pub enum SelectedLocatorServerHandlers<V1, V2> {
    V1(V1),
    V2(V2),
}

impl<V1, V2> ServerCapabilitySource for SelectedLocatorServerHandlers<V1, V2>
where
    V1: ServerCapabilitySource,
    V2: ServerCapabilitySource,
{
    fn register_capabilities(
        &self,
        inventory: &mut ServerCapabilityInventory,
    ) -> Result<(), ServerCapabilityInventoryError> {
        match self {
            Self::V1(handlers) => handlers.register_capabilities(inventory),
            Self::V2(handlers) => handlers.register_capabilities(inventory),
        }
    }
}

impl<Transport, V1, V2> ConcurrentServerFrameHandler<Transport>
    for SelectedLocatorServerHandlers<V1, V2>
where
    Transport: Send + Sync + 'static,
    V1: ConcurrentServerFrameHandler<Transport> + Send + Sync + 'static,
    V2: ConcurrentServerFrameHandler<Transport> + Send + Sync + 'static,
    V1::Event: Send + 'static,
    V1::Error: Send + 'static,
    V2::Event: Send + 'static,
    V2::Error: Send + 'static,
{
    type Event = Either<V1::Event, V2::Event>;
    type Error = ServerHandlerChainError<V1::Error, V2::Error>;
    type Future = Pin<
        Box<
            dyn Future<Output = Result<ServerFrameDisposition<Self::Event>, Self::Error>>
                + Send
                + 'static,
        >,
    >;

    fn handle_concurrently(&self, frame: WireFrame, transport: Arc<Transport>) -> Self::Future {
        match self {
            Self::V1(handlers) => {
                let future = handlers.handle_concurrently(frame, transport);
                Box::pin(async move {
                    future
                        .await
                        .map(|disposition| match disposition {
                            ServerFrameDisposition::Handled(event) => {
                                ServerFrameDisposition::Handled(Either::Left(event))
                            }
                            ServerFrameDisposition::Unhandled(frame) => {
                                ServerFrameDisposition::Unhandled(frame)
                            }
                        })
                        .map_err(ServerHandlerChainError::Left)
                })
            }
            Self::V2(handlers) => {
                let future = handlers.handle_concurrently(frame, transport);
                Box::pin(async move {
                    future
                        .await
                        .map(|disposition| match disposition {
                            ServerFrameDisposition::Handled(event) => {
                                ServerFrameDisposition::Handled(Either::Right(event))
                            }
                            ServerFrameDisposition::Unhandled(frame) => {
                                ServerFrameDisposition::Unhandled(frame)
                            }
                        })
                        .map_err(ServerHandlerChainError::Right)
                })
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnsupportedLocatorServerMajor {
    pub selected: ProtocolVersion,
}

impl Display for UnsupportedLocatorServerMajor {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "negotiated locator version {} has no installed handler table",
            self.selected
        )
    }
}

impl Error for UnsupportedLocatorServerMajor {}

/// Resolves the one locator major selected by a completed handshake.
///
/// `Ok(None)` means the connection did not negotiate locator. An unknown
/// future major fails closed instead of decoding it with the newest known
/// codec.
pub fn selected_locator_server_major(
    hello: &ServerHello,
) -> Result<Option<LocatorServerMajor>, UnsupportedLocatorServerMajor> {
    let id = WireCapabilityId::new(LOCATOR_CAPABILITY_ID.as_str(), 64)
        .expect("the static locator capability ID is valid");
    let Some(selected) = hello.enabled_capability_version(&id) else {
        return Ok(None);
    };
    match selected.major {
        1 => Ok(Some(LocatorServerMajor::V1)),
        2 => Ok(Some(LocatorServerMajor::V2)),
        _ => Err(UnsupportedLocatorServerMajor { selected }),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_browser_model::BROWSER_OBJECT_KIND_CATALOG;
    use pwright_codec_locator::{LocatorCodec, LocatorCodecLimits, LocatorV2Codec};
    use pwright_core::ProtocolVersion;
    use pwright_server_wire::ServerCapabilitySource;
    use pwright_transport::{
        FrameLimits, NegotiationLimits, ServerHello, VersionPolicy, WireCapability,
        WireCapabilityId, WireProtocol,
    };

    use super::{
        LocatorServerMajor, LocatorServerServices, LocatorServerServicesLeft,
        LocatorServerServicesRight, locator_server_handlers, selected_locator_server_major,
        v2::{
            LocatorV2ServerServices, LocatorV2ServerServicesLeft, LocatorV2ServerServicesRight,
            locator_v2_server_handlers,
        },
    };

    #[test]
    fn generated_chain_advertises_the_complete_locator_capability() {
        // Arrange
        let version = ProtocolVersion::new(1, 24);
        let frame_limits = FrameLimits::new(1_024, 0, 0, 1_024);
        let protocol = Arc::new(WireProtocol::new_exact_directional(
            LocatorCodec::new(LocatorCodecLimits::new(256)),
            version,
            frame_limits,
            frame_limits,
        ));
        let handlers = locator_server_handlers(
            protocol,
            LocatorServerServices {
                left: LocatorServerServicesLeft {
                    all_inner_texts: Arc::new(()),
                    all_text_contents: Arc::new(()),
                    blur: Arc::new(()),
                    click: Arc::new(()),
                    count: Arc::new(()),
                    fill: Arc::new(()),
                    focus: Arc::new(()),
                    get_attribute: Arc::new(()),
                    hover: Arc::new(()),
                    inner_html: Arc::new(()),
                    inner_text: Arc::new(()),
                },
                right: LocatorServerServicesRight {
                    input_value: Arc::new(()),
                    is_checked: Arc::new(()),
                    is_editable: Arc::new(()),
                    is_enabled: Arc::new(()),
                    is_visible: Arc::new(()),
                    press: Arc::new(()),
                    press_sequentially: Arc::new(()),
                    scroll_into_view_if_needed: Arc::new(()),
                    select_option: Arc::new(()),
                    set_checked: Arc::new(()),
                    text_content: Arc::new(()),
                    wait_for: Arc::new(()),
                },
            },
        );
        let capability =
            WireCapabilityId::new("pwright.locator", 64).expect("capability ID should build");

        // Act
        let manifest = handlers
            .capability_manifest_with_object_kinds(
                BROWSER_OBJECT_KIND_CATALOG,
                NegotiationLimits::new(8, 64),
            )
            .expect("the generated chain should register every locator command");

        // Assert
        assert_eq!(
            manifest.support(&capability),
            Some(VersionPolicy::compatible_through(version))
        );
    }

    #[test]
    fn generated_v2_chain_advertises_only_the_complete_locator_v2_track() {
        // Arrange
        let version = ProtocolVersion::new(2, 7);
        let frame_limits = FrameLimits::new(1_024, 0, 0, 1_024);
        let protocol = Arc::new(WireProtocol::new_exact_directional(
            LocatorV2Codec::new(LocatorCodecLimits::new(256)),
            version,
            frame_limits,
            frame_limits,
        ));
        let handlers = locator_v2_server_handlers(
            protocol,
            LocatorV2ServerServices {
                left: LocatorV2ServerServicesLeft {
                    all_inner_texts: Arc::new(()),
                    all_text_contents: Arc::new(()),
                    blur: Arc::new(()),
                    click: Arc::new(()),
                    count: Arc::new(()),
                    fill: Arc::new(()),
                    focus: Arc::new(()),
                    get_attribute: Arc::new(()),
                    hover: Arc::new(()),
                    inner_html: Arc::new(()),
                    inner_text: Arc::new(()),
                },
                right: LocatorV2ServerServicesRight {
                    input_value: Arc::new(()),
                    is_checked: Arc::new(()),
                    is_editable: Arc::new(()),
                    is_enabled: Arc::new(()),
                    is_visible: Arc::new(()),
                    press: Arc::new(()),
                    press_sequentially: Arc::new(()),
                    scroll_into_view_if_needed: Arc::new(()),
                    select_option: Arc::new(()),
                    set_checked: Arc::new(()),
                    text_content: Arc::new(()),
                    wait_for: Arc::new(()),
                },
            },
        );
        let capability =
            WireCapabilityId::new("pwright.locator", 64).expect("capability ID should build");

        // Act
        let manifest = handlers
            .capability_manifest_with_object_kinds(
                BROWSER_OBJECT_KIND_CATALOG,
                NegotiationLimits::new(8, 64),
            )
            .expect("the generated v2 chain should register every locator command");

        // Assert
        assert_eq!(
            manifest.support_major(&capability, 2),
            Some(VersionPolicy::compatible_through(version))
        );
        assert_eq!(manifest.support_major(&capability, 1), None);
    }

    #[test]
    fn accepted_hello_selects_exactly_one_locator_handler_major() {
        // Arrange
        let frames = FrameLimits::new(1_024, 0, 0, 1_024);
        let limits = NegotiationLimits::new(4, 64);
        let id = WireCapabilityId::new("pwright.locator", 64).expect("valid capability ID");
        let hello = |version| {
            ServerHello::new(
                ProtocolVersion::new(1, 0),
                frames,
                frames,
                vec![WireCapability::new(id.clone(), version)],
                limits,
            )
            .expect("hello should build")
        };

        // Act
        let v1 = selected_locator_server_major(&hello(ProtocolVersion::new(1, 17)));
        let v2 = selected_locator_server_major(&hello(ProtocolVersion::new(2, 0)));
        let unsupported = selected_locator_server_major(&hello(ProtocolVersion::new(3, 0)));

        // Assert
        assert_eq!(v1, Ok(Some(LocatorServerMajor::V1)));
        assert_eq!(v2, Ok(Some(LocatorServerMajor::V2)));
        assert_eq!(
            unsupported
                .expect_err("unknown major must fail closed")
                .selected,
            ProtocolVersion::new(3, 0)
        );
    }
}
