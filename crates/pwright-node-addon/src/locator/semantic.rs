//! String/option conversion only; Rust host constructs typed selector leaves.

use super::{NativeLocator, SELECTOR_BYTES};
use crate::{NativeSession, binding_error, boundary};
use napi::{
    JsString,
    bindgen_prelude::{JsObjectValue, Object},
};
use napi_derive::napi;
use pwright_node_host::{
    HostError,
    locator_query::semantic::{RoleQueryOptions, SemanticSelector},
};
use std::str::FromStr;

fn text(value: JsString<'_>, maximum: usize) -> napi::Result<String> {
    if value.utf8_len()? > maximum {
        return Err(binding_error(HostError::InvalidArgument(
            "semantic Locator string is too large".into(),
        )));
    }
    {
        let length = value.utf16_len()?;
        let units = value.into_utf16()?;
        if char::decode_utf16(units.as_slice()[..length].iter().copied()).any(|unit| unit.is_err())
        {
            return Err(binding_error(HostError::UnsupportedOperation(
                "unpaired UTF-16 in semantic Locator input".into(),
            )));
        }
    }
    // UTF-16 validation storage is gone before UTF-8 conversion. The query
    // draft's two-copy allowance covers each phase, plus fixed terminators.
    boundary::bounded_string(value, maximum, "semantic Locator input")
}

fn bytes(value: Option<JsString<'_>>) -> napi::Result<usize> {
    value
        .map(|value| value.utf8_len())
        .transpose()
        .map(|value| value.unwrap_or(0))
}

#[napi]
impl NativeSession {
    #[napi]
    pub fn semantic_locator(
        &self,
        frame: JsString<'_>,
        kind: JsString<'_>,
        source: JsString<'_>,
        exact: bool,
        flags: Option<JsString<'_>>,
    ) -> napi::Result<NativeLocator> {
        let kind = boundary::bounded_string(kind, 16, "semantic Locator kind")?;
        let kind = SemanticSelector::from_str(&kind).map_err(binding_error)?;
        let draft = self
            .queries
            .prepare_leaf(
                source
                    .utf8_len()?
                    .saturating_add(bytes(flags)?)
                    .saturating_add(kind.implicit_string_bytes()),
            )
            .map_err(binding_error)?;
        let frame = boundary::bounded_string(frame, 20, "Frame ID")?;
        let source = text(source, SELECTOR_BYTES)?;
        let flags = flags.map(|value| text(value, 8)).transpose()?;
        let query = draft
            .semantic(kind, source, flags, exact)
            .map_err(binding_error)?;
        Ok(NativeLocator {
            locator: self.host.locator(&frame, query).map_err(binding_error)?,
            publications: self.publications.clone(),
            ffi: self.ffi.clone(),
            queries: self.queries.clone(),
            conversions: self.locator_conversions.clone(),
        })
    }

    #[napi]
    pub fn role_locator(
        &self,
        frame: JsString<'_>,
        role: JsString<'_>,
        options: Object<'_>,
    ) -> napi::Result<NativeLocator> {
        let name = options.get_named_property::<Option<JsString<'_>>>("name")?;
        let flags = options.get_named_property::<Option<JsString<'_>>>("nameFlags")?;
        let draft = self
            .queries
            .prepare_leaf(bytes(name)?.saturating_add(bytes(flags)?))
            .map_err(binding_error)?;
        let frame = boundary::bounded_string(frame, 20, "Frame ID")?;
        let role = text(role, 32)?;
        let level = options
            .get_named_property::<Option<f64>>("level")?
            .map(|value| {
                if !value.is_finite()
                    || value.fract() != 0.0
                    || value < 0.0
                    || value > f64::from(u32::MAX)
                {
                    return Err(binding_error(HostError::InvalidArgument(
                        "role level must be a uint32".into(),
                    )));
                }
                Ok(value as u32)
            })
            .transpose()?;
        let query = draft
            .role(
                &role,
                RoleQueryOptions {
                    name: name.map(|value| text(value, SELECTOR_BYTES)).transpose()?,
                    name_flags: flags.map(|value| text(value, 8)).transpose()?,
                    exact: options.get_named_property("exact")?,
                    checked: options.get_named_property("checked")?,
                    disabled: options.get_named_property("disabled")?,
                    expanded: options.get_named_property("expanded")?,
                    include_hidden: options.get_named_property("includeHidden")?,
                    level,
                    pressed: options.get_named_property("pressed")?,
                    selected: options.get_named_property("selected")?,
                },
            )
            .map_err(binding_error)?;
        Ok(NativeLocator {
            locator: self.host.locator(&frame, query).map_err(binding_error)?,
            publications: self.publications.clone(),
            ffi: self.ffi.clone(),
            queries: self.queries.clone(),
            conversions: self.locator_conversions.clone(),
        })
    }
}
