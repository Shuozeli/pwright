//! Synchronous identity admission on the environment thread, before commit.
//! Only missing wrappers are bound; descriptor storage belongs to the port's
//! admitted conversion allowance and lasting bindings own metadata-only leases.
use crate::{ContextDescriptor, PageDescriptor, binding_error, context_settings, frame_tree};
use napi::{
    Unknown, ValueType,
    bindgen_prelude::{FnArgs, Function, JsObjectValue, Object},
};
use pwright_node_host::{HostError, LifecycleNotice, LifecycleNoticeView};

pub(super) type Callback<'env> =
    Function<'env, FnArgs<(u32, String, String, Option<String>)>, Unknown<'env>>;

type CreationConverter<'env> =
    Function<'env, FnArgs<(Option<ContextDescriptor>, Option<PageDescriptor>)>, Unknown<'env>>;

pub(super) fn arguments(
    view: &LifecycleNoticeView<'_>,
) -> napi::Result<(u32, String, String, Option<String>)> {
    let (kind, context, page, opener) = match view.notice {
        LifecycleNotice::PageClose { context, page } => (0, context, Some(page), None),
        LifecycleNotice::ContextClose { context } => (1, context, None, None),
        LifecycleNotice::ContextPage { context, page } => (2, context, Some(page), None),
        LifecycleNotice::Popup { opener, popup } => {
            let page = view
                .revision
                .pages()
                .iter()
                .find(|page| page.id == popup)
                .ok_or_else(|| invalid("popup descriptor is missing"))?;
            (3, page.context, Some(popup), Some(opener))
        }
    };
    Ok((
        kind,
        context.get().to_string(),
        page.map(|id| id.get().to_string()).unwrap_or_default(),
        opener.map(|id| id.get().to_string()),
    ))
}

fn invalid(message: &str) -> napi::Error {
    binding_error(HostError::Backend(message.into()))
}

pub(super) fn finish<'env>(
    view: LifecycleNoticeView<'_>,
    prepared: Unknown<'env>,
) -> napi::Result<Option<Unknown<'env>>> {
    if prepared.get_type()? == ValueType::Undefined {
        return Ok(None);
    }
    let LifecycleNotice::ContextPage { context, page } = view.notice else {
        return emitter(prepared);
    };
    if prepared.get_type()? != ValueType::Object {
        return Err(invalid(
            "Page-ready preparation must return a descriptor request",
        ));
    }
    let request = unsafe { prepared.cast::<Object<'_>>()? };
    let context_needed: bool = request.get_named_property("contextNeeded")?;
    let page_needed: bool = request.get_named_property("pageNeeded")?;
    if context_needed && !page_needed {
        return Err(invalid("a new Context cannot contain a cached Page"));
    }
    let context_descriptor = context_needed
        .then(|| {
            let value = view
                .revision
                .contexts()
                .iter()
                .find(|value| value.id == context)
                .ok_or_else(|| invalid("ready Context descriptor is missing"))?;
            let binding = value.settings.bind().map_err(binding_error)?;
            Ok::<_, napi::Error>(ContextDescriptor {
                id: context.get().to_string(),
                settings: context_settings::NativeContextSettings { binding },
            })
        })
        .transpose()?;
    let page_descriptor = page_needed
        .then(|| {
            let value = view
                .revision
                .pages()
                .iter()
                .find(|value| value.id == page && value.context == context)
                .ok_or_else(|| invalid("ready Page descriptor is missing"))?;
            // Both admissions precede native wrapper/ID conversion. Partial
            // failure releases the first binding without advancing the notice.
            let root = value.bind_main_frame().map_err(binding_error)?;
            let tree = value.tree.bind_tree().map_err(binding_error)?;
            Ok::<_, napi::Error>(PageDescriptor {
                id: page.get().to_string(),
                context_id: context.get().to_string(),
                main_frame_id: value.main_frame.id().get().to_string(),
                frame_tree: frame_tree::NativeFrameTree { binding: tree },
                main_frame_binding: Some(frame_tree::NativeFrameNode { binding: root }),
            })
        })
        .transpose()?;
    let convert: CreationConverter<'_> = request.get_named_property("prepare")?;
    let emit = convert.call((context_descriptor, page_descriptor).into())?;
    if emit.get_type()? != ValueType::Function {
        return Err(invalid(
            "ready descriptor conversion must return an emitter",
        ));
    }
    Ok(Some(emit))
}

fn emitter(value: Unknown<'_>) -> napi::Result<Option<Unknown<'_>>> {
    match value.get_type()? {
        ValueType::Undefined => Ok(None),
        ValueType::Function => Ok(Some(value)),
        _ => Err(invalid(
            "lifecycle preparation must return a function or undefined",
        )),
    }
}
