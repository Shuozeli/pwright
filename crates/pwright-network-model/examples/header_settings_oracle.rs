//! Development-only metadata transaction oracle with injected native outcomes.
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceScope,
};
use pwright_network_model::extra_headers::{
    HeaderPool, HeaderProfile, HeaderRef,
    settings::{HeaderSettings, HeaderSettingsProfile, HeaderUpdateStatus},
};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Header {
    name: String,
    value: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    context: bool,
    prior: Vec<Header>,
    next: Vec<Header>,
    fail: bool,
    compensation_fails: bool,
}
#[derive(Serialize)]
struct Outcome {
    during: Vec<Vec<Header>>,
    final_headers: Vec<Header>,
    failed: bool,
    uncertain: bool,
}
fn borrowed(headers: &[Header]) -> Vec<HeaderRef<'_>> {
    headers
        .iter()
        .map(|header| HeaderRef {
            name: &header.name,
            value: &header.value,
        })
        .collect()
}
fn current(book: &HeaderSettings) -> Result<Vec<Header>, Box<dyn std::error::Error>> {
    Ok(book
        .effective(ObjectId::new(1), ObjectId::new(2))?
        .iter()
        .map(|header| Header {
            name: header.name.into(),
            value: header.value.into(),
        })
        .collect())
}
fn run(case: Case) -> Result<Outcome, Box<dyn std::error::Error>> {
    let profile = HeaderProfile::new(ResourceScope::extension(130, 0), 8, 4, 128)?;
    let settings = HeaderSettingsProfile::new(ResourceScope::extension(130, 1), 1, 1)?;
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
    let begin = |headers: &[Header]| {
        let input = borrowed(headers);
        if case.context {
            book.begin_context(ObjectId::new(1), &input)
        } else {
            book.begin_page(ObjectId::new(1), ObjectId::new(2), &input)
        }
    };
    begin(&case.prior)?.commit()?;
    let update = begin(&case.next)?;
    let mut during = vec![current(&book)?];
    if case.fail {
        update.restore_prior()?;
        during.push(current(&book)?);
        if case.compensation_fails {
            drop(update);
        } else {
            update.rollback()?;
        }
    } else {
        update.commit()?;
    }
    let outcome = Outcome {
        during,
        final_headers: current(&book)?,
        failed: case.fail,
        uncertain: book.update_status(ObjectId::new(1), ObjectId::new(2))?
            == HeaderUpdateStatus::Indeterminate,
    };
    drop(book);
    for class in ResourceClass::ALL {
        assert_eq!(ledger.metrics()?.class(class).current, 0);
    }
    Ok(outcome)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    const LIMIT: usize = 256 * 1024;
    let mut input = Vec::new();
    std::io::stdin()
        .take((LIMIT + 1) as u64)
        .read_to_end(&mut input)?;
    if input.len() > LIMIT {
        return Err("header transaction input limit".into());
    }
    let cases: Vec<Case> = serde_json::from_slice(&input)?;
    if cases.len() > 128 {
        return Err("header transaction case limit".into());
    }
    let results: Result<Vec<_>, _> = cases.into_iter().map(run).collect();
    let output = serde_json::to_vec(&results?)?;
    if output.len() > LIMIT {
        return Err("header transaction output limit".into());
    }
    std::io::stdout().write_all(&output)?;
    Ok(())
}
