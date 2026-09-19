//! Bounded offline bridge for the pinned gotoImpl oracle; not browser evidence.
use pwright_backend_cdp_navigation::frame_lifecycle::{
    DocumentGeneration, LifecycleSnapshot,
    native::NativeLifecycleNotice,
    navigation::{
        FrameNavigationWait, FrameNavigationWaitProfile, NavigationPoll, NavigationWaitError,
    },
};
use pwright_cap_navigation::WaitUntil;
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceScope,
};
use serde_json::{Value, json};
use std::{
    error::Error,
    io::{Read, Write},
    time::Duration,
};

fn string<'a>(value: &'a Value, name: &str) -> Result<&'a str, Box<dyn Error>> {
    value[name]
        .as_str()
        .ok_or_else(|| format!("missing string {name}").into())
}
fn id(value: &Value, name: &str) -> Result<ObjectId, Box<dyn Error>> {
    Ok(ObjectId::new(value[name].as_u64().ok_or("missing id")?))
}
fn main() -> Result<(), Box<dyn Error>> {
    let mut input = Vec::new();
    std::io::stdin()
        .take(256 * 1024 + 1)
        .read_to_end(&mut input)?;
    if input.len() > 256 * 1024 {
        return Err("input bound".into());
    }
    let cases: Value = serde_json::from_slice(&input)?;
    let cases = cases.as_array().ok_or("cases must be an array")?;
    if cases.len() > 64 {
        return Err("case bound".into());
    }
    let mut output = Vec::new();
    for case in cases {
        let profile =
            FrameNavigationWaitProfile::new(ResourceScope::Connection, 32, 16, 128, 256, 1024)?;
        let plan = profile.requirements();
        let ledger = ProductionResourceLedger::new(
            ResourceLimits::deny_all(plan.scopes, plan.leases).with_class(
                ResourceClass::BackendStateBytes,
                profile.retained_bytes(),
                profile.retained_bytes(),
            ),
            plan,
        )?;
        let wait = match string(case, "wait")? {
            "commit" => WaitUntil::Commit,
            "domcontentloaded" => WaitUntil::DomContentLoaded,
            "load" => WaitUntil::Load,
            "networkidle" => WaitUntil::NetworkIdle,
            _ => return Err("unknown wait".into()),
        };
        let mut operation =
            FrameNavigationWait::new(profile, &ledger, ObjectId::new(2), wait, None)?;
        let mut snapshot = LifecycleSnapshot {
            document: None,
            commit: true,
            dom_content_loaded: false,
            load: false,
            network_idle: false,
        };
        let mut snapshots = Vec::new();
        let steps = case["steps"].as_array().ok_or("steps required")?;
        if steps.len() > 128 {
            return Err("step bound".into());
        }
        for step in steps {
            let frame = ObjectId::new(step["frame"].as_u64().unwrap_or(2));
            let notice = match string(step, "type")? {
                "start" => Some(NativeLifecycleNotice::RequestStarted {
                    frame,
                    request: id(step, "request")?,
                    document_loader: Some(string(step, "loader")?),
                    redirected_from: step["from"].as_u64().map(ObjectId::new),
                }),
                "commit" => {
                    if frame == ObjectId::new(2) {
                        snapshot.dom_content_loaded = false;
                        snapshot.load = false;
                        snapshot.network_idle = false;
                    }
                    Some(NativeLifecycleNotice::Committed {
                        frame,
                        loader: string(step, "loader")?,
                        document: DocumentGeneration(1),
                    })
                }
                "hash" => Some(NativeLifecycleNotice::SameDocument { frame }),
                "response" => Some(NativeLifecycleNotice::Response {
                    frame,
                    request: id(step, "request")?,
                    response: &step["response"],
                }),
                "fail" => Some(NativeLifecycleNotice::RequestFailed {
                    frame,
                    request: id(step, "request")?,
                    error_text: "net::ERR_ABORTED",
                    canceled: false,
                }),
                "receipt" => {
                    let _ = operation.receipt(Duration::ZERO, step["loader"].as_str());
                    None
                }
                "lifecycle" => {
                    match string(step, "event")? {
                        "load" => snapshot.load = true,
                        "domcontentloaded" => snapshot.dom_content_loaded = true,
                        "networkidle" => snapshot.network_idle = true,
                        _ => return Err("unknown lifecycle".into()),
                    }
                    None
                }
                _ => return Err("unknown step".into()),
            };
            // The observed error is retained by the operation and sampled below.
            if let Some(notice) = notice {
                let _ = operation.observe(Duration::ZERO, &notice);
            }
            snapshots.push(match operation.poll(Duration::ZERO, Some(snapshot)) {
                Ok(NavigationPoll::Pending) => json!({"state":"pending"}),
                Ok(NavigationPoll::Complete { response_json, .. }) => json!({"state":"complete","response":response_json.map(serde_json::from_slice::<Value>).transpose()?}),
                Err(NavigationWaitError::Interrupted) => json!({"state":"interrupted"}),
                Err(NavigationWaitError::Network { .. }) => json!({"state":"network"}),
                Err(error) => return Err(error.into()),
            });
        }
        drop(operation);
        if ledger.metrics()?.active_leases != 0 {
            return Err("operation lease leak".into());
        }
        output.push(snapshots);
    }
    let bytes = serde_json::to_vec(&output)?;
    if bytes.len() > 1024 * 1024 {
        return Err("output bound".into());
    }
    std::io::stdout().write_all(&bytes)?;
    Ok(())
}
