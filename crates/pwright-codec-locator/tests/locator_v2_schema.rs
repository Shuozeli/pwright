use std::time::Duration;

use pwright_cap_locator::{
    Blur, BlurParams, ClickOptionsV2, ClickParamsV2, Count, CountParams, Focus, FocusParams,
    KeyboardModifier, KeyboardModifiers, MouseButton, Position, Press, PressKey, PressOptions,
    PressParams, ScrollIntoViewIfNeeded, ScrollIntoViewIfNeededParams, ScrollMode, SelectOption,
    SelectOptionOptions, SelectOptionParams, SelectOptionValue, Selector, SetChecked,
    SetCheckedOptions, SetCheckedParams, v2 as locator_v2,
};
use pwright_codec_binary::{
    BinaryDecode, BinaryEncode, BinaryReader, BinaryTypeKind, BinaryValueContext,
    BinaryValueLimits, BinaryWriter,
};
use pwright_codec_locator::generated_schema_v2::{
    PwrightCommonDuration, PwrightLocatorClickOptions, PwrightLocatorKeyboardModifier,
    PwrightLocatorKeyboardModifiers, PwrightLocatorMouseButton, PwrightLocatorPoint,
    PwrightLocatorScrollMode, PwrightLocatorSetCheckedOptions,
};
use pwright_codec_locator::{
    LOCATOR_SCHEMA, LOCATOR_SCHEMA_CATALOG, LOCATOR_V2_SCHEMA, LOCATOR_V2_SCHEMA_CATALOG,
    LocatorCodec, LocatorCodecError, LocatorCodecLimits, LocatorV2Codec,
};
use pwright_core::{CallId, CallMetadata, Command, CommandResult, ObjectId, ProtocolVersion};
use pwright_transport::{FrameLimits, WireCodec, WireFrame};

fn limits() -> BinaryValueLimits {
    BinaryValueLimits::new(64, 0, 16, 16, 64)
}

fn empty_options() -> PwrightLocatorClickOptions {
    PwrightLocatorClickOptions {
        button: None,
        click_count: None,
        delay: None,
        force: None,
        modifiers: None,
        no_wait_after: None,
        position: None,
        scroll: None,
        steps: None,
        trial: None,
    }
}

fn encode(value: &PwrightLocatorClickOptions) -> Vec<u8> {
    let mut writer = BinaryWriter::new();
    let mut context = BinaryValueContext::new(limits());
    value
        .encode_binary(&mut writer, &mut context)
        .expect("locator v2 Click options should encode");
    writer.into_bytes()
}

fn decode(bytes: &[u8]) -> PwrightLocatorClickOptions {
    let mut reader = BinaryReader::new(bytes);
    let mut context = BinaryValueContext::new(limits());
    let value = PwrightLocatorClickOptions::decode_binary(&mut reader, &mut context)
        .expect("locator v2 Click options should decode");
    reader.finish().expect("the value should consume its input");
    value
}

fn encode_set_checked_options(value: &PwrightLocatorSetCheckedOptions) -> Vec<u8> {
    let mut writer = BinaryWriter::new();
    let mut context = BinaryValueContext::new(limits());
    value
        .encode_binary(&mut writer, &mut context)
        .expect("SetChecked options should encode");
    writer.into_bytes()
}

fn decode_set_checked_options(bytes: &[u8]) -> PwrightLocatorSetCheckedOptions {
    let mut reader = BinaryReader::new(bytes);
    let mut context = BinaryValueContext::new(limits());
    let value = PwrightLocatorSetCheckedOptions::decode_binary(&mut reader, &mut context)
        .expect("SetChecked options should decode");
    reader.finish().expect("the value should consume its input");
    value
}

