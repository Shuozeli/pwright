//! Bounded offline oracle bridge. No browser or Node runtime is loaded by Rust.
use std::io::{Read, Write};

use pwright_node_host::frame_lookup::GlobPlan;
use pwright_node_host::frame_tree::{FrameTreeFactory, FrameTreeLimits};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = String::new();
    std::io::stdin()
        .take(512 * 1024 + 1)
        .read_to_string(&mut input)?;
    if input.len() > 512 * 1024 {
        return Err("glob probe input exceeds 512 KiB".into());
    }
    let cases: Vec<String> = serde_json::from_str(&input)?;
    if cases.len() > 4096 {
        return Err("glob probe exceeds 4096 cases".into());
    }
    let resources = FrameTreeFactory::new(FrameTreeLimits {
        states: 1,
        frames_per_page: 1,
        revisions: 1,
        bytes: 128 * 1024,
    })?;
    let mut output = std::io::BufWriter::new(std::io::stdout().lock());
    output.write_all(b"[")?;
    for (index, case) in cases.iter().enumerate() {
        if index != 0 {
            output.write_all(b",")?;
        }
        let plan = GlobPlan::compile_resolved(&resources, case)?;
        serde_json::to_writer(&mut output, plan.pattern())?;
    }
    output.write_all(b"]\n")?;
    output.flush()?;
    assert_eq!(resources.metrics()?.bytes, 0);
    assert_eq!(resources.metrics()?.revisions, 0);
    Ok(())
}
