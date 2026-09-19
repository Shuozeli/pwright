//! Bounded development-only bridge; not part of the Node/native dependency graph.
use pwright_core::{ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceScope};
use pwright_network_model::extra_headers::{HeaderError, HeaderPool, HeaderProfile, HeaderRef};
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
    context: Vec<Header>,
    page: Vec<Header>,
    option: Option<String>,
}

#[derive(Serialize)]
struct Outcome {
    headers: Vec<Header>,
    referer: Option<String>,
    conflict: bool,
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const LIMIT: usize = 1024 * 1024;
    let mut input = Vec::new();
    std::io::stdin()
        .take((LIMIT + 1) as u64)
        .read_to_end(&mut input)?;
    if input.len() > LIMIT {
        return Err("header oracle input limit".into());
    }
    let cases: Vec<Case> = serde_json::from_slice(&input)?;
    if cases.len() > 2048 {
        return Err("header oracle case limit".into());
    }
    let profile = HeaderProfile::new(ResourceScope::extension(128, 0), 3, 16, 4096)?;
    let req = profile.requirements()?;
    let mut limits = ResourceLimits::deny_all(req.scopes, req.leases);
    for class in ResourceClass::ALL {
        let bound = req.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    let ledger = ProductionResourceLedger::new(limits, req)?;
    let pool = HeaderPool::new(profile, &ledger)?;
    let mut outcomes = Vec::with_capacity(cases.len());
    for case in cases {
        let context = pool.context(&borrowed(&case.context))?;
        let page = pool.page(&borrowed(&case.page))?;
        let merged = pool.merge(Some(&context), Some(&page))?;
        let (referer, conflict) = match page.navigation_referrer(case.option.as_deref()) {
            Ok(value) => (value.map(str::to_owned), false),
            Err(HeaderError::ReferrerConflict) => (None, true),
            Err(error) => return Err(error.into()),
        };
        outcomes.push(Outcome {
            headers: merged
                .iter()
                .map(|header| Header {
                    name: header.name.into(),
                    value: header.value.into(),
                })
                .collect(),
            referer,
            conflict,
        });
    }
    drop(pool);
    for class in ResourceClass::ALL {
        assert_eq!(ledger.metrics()?.class(class).current, 0);
    }
    let output = serde_json::to_vec(&outcomes)?;
    if output.len() > LIMIT {
        return Err("header oracle output limit".into());
    }
    std::io::stdout().write_all(&output)?;
    Ok(())
}