fn field_tags(bytes: &[u8]) -> Vec<u16> {
    let body_len = usize::try_from(u32::from_be_bytes(
        bytes[..4].try_into().expect("record length prefix"),
    ))
    .expect("u32 record length should fit usize");
    assert_eq!(body_len, bytes.len() - 4);
    let mut offset = 4;
    let mut tags = Vec::new();
    while offset < bytes.len() {
        tags.push(u16::from_be_bytes(
            bytes[offset..offset + 2].try_into().expect("field tag"),
        ));
        let value_len = usize::try_from(u32::from_be_bytes(
            bytes[offset + 2..offset + 6]
                .try_into()
                .expect("field length"),
        ))
        .expect("u32 field length should fit usize");
        offset += 6 + value_len;
    }
    assert_eq!(offset, bytes.len());
    tags
}

#[test]
fn empty_locator_v2_click_options_are_the_exact_default_record() {
    // Arrange
    let options = empty_options();

    // Act
    let bytes = encode(&options);
    let decoded = decode(&bytes);

    // Assert
    assert_eq!(bytes, [0, 0, 0, 0]);
    assert_eq!(decoded, options);
}

#[test]
fn locator_v2_click_options_round_trip_all_playwright_fields_and_skip_future_tags() {
    // Arrange
    let options = PwrightLocatorClickOptions {
        button: Some(PwrightLocatorMouseButton::Right),
        click_count: Some(2),
        delay: Some(PwrightCommonDuration {
            seconds: 0,
            nanoseconds: 500_000,
        }),
        force: Some(false),
        modifiers: Some(PwrightLocatorKeyboardModifiers {
            values: vec![
                PwrightLocatorKeyboardModifier::ControlOrMeta,
                PwrightLocatorKeyboardModifier::Shift,
            ],
        }),
        no_wait_after: Some(true),
        position: Some(PwrightLocatorPoint { x: 1.5, y: -2.0 }),
        scroll: Some(PwrightLocatorScrollMode::None),
        steps: Some(3),
        trial: Some(true),
    };
    let encoded = encode(&options);
    let mut with_future_field = encoded.clone();
    with_future_field.extend_from_slice(&[0, 11, 0, 0, 0, 1, 99]);
    let body_len = u32::from_be_bytes(
        with_future_field[..4]
            .try_into()
            .expect("record length prefix"),
    );
    with_future_field[..4].copy_from_slice(&(body_len + 7).to_be_bytes());

    // Act
    let decoded = decode(&with_future_field);

    // Assert
    assert_eq!(field_tags(&encoded), (1..=10).collect::<Vec<_>>());
    assert_eq!(decoded, options);
    assert!(encoded.len() - 4 <= 128);
}

#[test]
fn set_checked_options_preserve_all_fields_and_skip_future_tags() {
    // Arrange
    let options = PwrightLocatorSetCheckedOptions {
        force: Some(false),
        position: Some(PwrightLocatorPoint { x: 4.5, y: -2.0 }),
        scroll: Some(PwrightLocatorScrollMode::None),
        trial: Some(true),
    };
    let encoded = encode_set_checked_options(&options);
    let mut with_future_field = encoded.clone();
    with_future_field.extend_from_slice(&[0, 5, 0, 0, 0, 1, 99]);
    let body_len = u32::from_be_bytes(
        with_future_field[..4]
            .try_into()
            .expect("record length prefix"),
    );
    with_future_field[..4].copy_from_slice(&(body_len + 7).to_be_bytes());

    // Act
    let decoded = decode_set_checked_options(&with_future_field);

    // Assert
    assert_eq!(field_tags(&encoded), (1..=4).collect::<Vec<_>>());
    assert_eq!(decoded, options);
    assert!(encoded.len() - 4 <= 64);
}

#[test]
fn locator_v2_click_steps_are_bounded_before_payload_creation() {
    // Arrange
    let mut options = empty_options();
    options.steps = Some(10_001);
    let mut writer = BinaryWriter::new();
    let mut context = BinaryValueContext::new(limits());

    // Act
    let error = options
        .encode_binary(&mut writer, &mut context)
        .expect_err("oversized mouse work must not encode");

    // Assert
    assert!(error.to_string().contains("click_options.steps"));
    assert!(writer.as_slice().len() < 16);
}

