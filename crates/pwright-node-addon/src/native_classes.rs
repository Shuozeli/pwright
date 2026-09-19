//! Read-only class-tag diagnostics. Never unwraps or casts a foreign native owner.

use crate::{
    NativeSession, NativeSessionSignal,
    context_settings::NativeContextSettings,
    environment::NativeEnvironment,
    frame_tree::{NativeFrameNode, NativeFrameTree},
    locator::NativeLocator,
};
use napi::{Env, JsString, JsValue, Unknown, ValueType, bindgen_prelude::TypeTag, sys};
use napi_derive::napi;

// One typed registry drives both diagnostic selection and the class identity
// proof. The source gate rejects newly exported classes missing from this list.
macro_rules! native_classes {
    ($($name:ident),+ $(,)?) => {
        const NAMES: &[&str] = &[$(stringify!($name)),+];
        fn class_tag(name: &str) -> Option<sys::napi_type_tag> {
            match name {
                $(stringify!($name) => Some(<$name as TypeTag>::type_tag()),)+
                _ => None,
            }
        }
    };
}

#[cfg(feature = "handles")]
use crate::handles::NativeHandle;

#[cfg(feature = "retained-file-input")]
use crate::handles::file_input::{NativeFileInputPreparation, NativeFileInputTimeoutSettings};

#[cfg(feature = "retained-file-input")]
native_classes!(
    NativeEnvironment,
    NativeSession,
    NativeSessionSignal,
    NativeFrameNode,
    NativeFrameTree,
    NativeContextSettings,
    NativeLocator,
    NativeHandle,
    NativeFileInputPreparation,
    NativeFileInputTimeoutSettings
);

#[cfg(all(feature = "handles", not(feature = "retained-file-input")))]
native_classes!(
    NativeEnvironment,
    NativeSession,
    NativeSessionSignal,
    NativeFrameTree,
    NativeFrameNode,
    NativeContextSettings,
    NativeLocator,
    NativeHandle
);

#[cfg(not(feature = "handles"))]
native_classes!(
    NativeEnvironment,
    NativeSession,
    NativeSessionSignal,
    NativeFrameTree,
    NativeFrameNode,
    NativeContextSettings,
    NativeLocator
);

#[napi]
pub fn native_class_names() -> Vec<&'static str> {
    NAMES.to_vec()
}

#[napi]
pub fn native_class_matches(
    env: Env,
    class_name: JsString<'_>,
    value: Unknown<'_>,
) -> napi::Result<bool> {
    if class_name.utf8_len()? > 64 {
        return Err(napi::Error::from_reason("Unknown native class"));
    }
    let name = class_name.into_utf8()?;
    let tag = class_tag(name.as_str()?)
        .ok_or_else(|| napi::Error::from_reason("Unknown native class"))?;
    if value.get_type()? != ValueType::Object {
        return Ok(false);
    }
    let mut matches = false;
    // SAFETY: both values are scoped to this callback. N-API only compares an
    // opaque tag; no native pointer or class-specific property is ever read.
    crate::evaluation_value::status(unsafe {
        sys::napi_check_object_type_tag(env.raw(), value.raw(), &tag, &mut matches)
    })?;
    Ok(matches)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_raw_class_is_module_local_and_pairwise_distinct() {
        // Arrange
        let identity = crate::native_napi::module_identity();

        // Act
        let tags: Vec<_> = NAMES.iter().map(|name| class_tag(name).unwrap()).collect();

        // Assert
        assert_ne!(identity, 0);
        for (index, tag) in tags.iter().enumerate() {
            assert_eq!(tag.lower, identity, "{} bypasses the facade", NAMES[index]);
            assert!(tags[..index].iter().all(|other| other.upper != tag.upper));
        }
    }
}
