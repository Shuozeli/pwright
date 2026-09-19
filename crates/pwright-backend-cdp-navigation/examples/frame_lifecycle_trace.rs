//! Bounded offline trace bridge, not a browser or Node runtime service.
use pwright_backend_cdp_navigation::frame_lifecycle::{
    DocumentGeneration, DocumentLifecycle, FrameLifecycleProfile, FrameLifecycleStore,
    LifecycleInput,
};
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceScope,
};
use serde_json::{Value, json};
use std::{
    error::Error,
    io::{Read, Write},
    time::Duration,
};

fn number(value: &Value, name: &str) -> Result<u64, Box<dyn Error>> {
    value[name]
        .as_u64()
        .ok_or_else(|| format!("missing integer {name}").into())
}
fn object(value: &Value, name: &str) -> Result<ObjectId, Box<dyn Error>> {
    Ok(ObjectId::new(number(value, name)?))
}
fn optional(value: &Value, name: &str) -> Option<ObjectId> {
    value[name].as_u64().map(ObjectId::new)
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut input = Vec::new();
    std::io::stdin()
        .take(512 * 1024 + 1)
        .read_to_end(&mut input)?;
    if input.len() > 512 * 1024 {
        return Err("trace input exceeds bound".into());
    }
    let value: Value = serde_json::from_slice(&input)?;
    let cases = value.as_array().ok_or("trace cases must be an array")?;
    if cases.len() > 64 {
        return Err("too many traces".into());
    }
    let mut traces = Vec::new();
    for case in cases {
        let profile = FrameLifecycleProfile::new(ResourceScope::Connection, 16, 64)?;
        let plan = profile.requirements();
        let resources = ProductionResourceLedger::new(
            ResourceLimits::deny_all(plan.scopes, plan.leases).with_class(
                ResourceClass::BackendStateBytes,
                profile.retained_bytes(),
                profile.retained_bytes(),
            ),
            plan,
        )?;
        let mut store = FrameLifecycleStore::new(profile, &resources)?;
        let mut now = Duration::ZERO;
        let mut snapshots = Vec::new();
        let steps = case["steps"].as_array().ok_or("missing steps")?;
        if steps.len() > 128 {
            return Err("too many steps".into());
        }
        for step in steps {
            let frame = || object(step, "frame");
            let document = || number(step, "document").map(DocumentGeneration);
            let input = match step["type"].as_str().ok_or("missing step type")? {
                "attach" => Some(LifecycleInput::Attach {
                    frame: frame()?,
                    parent: optional(step, "parent"),
                }),
                "commit" => Some(LifecycleInput::Commit {
                    frame: frame()?,
                    document: document()?,
                    request: optional(step, "request"),
                }),
                "lifecycle" => Some(LifecycleInput::DocumentEvent {
                    frame: frame()?,
                    document: document()?,
                    event: match step["event"].as_str() {
                        Some("load") => DocumentLifecycle::Load,
                        Some("domcontentloaded") => DocumentLifecycle::DomContentLoaded,
                        _ => return Err("unexpected lifecycle".into()),
                    },
                }),
                "start" => Some(LifecycleInput::RequestStarted {
                    frame: frame()?,
                    request: object(step, "request")?,
                    favicon: step["favicon"].as_bool().ok_or("missing favicon flag")?,
                }),
                "finish" => Some(LifecycleInput::RequestFinished {
                    request: object(step, "request")?,
                }),
                "detach" => Some(LifecycleInput::Detach { frame: frame()? }),
                "hash" => None,
                "tick" => {
                    now = Duration::from_millis(number(step, "ms")?);
                    store.advance(now)?;
                    None
                }
                _ => return Err("unknown step".into()),
            };
            if let Some(input) = input {
                store.apply(now, input)?;
            }
            let mut snapshot = Vec::new();
            for id in 1..=8 {
                if let Some(state) = store.snapshot(ObjectId::new(id))? {
                    snapshot.push(json!([
                        id,
                        state.document.map(|d| d.0),
                        state.commit,
                        state.dom_content_loaded,
                        state.load,
                        state.network_idle
                    ]));
                }
            }
            snapshots.push(snapshot);
        }
        drop(store);
        if resources.metrics()?.active_leases != 0 {
            return Err("lifecycle admission leaked".into());
        }
        traces.push(snapshots);
    }
    let output = serde_json::to_vec(&traces)?;
    if output.len() > 1024 * 1024 {
        return Err("trace output exceeds bound".into());
    }
    std::io::stdout().write_all(&output)?;
    Ok(())
}