#[test]
fn locator_v1_and_v2_keep_separate_versions_and_click_record_kinds() {
    // Arrange
    let click_options = "pwright.locator.click_options";

    // Act
    let v1_kind = LOCATOR_SCHEMA_CATALOG
        .types()
        .iter()
        .find(|schema| schema.id() == click_options)
        .expect("locator v1 Click options should exist")
        .kind();
    let v2_kind = LOCATOR_V2_SCHEMA_CATALOG
        .types()
        .iter()
        .find(|schema| schema.id() == click_options)
        .expect("locator v2 Click options should exist")
        .kind();

    // Assert
    assert_eq!(LOCATOR_SCHEMA.version(), ProtocolVersion::new(1, 23));
    assert_eq!(LOCATOR_V2_SCHEMA.version(), ProtocolVersion::new(2, 6));
    assert_eq!(v1_kind, BinaryTypeKind::Record);
    assert_eq!(v2_kind, BinaryTypeKind::ExtensibleRecord);
    assert_eq!(LOCATOR_SCHEMA_CATALOG.validate(), Ok(()));
    assert_eq!(LOCATOR_V2_SCHEMA_CATALOG.validate(), Ok(()));
}

#[test]
fn locator_v2_focus_management_matches_v1_bytes_and_requires_minor_two_one() {
    // Arrange
    let limits = LocatorCodecLimits::new(1_024);
    let v1_codec = LocatorCodec::new(limits);
    let v2_codec = LocatorV2Codec::new(limits);
    let old_v2_codec = v2_codec.with_capability_version(ProtocolVersion::new(2, 0));
    let focus_params = FocusParams::new(Selector::css("input.name").expect("valid CSS"));
    let blur_params = BlurParams::new(Selector::css("input.name").expect("valid CSS"));
    let v1_focus = Command::<Focus>::new(
        CallId::new(17),
        ObjectId::new(19),
        CallMetadata::default(),
        focus_params.clone(),
    );
    let v2_focus = Command::<locator_v2::Focus>::new(
        CallId::new(17),
        ObjectId::new(19),
        CallMetadata::default(),
        focus_params,
    );
    let v1_blur = Command::<Blur>::new(
        CallId::new(23),
        ObjectId::new(29),
        CallMetadata::default(),
        blur_params.clone(),
    );
    let v2_blur = Command::<locator_v2::Blur>::new(
        CallId::new(23),
        ObjectId::new(29),
        CallMetadata::default(),
        blur_params,
    );

    // Act
    let v1_focus_parts = v1_codec
        .encode_parts(&v1_focus)
        .expect("v1 Focus should encode");
    let v2_focus_parts = v2_codec
        .encode_parts(&v2_focus)
        .expect("v2.1 Focus should encode");
    let v1_blur_parts = v1_codec
        .encode_parts(&v1_blur)
        .expect("v1 Blur should encode");
    let v2_blur_parts = v2_codec
        .encode_parts(&v2_blur)
        .expect("v2.1 Blur should encode");
    let old_error = old_v2_codec
        .encode_parts(&v2_focus)
        .expect_err("v2.0 must reject Focus before writing a payload");
    let result = CommandResult::<locator_v2::Blur>::new(CallId::new(31), Ok(()));
    let result_parts = v2_codec
        .encode_parts(&result)
        .expect("v2.1 Blur result should encode");

    // Assert
    assert_eq!(v2_focus_parts.route(), v1_focus_parts.route());
    assert_eq!(v2_focus_parts.payload(), v1_focus_parts.payload());
    assert_eq!(v2_blur_parts.route(), v1_blur_parts.route());
    assert_eq!(v2_blur_parts.payload(), v1_blur_parts.payload());
    assert_eq!(
        old_error,
        LocatorCodecError::FocusManagementUnsupported {
            version: ProtocolVersion::new(2, 0),
        }
    );
    assert_eq!(
        result_parts.route(),
        pwright_transport::FrameRoute::Result {
            call_id: CallId::new(31),
        }
    );
}

