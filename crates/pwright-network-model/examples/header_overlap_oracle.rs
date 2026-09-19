//! Development-only overlapping metadata comparison; native outcomes are inputs.
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceScope,
};
use pwright_network_model::extra_headers::{
    HeaderPool, HeaderProfile, HeaderRef,
    settings::{HeaderSettings, HeaderSettingsProfile, HeaderUpdate},
};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    context: bool,
    initial_empty: bool,
    order: [usize; 2],
    fail: [bool; 2],
    compensation_fail: [bool; 2],
}
#[derive(Serialize)]
struct Outcome {
    during: Vec<Option<String>>,
    phases: Vec<Option<String>>,
    final_value: Option<String>,
}
fn current(book: &HeaderSettings) -> Result<Option<String>, Box<dyn std::error::Error>> {
    Ok(book
        .effective(ObjectId::new(1), ObjectId::new(2))?
        .iter()
        .next()
        .map(|header| header.value.to_owned()))
}
fn run(case: Case) -> Result<Outcome, Box<dyn std::error::Error>> {
    if case.order != [0, 1] && case.order != [1, 0] {
        return Err("invalid completion order".into());
    }
    let profile = HeaderProfile::new(ResourceScope::extension(131, 0), 8, 4, 128)?;
    let settings = HeaderSettingsProfile::new(ResourceScope::extension(131, 1), 1, 1)?
        .with_concurrent_writers(2)?;
    let req = profile
        .requirements()?
        .combine_concurrent(settings.requirements());
    let mut limits = ResourceLimits::deny_all(req.scopes, req.leases);
    for class in ResourceClass::ALL {
        let bound = req.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    let ledger = ProductionResourceLedger::new(limits, req)?;
    let pool = HeaderPool::new(profile, &ledger)?;
    let book = HeaderSettings::new(settings, pool, &ledger)?;
    let begin = |value| {
        let input = [HeaderRef { name: "X-A", value }];
        if case.context {
            book.begin_context(ObjectId::new(1), &input)
        } else {
            book.begin_page(ObjectId::new(1), ObjectId::new(2), &input)
        }
    };
    if !case.initial_empty {
        begin("old")?.commit()?;
    }
    let mut during = Vec::new();
    let mut pending: [Option<HeaderUpdate>; 2] = [None, None];
    for (index, value) in ["first", "second"].into_iter().enumerate() {
        pending[index] = Some(begin(value)?);
        during.push(current(&book)?);
    }
    let mut phases = Vec::new();
    for index in case.order {
        if case.fail[index] {
            pending[index]
                .as_ref()
                .ok_or("missing pending writer")?
                .restore_prior()?;
            during.push(current(&book)?);
        } else {
            pending[index]
                .take()
                .ok_or("missing pending writer")?
                .commit()?;
        }
        phases.push(current(&book)?);
    }
    for index in case.order.into_iter().rev() {
        if let Some(update) = pending[index].take() {
            if case.compensation_fail[index] {
                drop(update);
            } else {
                update.rollback()?;
            }
            phases.push(current(&book)?);
        }
    }
    let result = Outcome {
        during,
        phases,
        final_value: current(&book)?,
    };
    drop(book);
    for class in ResourceClass::ALL {
        assert_eq!(ledger.metrics()?.class(class).current, 0);
    }
    Ok(result)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    const LIMIT: usize = 256 * 1024;
    let mut input = Vec::new();
    std::io::stdin()
        .take((LIMIT + 1) as u64)
        .read_to_end(&mut input)?;
    if input.len() > LIMIT {
        return Err("overlap oracle input limit".into());
    }
    let cases: Vec<Case> = serde_json::from_slice(&input)?;
    if cases.len() > 128 {
        return Err("overlap oracle case limit".into());
    }
    let outcomes: Result<Vec<_>, _> = cases.into_iter().map(run).collect();
    let output = serde_json::to_vec(&outcomes?)?;
    if output.len() > LIMIT {
        return Err("overlap oracle output limit".into());
    }
    std::io::stdout().write_all(&output)?;
    Ok(())
}
