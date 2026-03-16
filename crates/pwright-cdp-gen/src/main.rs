//! CDP protocol code generator.
//!
//! Reads Chrome DevTools Protocol JSON and generates typed Rust modules.
//! Supports a single combined protocol file (from `/json/protocol`) or
//! separate browser + JS protocol files.
//!
//! Usage:
//!   # From live Chrome (preferred — matches your actual Chrome version):
//!   curl -s http://HOST:9222/json/protocol > proto/cdp/chrome_protocol.json
//!   cargo run -p pwright-cdp-gen -- --out crates/pwright-cdp/src/generated/
//!
//!   # Or from vendored GitHub spec:
//!   cargo run -p pwright-cdp-gen -- --vendored --out crates/pwright-cdp/src/generated/

mod codegen;
mod protocol;

use protocol::ProtocolFile;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{env, fs};

fn main() {
    let args: Vec<String> = env::args().collect();
    let out_dir = args
        .iter()
        .position(|a| a == "--out")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("generated"));

    let use_vendored = args.iter().any(|a| a == "--vendored");

    let proto_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("proto/cdp");

    // Load protocol
    let protocol = if use_vendored {
        let browser_path = proto_dir.join("browser_protocol.json");
        let js_path = proto_dir.join("js_protocol.json");
        println!("Reading vendored specs:");
        println!("  {}", browser_path.display());
        println!("  {}", js_path.display());
        let mut browser: ProtocolFile = serde_json::from_str(
            &fs::read_to_string(&browser_path).expect("read browser_protocol"),
        )
        .expect("parse browser_protocol");
        let js: ProtocolFile =
            serde_json::from_str(&fs::read_to_string(&js_path).expect("read js_protocol"))
                .expect("parse js_protocol");
        browser.domains.extend(js.domains);
        browser
    } else {
        let live_path = proto_dir.join("chrome_protocol.json");
        println!("Reading live protocol: {}", live_path.display());
        serde_json::from_str(&fs::read_to_string(&live_path).expect("read live protocol"))
            .expect("parse live protocol")
    };

    println!(
        "Protocol v{}.{}: {} domains",
        protocol.version.major,
        protocol.version.minor,
        protocol.domains.len()
    );

    // Load HTTP endpoint aliases
    let http_path = proto_dir.join("http_endpoints.json");
    let aliases = if http_path.exists() {
        println!("Reading HTTP aliases: {}", http_path.display());
        let raw = fs::read_to_string(&http_path).expect("read http_endpoints");
        let parsed: serde_json::Value = serde_json::from_str(&raw).expect("parse http_endpoints");
        let mut map: HashMap<(String, String, String), String> = HashMap::new();
        if let Some(arr) = parsed["field_aliases"].as_array() {
            for entry in arr {
                let domain = entry["domain"].as_str().unwrap_or_default().to_string();
                let type_name = entry["type"].as_str().unwrap_or_default().to_string();
                let cdp_field = entry["cdp_field"].as_str().unwrap_or_default().to_string();
                let http_field = entry["http_field"].as_str().unwrap_or_default().to_string();
                println!("  Alias: {domain}.{type_name}.{cdp_field} -> {http_field}");
                map.insert((domain, type_name, cdp_field), http_field);
            }
        }
        map
    } else {
        HashMap::new()
    };

    fs::create_dir_all(&out_dir).expect("create output dir");

    let domains: Vec<&protocol::Domain> = protocol
        .domains
        .iter()
        .filter(|d| codegen::should_generate(&d.domain))
        .collect();

    println!(
        "Generating {} domains: {}",
        domains.len(),
        domains
            .iter()
            .map(|d| d.domain.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    let mut mod_lines = Vec::new();
    mod_lines.push("//! Generated CDP domain types.".to_string());
    mod_lines.push("//!".to_string());
    mod_lines.push(format!(
        "//! Protocol version {}.{}",
        protocol.version.major, protocol.version.minor
    ));
    if use_vendored {
        mod_lines.push("//! Source: ChromeDevTools/devtools-protocol (vendored)".to_string());
    } else {
        mod_lines.push("//! Source: Chrome live /json/protocol".to_string());
    }
    mod_lines.push(String::new());

    for domain in &domains {
        let snake = codegen::to_snake_case(&domain.domain);
        let code = codegen::generate_domain_module(domain, &aliases);
        let file_path = out_dir.join(format!("{snake}.rs"));
        fs::write(&file_path, &code).expect("write domain file");
        let cmds = domain.commands.len();
        let types = domain.types.len();
        let events = domain.events.len();
        println!(
            "  {} -> {} ({} types, {} commands, {} events)",
            domain.domain,
            file_path.display(),
            types,
            cmds,
            events,
        );
        mod_lines.push(format!("pub mod {snake};"));
    }
    mod_lines.push(String::new());

    let mod_path = out_dir.join("mod.rs");
    fs::write(&mod_path, mod_lines.join("\n")).expect("write mod.rs");
    println!("Wrote {}", mod_path.display());

    println!("Done.");
}