#[test]
fn locator_v2_press_matches_v1_bytes_and_requires_minor_two_two() {
    // Arrange
    let limits = LocatorCodecLimits::new(1_024);
    let v1_codec = LocatorCodec::new(limits);
    let v2_codec = LocatorV2Codec::new(limits);
    let old_v2_codec = v2_codec.with_capability_version(ProtocolVersion::new(2, 1));
    let params = PressParams::new(
        Selector::test_id("editor"),
        PressKey::new("ControlOrMeta+Enter").expect("key should build"),
        PressOptions::new()
            .with_delay(Duration::from_millis(5))
            .with_no_wait_after(true),
    );
    let v1 = Command::<Press>::new(
        CallId::new(41),
        ObjectId::new(43),
        CallMetadata::default(),
        params.clone(),
    );
    let v2 = Command::<locator_v2::Press>::new(
        CallId::new(41),
        ObjectId::new(43),
        CallMetadata::default(),
        params,
    );

    // Act
    let v1_parts = v1_codec
        .encode_parts(&v1)
        .expect("v1.19 Press should encode");
    let v2_parts = v2_codec
        .encode_parts(&v2)
        .expect("v2.2 Press should encode");
    let old_error = old_v2_codec
        .encode_parts(&v2)
        .expect_err("v2.1 must reject Press before writing a payload");

    // Assert
    assert_eq!(v2_parts.route(), v1_parts.route());
    assert_eq!(v2_parts.payload(), v1_parts.payload());
    assert_eq!(
        old_error,
        LocatorCodecError::PressUnsupported {
            version: ProtocolVersion::new(2, 1),
        }
    );
}

#[test]
fn locator_set_checked_matches_across_majors_and_requires_the_new_minors() {
    // Arrange
    let limits = LocatorCodecLimits::new(1_024);
    let v1_codec = LocatorCodec::new(limits);
    let v2_codec = LocatorV2Codec::new(limits);
    let old_v1_codec = v1_codec.with_capability_version(ProtocolVersion::new(1, 19));
    let old_v2_codec = v2_codec.with_capability_version(ProtocolVersion::new(2, 2));
    let params = SetCheckedParams::new(
        Selector::label("Remember me", true),
        true,
        SetCheckedOptions::new()
            .with_force(false)
            .with_position(Position::new(3.25, 7.5).expect("finite point should build"))
            .with_scroll(ScrollMode::None)
            .with_trial(false),
    );
    let v1 = Command::<SetChecked>::new(
        CallId::new(47),
        ObjectId::new(53),
        CallMetadata::default(),
        params.clone(),
    );
    let v2 = Command::<locator_v2::SetChecked>::new(
        CallId::new(47),
        ObjectId::new(53),
        CallMetadata::default(),
        params,
    );

    // Act
    let v1_parts = v1_codec
        .encode_parts(&v1)
        .expect("v1.20 SetChecked should encode");
    let v2_parts = v2_codec
        .encode_parts(&v2)
        .expect("v2.3 SetChecked should encode");
    let old_v1_error = old_v1_codec
        .encode_parts(&v1)
        .expect_err("v1.19 must reject SetChecked before writing a payload");
    let old_v2_error = old_v2_codec
        .encode_parts(&v2)
        .expect_err("v2.2 must reject SetChecked before writing a payload");
    let frame = WireFrame::new(
        ProtocolVersion::new(2, 3),
        v2_parts.route(),
        v2_parts.payload().to_vec(),
        v2_parts.attachments().to_vec(),
        FrameLimits::new(4_096, 0, 0, 4_096),
    )
    .expect("SetChecked command should fit frame limits");
    let decoded: Command<locator_v2::SetChecked> = v2_codec
        .decode_frame(frame)
        .expect("v2.3 SetChecked should decode");

    // Assert
    assert_eq!(v2_parts.route(), v1_parts.route());
    assert_eq!(v2_parts.payload(), v1_parts.payload());
    assert_eq!(v2_parts.attachments(), v1_parts.attachments());
    assert_eq!(decoded.params(), v2.params());
    assert_eq!(
        old_v1_error,
        LocatorCodecError::SetCheckedUnsupported {
            version: ProtocolVersion::new(1, 19),
        }
    );
    assert_eq!(
        old_v2_error,
        LocatorCodecError::SetCheckedUnsupported {
            version: ProtocolVersion::new(2, 2),
        }
    );
}

