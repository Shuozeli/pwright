//! Offline semantic bridge. URL parser allocation belongs to this diagnostic,
//! not the production PreparedGlob admission/retention claim.
use std::io::{Read, Write};

use pwright_node_host::frame_lookup::{GlobUrlResolution, PreparedGlob, RESOLVED_GLOB_BYTES};
use pwright_node_host::frame_tree::{FrameTreeFactory, FrameTreeLimits};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Case {
    #[serde(rename = "baseURL")]
    base_url: Option<String>,
    glob: String,
}

fn resolve(base: Option<&str>, relative: &str) -> Option<url::Url> {
    match base {
        Some(base) => url::Url::parse(base).ok()?.join(relative).ok(),
        None => url::Url::parse(relative).ok(),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = String::new();
    std::io::stdin()
        .take(512 * 1024 + 1)
        .read_to_string(&mut input)?;
    if input.len() > 512 * 1024 {
        return Err("URL glob probe input exceeds 512 KiB".into());
    }
    let cases: Vec<Case> = serde_json::from_str(&input)?;
    if cases.len() > 4096 {
        return Err("URL glob probe exceeds 4096 cases".into());
    }
    let resources = FrameTreeFactory::new(FrameTreeLimits {
        states: 1,
        frames_per_page: 1,
        revisions: 4,
        bytes: 2 * 1024 * 1024,
    })?;
    let mut output = std::io::BufWriter::new(std::io::stdout().lock());
    output.write_all(b"[")?;
    for (index, case) in cases.iter().enumerate() {
        if case
            .base_url
            .as_ref()
            .is_some_and(|base| base.len() > RESOLVED_GLOB_BYTES)
        {
            return Err("probe baseURL exceeds 16 KiB".into());
        }
        if index != 0 {
            output.write_all(b",")?;
        }
        let prepared = PreparedGlob::new(&resources, &case.glob)?;
        let parsed = prepared
            .relative_path()
            .and_then(|relative| resolve(case.base_url.as_deref(), relative));
        let origin = parsed
            .as_ref()
            .map(|url| url.origin().ascii_serialization());
        let resolution =
            parsed
                .as_ref()
                .zip(origin.as_ref())
                .map(|(url, origin)| GlobUrlResolution {
                    serialized: url.as_str(),
                    origin,
                });
        let plan = prepared.compile(&resources, resolution)?;
        serde_json::to_writer(&mut output, plan.pattern())?;
    }
    output.write_all(b"]\n")?;
    output.flush()?;
    assert_eq!(resources.metrics()?.bytes, 0);
    assert_eq!(resources.metrics()?.revisions, 0);
    Ok(())
}