#[test]
fn locator_select_option_matches_across_majors_and_requires_the_new_minors() {
    // Arrange
    let limits = LocatorCodecLimits::new(1_024);
    let v1_codec = LocatorCodec::new(limits);
    let v2_codec = LocatorV2Codec::new(limits);
    let old_v1_codec = v1_codec.with_capability_version(ProtocolVersion::new(1, 20));
    let old_v2_codec = v2_codec.with_capability_version(ProtocolVersion::new(2, 3));
    let params = SelectOptionParams::new(
        Selector::label("Color", true),
        vec![
            SelectOptionValue::value_or_label("Blue"),
            SelectOptionValue::new()
                .with_value("green")
                .with_label("Green"),
        ],
        SelectOptionOptions::new().with_force(false),
    );
    let v1 = Command::<SelectOption>::new(
        CallId::new(59),
        ObjectId::new(61),
        CallMetadata::default(),
        params.clone(),
    );
    let v2 = Command::<locator_v2::SelectOption>::new(
        CallId::new(59),
        ObjectId::new(61),
        CallMetadata::default(),
        params,
    );

    // Act
    let v1_parts = v1_codec
        .encode_parts(&v1)
        .expect("v1.21 SelectOption should encode");
    let v2_parts = v2_codec
        .encode_parts(&v2)
        .expect("v2.4 SelectOption should encode");
    let old_v1_error = old_v1_codec
        .encode_parts(&v1)
        .expect_err("v1.20 must reject SelectOption before writing a payload");
    let old_v2_error = old_v2_codec
        .encode_parts(&v2)
        .expect_err("v2.3 must reject SelectOption before writing a payload");

    // Assert
    assert_eq!(v2_parts.route(), v1_parts.route());
    assert_eq!(v2_parts.payload(), v1_parts.payload());
    assert_eq!(v2_parts.attachments(), v1_parts.attachments());
    assert_eq!(
        old_v1_error,
        LocatorCodecError::SelectOptionUnsupported {
            version: ProtocolVersion::new(1, 20),
        }
    );
    assert_eq!(
        old_v2_error,
        LocatorCodecError::SelectOptionUnsupported {
            version: ProtocolVersion::new(2, 3),
        }
    );
}

#[test]
fn locator_scroll_into_view_matches_across_majors_and_requires_the_new_minors() {
    // Arrange
    let limits = LocatorCodecLimits::new(1_024);
    let v1_codec = LocatorCodec::new(limits);
    let v2_codec = LocatorV2Codec::new(limits);
    let old_v1_codec = v1_codec.with_capability_version(ProtocolVersion::new(1, 21));
    let old_v2_codec = v2_codec.with_capability_version(ProtocolVersion::new(2, 4));
    let params = ScrollIntoViewIfNeededParams::new(
        Selector::css("#below-fold").expect("valid CSS selector"),
    );
    let v1 = Command::<ScrollIntoViewIfNeeded>::new(
        CallId::new(63),
        ObjectId::new(65),
        CallMetadata::default(),
        params.clone(),
    );
    let v2 = Command::<locator_v2::ScrollIntoViewIfNeeded>::new(
        CallId::new(63),
        ObjectId::new(65),
        CallMetadata::default(),
        params,
    );

    // Act
    let v1_parts = v1_codec
        .encode_parts(&v1)
        .expect("v1.22 ScrollIntoViewIfNeeded should encode");
    let v2_parts = v2_codec
        .encode_parts(&v2)
        .expect("v2.5 ScrollIntoViewIfNeeded should encode");
    let old_v1_error = old_v1_codec
        .encode_parts(&v1)
        .expect_err("v1.21 must reject ScrollIntoViewIfNeeded before writing a payload");
    let old_v2_error = old_v2_codec
        .encode_parts(&v2)
        .expect_err("v2.4 must reject ScrollIntoViewIfNeeded before writing a payload");

    // Assert
    assert_eq!(v2_parts.route(), v1_parts.route());
    assert_eq!(v2_parts.payload(), v1_parts.payload());
    assert_eq!(v2_parts.attachments(), v1_parts.attachments());
    assert_eq!(
        old_v1_error,
        LocatorCodecError::ScrollIntoViewIfNeededUnsupported {
            version: ProtocolVersion::new(1, 21),
        }
    );
    assert_eq!(
        old_v2_error,
        LocatorCodecError::ScrollIntoViewIfNeededUnsupported {
            version: ProtocolVersion::new(2, 4),
        }
    );
}

#[test]
fn public_locator_v2_click_command_round_trips_every_option() {
    // Arrange
    let codec = LocatorV2Codec::new(LocatorCodecLimits::new(1_024));
    let position = Position::new(10.25, -4.5).expect("finite point should build");
    let options = ClickOptionsV2::new()
        .with_button(MouseButton::Right)
        .with_click_count(std::num::NonZeroU8::new(2).expect("two is nonzero"))
        .with_delay(std::time::Duration::from_millis(15))
        .with_force(false)
        .with_modifiers(
            KeyboardModifiers::new()
                .with(KeyboardModifier::ControlOrMeta)
                .with(KeyboardModifier::Shift),
        )
        .with_no_wait_after(true)
        .with_position(position)
        .with_scroll(ScrollMode::None)
        .with_steps(std::num::NonZeroU32::new(3).expect("three is nonzero"))
        .with_trial(false);
    let command = Command::<locator_v2::Click>::new(
        CallId::new(7),
        ObjectId::new(9),
        CallMetadata::default(),
        ClickParamsV2::new(Selector::css("button.save").expect("valid CSS"), options),
    );

    // Act
    let parts = codec
        .encode_parts(&command)
        .expect("public locator v2 command should encode");
    let frame = WireFrame::new(
        ProtocolVersion::new(2, 3),
        parts.route(),
        parts.payload().to_vec(),
        parts.attachments().to_vec(),
        FrameLimits::new(4_096, 0, 0, 4_096),
    )
    .expect("encoded command should fit frame limits");
    let decoded: Command<locator_v2::Click> = codec
        .decode_frame(frame)
        .expect("public locator v2 command should decode");

    // Assert
    assert_eq!(decoded.call_id(), command.call_id());
    assert_eq!(decoded.object_id(), command.object_id());
    assert_eq!(decoded.params(), command.params());
}

#[test]
fn locator_v2_unchanged_commands_delegate_to_the_stable_v1_bytes() {
    // Arrange
    let limits = LocatorCodecLimits::new(1_024);
    let v1_codec = LocatorCodec::new(limits);
    let v2_codec = LocatorV2Codec::new(limits);
    let params = CountParams::new(Selector::css("li").expect("valid CSS"));
    let v1 = Command::<Count>::new(
        CallId::new(11),
        ObjectId::new(13),
        CallMetadata::default(),
        params.clone(),
    );
    let v2 = Command::<locator_v2::Count>::new(
        CallId::new(11),
        ObjectId::new(13),
        CallMetadata::default(),
        params,
    );

    // Act
    let v1_parts = v1_codec.encode_parts(&v1).expect("v1 Count should encode");
    let v2_parts = v2_codec.encode_parts(&v2).expect("v2 Count should encode");

    // Assert
    assert_eq!(v2_parts.route(), v1_parts.route());
    assert_eq!(v2_parts.payload(), v1_parts.payload());
    assert_eq!(v2_parts.attachments(), v1_parts.attachments());
}
