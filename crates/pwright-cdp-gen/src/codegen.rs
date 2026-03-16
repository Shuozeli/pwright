//! Generates typed Rust code from parsed CDP protocol domains.

use crate::protocol::{Command, Domain, Event, ItemRef, Property, TypeDef};
use std::collections::HashMap;
use std::fmt::Write;

/// Aliases from `http_endpoints.json`: (domain, type, cdp_field) -> http_field
pub type FieldAliases = HashMap<(String, String, String), String>;

/// Domains to generate code for.
const DOMAINS: &[&str] = &[
    "Accessibility",
    "Browser",
    "DOM",
    "Fetch",
    "Input",
    "Network",
    "Page",
    "Runtime",
    "Target",
];

/// Generate a complete Rust module file for a single CDP domain.
pub fn generate_domain_module(domain: &Domain, aliases: &FieldAliases) -> String {
    let mut out = String::with_capacity(8192);

    let snake = to_snake_case(&domain.domain);
    writeln!(
        out,
        "//! CDP `{}` domain — generated from protocol JSON.",
        domain.domain
    )
    .unwrap();
    if !domain.description.is_empty() {
        writeln!(out, "//!").unwrap();
        for line in domain.description.lines() {
            writeln!(out, "//! {line}").unwrap();
        }
    }
    writeln!(out).unwrap();
    writeln!(out, "#![allow(clippy::doc_markdown)]").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "use serde::{{Deserialize, Serialize}};").unwrap();
    // Only import Value if it's actually used in the domain
    if domain_uses_value(domain) {
        writeln!(out, "use serde_json::Value;").unwrap();
    }
    writeln!(out).unwrap();

    // Types
    for ty in &domain.types {
        generate_type(&mut out, ty, &domain.domain, aliases);
    }

    // Command params + returns
    for cmd in &domain.commands {
        generate_command_types(&mut out, cmd, &domain.domain);
    }

    // Event types
    for evt in &domain.events {
        generate_event_type(&mut out, evt, &domain.domain);
    }

    // Method signatures (as a trait-like impl block comment showing the API surface)
    writeln!(out, "// ── Methods ──").unwrap();
    writeln!(out, "//").unwrap();
    writeln!(
        out,
        "// These are the typed method signatures for {}.* commands.",
        domain.domain
    )
    .unwrap();
    writeln!(
        out,
        "// Integration into CdpSession is done in pwright-cdp."
    )
    .unwrap();
    writeln!(out).unwrap();

    for cmd in &domain.commands {
        generate_method_signature(&mut out, cmd, &domain.domain, &snake);
    }

    // Trim trailing whitespace/newlines for rustfmt
    while out.ends_with('\n') {
        out.pop();
    }
    out.push('\n');

    out
}

/// Check if a domain should be generated.
pub fn should_generate(domain_name: &str) -> bool {
    DOMAINS.contains(&domain_name)
}

/// Check if any type/command/event in the domain resolves to `Value`.
fn domain_uses_value(domain: &Domain) -> bool {
    let check_props = |props: &[Property]| -> bool {
        props.iter().any(|p| {
            // Cross-domain ref to non-generated domain => Value
            if let Some(ref r) = p.ref_type
                && r.contains('.')
            {
                let domain_name = r.split('.').next().unwrap_or("");
                return !should_generate(domain_name);
            }
            if matches!(p.type_kind.as_deref(), Some("object" | "any")) {
                return true;
            }
            // Array items that resolve to Value
            if p.type_kind.as_deref() == Some("array") {
                if let Some(ref items) = p.items {
                    if matches!(items.type_kind.as_deref(), Some("object" | "any")) {
                        return true;
                    }
                    if let Some(ref r) = items.ref_type
                        && r.contains('.')
                    {
                        let domain_name = r.split('.').next().unwrap_or("");
                        return !should_generate(domain_name);
                    }
                } else {
                    return true; // array with no items => Vec<Value>
                }
            }
            false
        })
    };

    // Check types
    for ty in &domain.types {
        if matches!(ty.type_kind.as_str(), "object" | "any")
            && ty.properties.is_empty()
            && ty.enum_values.is_empty()
        {
            return true;
        }
        if check_props(&ty.properties) {
            return true;
        }
    }
    // Check commands
    for cmd in &domain.commands {
        if check_props(&cmd.parameters) || check_props(&cmd.returns) {
            return true;
        }
    }
    // Check events
    for evt in &domain.events {
        if check_props(&evt.parameters) {
            return true;
        }
    }
    false
}

// ── Type generation ──

fn generate_type(out: &mut String, ty: &TypeDef, domain: &str, aliases: &FieldAliases) {
    write_doc_comment(out, &ty.description, 0);

    if !ty.enum_values.is_empty() {
        generate_enum(out, &ty.id, &ty.enum_values);
        return;
    }

    match ty.type_kind.as_str() {
        "object" if !ty.properties.is_empty() => {
            generate_struct(out, &ty.id, &ty.properties, domain, aliases);
        }
        "object" => {
            // Empty object — alias to Value
            writeln!(out, "pub type {} = Value;", ty.id).unwrap();
            writeln!(out).unwrap();
        }
        "string" | "binary" => {
            writeln!(out, "pub type {} = String;", ty.id).unwrap();
            writeln!(out).unwrap();
        }
        "integer" => {
            writeln!(out, "pub type {} = i64;", ty.id).unwrap();
            writeln!(out).unwrap();
        }
        "number" => {
            writeln!(out, "pub type {} = f64;", ty.id).unwrap();
            writeln!(out).unwrap();
        }
        "boolean" => {
            writeln!(out, "pub type {} = bool;", ty.id).unwrap();
            writeln!(out).unwrap();
        }
        "array" => {
            let elem = resolve_item_type(ty.items.as_ref(), domain);
            writeln!(out, "pub type {} = Vec<{}>;", ty.id, elem).unwrap();
            writeln!(out).unwrap();
        }
        "any" => {
            writeln!(out, "pub type {} = Value;", ty.id).unwrap();
            writeln!(out).unwrap();
        }
        _ => {
            writeln!(out, "pub type {} = Value;", ty.id).unwrap();
            writeln!(out).unwrap();
        }
    }
}

fn generate_enum(out: &mut String, name: &str, variants: &[String]) {
    writeln!(
        out,
        "#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]"
    )
    .unwrap();
    writeln!(out, "pub enum {name} {{").unwrap();
    for v in variants {
        let rust_variant = enum_variant_name(v);
        if rust_variant != *v {
            writeln!(out, "    #[serde(rename = \"{v}\")]").unwrap();
        }
        writeln!(out, "    {rust_variant},").unwrap();
    }
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();
}

/// Known self-referencing fields that require Box to break infinite size.
fn is_recursive_field(struct_name: &str, field_ref: Option<&str>) -> bool {
    matches!(
        (struct_name, field_ref),
        ("Node", Some("Node"))
            | ("StackTrace", Some("StackTrace"))
            | ("StackTraceId", Some("StackTraceId"))
    )
}

/// Check if all fields in a struct are optional (support Default derive).
fn all_fields_optional(properties: &[Property]) -> bool {
    properties.iter().all(|p| p.optional)
}

fn generate_struct(
    out: &mut String,
    name: &str,
    properties: &[Property],
    domain: &str,
    aliases: &FieldAliases,
) {
    if all_fields_optional(properties) {
        writeln!(
            out,
            "#[derive(Debug, Clone, Serialize, Deserialize, Default)]"
        )
        .unwrap();
    } else {
        writeln!(out, "#[derive(Debug, Clone, Serialize, Deserialize)]").unwrap();
    }
    writeln!(out, "#[serde(rename_all = \"camelCase\")]").unwrap();
    writeln!(out, "pub struct {name} {{").unwrap();
    for prop in properties {
        write_doc_comment(out, &prop.description, 1);
        let rust_name = to_snake_case(&prop.name);
        let raw_type = resolve_property_type(prop, domain);
        let needs_box = is_recursive_field(name, prop.ref_type.as_deref());
        let rust_type = if needs_box && prop.optional {
            format!("Option<Box<{raw_type}>>")
        } else if needs_box {
            format!("Box<{raw_type}>")
        } else if prop.optional {
            format!("Option<{raw_type}>")
        } else {
            raw_type
        };
        // HTTP endpoint aliases (from http_endpoints.json)
        let alias_key = (domain.to_string(), name.to_string(), prop.name.clone());
        if let Some(http_field) = aliases.get(&alias_key) {
            writeln!(out, "    #[serde(alias = \"{http_field}\")]").unwrap();
        }
        // Rename if snake_case differs from camelCase serde expectation
        if needs_serde_rename(&prop.name, &rust_name) {
            writeln!(out, "    #[serde(rename = \"{}\")]", prop.name).unwrap();
        }
        if prop.optional {
            writeln!(
                out,
                "    #[serde(default, skip_serializing_if = \"Option::is_none\")]"
            )
            .unwrap();
        }
        writeln!(out, "    pub {rust_name}: {rust_type},").unwrap();
    }
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();
}

// ── Command types ──

fn generate_command_types(out: &mut String, cmd: &Command, domain: &str) {
    let pascal = to_pascal_case(&cmd.name);

    // Params struct (only if there are parameters)
    if !cmd.parameters.is_empty() {
        write_doc_comment(
            out,
            &format!("Parameters for `{}.{}`.", domain, cmd.name),
            0,
        );
        // Only derive Default if all required fields have Default-implementing types
        let all_defaultable = cmd
            .parameters
            .iter()
            .all(|p| p.optional || is_default_type(p));
        if all_defaultable {
            writeln!(
                out,
                "#[derive(Debug, Clone, Serialize, Deserialize, Default)]"
            )
            .unwrap();
        } else {
            writeln!(out, "#[derive(Debug, Clone, Serialize, Deserialize)]").unwrap();
        }
        writeln!(out, "#[serde(rename_all = \"camelCase\")]").unwrap();
        writeln!(out, "pub struct {pascal}Params {{").unwrap();
        for prop in &cmd.parameters {
            write_doc_comment(out, &prop.description, 1);
            let rust_name = to_snake_case(&prop.name);
            let rust_type = resolve_property_type(prop, domain);
            let rust_type = if prop.optional {
                format!("Option<{rust_type}>")
            } else {
                rust_type
            };
            if needs_serde_rename(&prop.name, &rust_name) {
                writeln!(out, "    #[serde(rename = \"{}\")]", prop.name).unwrap();
            }
            if prop.optional {
                writeln!(
                    out,
                    "    #[serde(skip_serializing_if = \"Option::is_none\")]"
                )
                .unwrap();
            }
            writeln!(out, "    pub {rust_name}: {rust_type},").unwrap();
        }
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();
    }

    // Returns struct (only if there are return values)
    if !cmd.returns.is_empty() {
        write_doc_comment(
            out,
            &format!("Return type for `{}.{}`.", domain, cmd.name),
            0,
        );
        writeln!(out, "#[derive(Debug, Clone, Deserialize)]").unwrap();
        writeln!(out, "#[serde(rename_all = \"camelCase\")]").unwrap();
        writeln!(out, "pub struct {pascal}Returns {{").unwrap();
        for prop in &cmd.returns {
            write_doc_comment(out, &prop.description, 1);
            let rust_name = to_snake_case(&prop.name);
            let rust_type = resolve_property_type(prop, domain);
            let rust_type = if prop.optional {
                format!("Option<{rust_type}>")
            } else {
                rust_type
            };
            if needs_serde_rename(&prop.name, &rust_name) {
                writeln!(out, "    #[serde(rename = \"{}\")]", prop.name).unwrap();
            }
            if prop.optional {
                writeln!(out, "    #[serde(default)]").unwrap();
            }
            writeln!(out, "    pub {rust_name}: {rust_type},").unwrap();
        }
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();
    }
}

// ── Event types ──

fn generate_event_type(out: &mut String, evt: &Event, domain: &str) {
    if evt.parameters.is_empty() {
        return;
    }
    let pascal = to_pascal_case(&evt.name);
    write_doc_comment(
        out,
        &format!("Event payload for `{}.{}`.", domain, evt.name),
        0,
    );
    writeln!(out, "#[derive(Debug, Clone, Deserialize)]").unwrap();
    writeln!(out, "#[serde(rename_all = \"camelCase\")]").unwrap();
    writeln!(out, "pub struct {pascal}Event {{").unwrap();
    for prop in &evt.parameters {
        write_doc_comment(out, &prop.description, 1);
        let rust_name = to_snake_case(&prop.name);
        let rust_type = resolve_property_type(prop, domain);
        let rust_type = if prop.optional {
            format!("Option<{rust_type}>")
        } else {
            rust_type
        };
        if needs_serde_rename(&prop.name, &rust_name) {
            writeln!(out, "    #[serde(rename = \"{}\")]", prop.name).unwrap();
        }
        if prop.optional {
            writeln!(out, "    #[serde(default)]").unwrap();
        }
        writeln!(out, "    pub {rust_name}: {rust_type},").unwrap();
    }
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();
}

// ── Method signatures ──

fn generate_method_signature(out: &mut String, cmd: &Command, domain: &str, snake_domain: &str) {
    let pascal = to_pascal_case(&cmd.name);
    let method_name = format!("{}_{}", snake_domain, to_snake_case(&cmd.name));
    let cdp_method = format!("{domain}.{}", cmd.name);

    // Use regular comments (not doc comments) for method signatures
    if !cmd.description.is_empty() {
        for line in cmd.description.lines() {
            let trimmed = line.trim_end();
            if trimmed.is_empty() {
                writeln!(out, "//").unwrap();
            } else {
                writeln!(out, "// {trimmed}").unwrap();
            }
        }
    }

    let params_type = if cmd.parameters.is_empty() {
        None
    } else {
        Some(format!("{pascal}Params"))
    };
    let returns_type = if cmd.returns.is_empty() {
        "()".to_string()
    } else {
        format!("{pascal}Returns")
    };

    match params_type {
        Some(pt) => {
            writeln!(
                out,
                "// async fn {method_name}(&self, params: {pt}) -> Result<{returns_type}>"
            )
            .unwrap();
        }
        None => {
            writeln!(
                out,
                "// async fn {method_name}(&self) -> Result<{returns_type}>"
            )
            .unwrap();
        }
    }
    writeln!(out, "// CDP: \"{cdp_method}\"").unwrap();
    writeln!(out).unwrap();
}

// ── Helpers ──

/// Check if a required property has a type that implements Default.
fn is_default_type(prop: &Property) -> bool {
    if prop.ref_type.is_some() {
        return false; // Refs to structs/enums may not implement Default
    }
    matches!(
        prop.type_kind.as_deref(),
        Some("string" | "binary" | "boolean" | "integer" | "number" | "array" | "object" | "any")
    )
}

fn resolve_property_type(prop: &Property, current_domain: &str) -> String {
    // Inline enum
    if !prop.enum_values.is_empty() {
        return "String".to_string(); // Inline enums stay as String for simplicity
    }

    if let Some(ref r) = prop.ref_type {
        return resolve_ref(r, current_domain);
    }

    match prop.type_kind.as_deref() {
        Some("string") => "String".to_string(),
        Some("binary") => "String".to_string(), // CDP binary = base64-encoded string
        Some("integer") => "i64".to_string(),
        Some("number") => "f64".to_string(),
        Some("boolean") => "bool".to_string(),
        Some("object") => "Value".to_string(),
        Some("any") => "Value".to_string(),
        Some("array") => {
            let elem = resolve_item_type(prop.items.as_ref(), current_domain);
            format!("Vec<{elem}>")
        }
        _ => "Value".to_string(),
    }
}

fn resolve_item_type(items: Option<&ItemRef>, current_domain: &str) -> String {
    match items {
        Some(item) => {
            if let Some(ref r) = item.ref_type {
                resolve_ref(r, current_domain)
            } else {
                match item.type_kind.as_deref() {
                    Some("string" | "binary") => "String".to_string(),
                    Some("integer") => "i64".to_string(),
                    Some("number") => "f64".to_string(),
                    Some("boolean") => "bool".to_string(),
                    Some("object") => "Value".to_string(),
                    Some("any") => "Value".to_string(),
                    _ => "Value".to_string(),
                }
            }
        }
        None => "Value".to_string(),
    }
}

fn resolve_ref(ref_str: &str, _current_domain: &str) -> String {
    if ref_str.contains('.') {
        // Cross-domain ref: "Network.LoaderId" -> "super::network::LoaderId"
        let parts: Vec<&str> = ref_str.splitn(2, '.').collect();
        let domain = parts[0];
        // Only resolve refs to domains we actually generate
        if should_generate(domain) {
            let domain_snake = to_snake_case(domain);
            let type_name = parts[1];
            format!("super::{domain_snake}::{type_name}")
        } else {
            // Domain not generated — fall back to Value
            "Value".to_string()
        }
    } else {
        // Same-domain ref: just the type name
        ref_str.to_string()
    }
}

fn write_doc_comment(out: &mut String, desc: &str, indent: usize) {
    if desc.is_empty() {
        return;
    }
    let prefix = "    ".repeat(indent);
    for line in desc.lines() {
        let escaped = escape_doc_comment(line);
        let trimmed = escaped.trim_end();
        if trimmed.is_empty() {
            writeln!(out, "{prefix}///").unwrap();
        } else {
            writeln!(out, "{prefix}/// {trimmed}").unwrap();
        }
    }
}

/// Escape doc comment content to prevent rustdoc warnings.
///
/// - Brackets like `[0,359]` are escaped to `\[0,359\]` (rustdoc sees them as links)
/// - Bare URLs are wrapped in angle brackets `<url>`
fn escape_doc_comment(line: &str) -> String {
    let mut result = String::with_capacity(line.len() + 8);
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '[' {
            // Check if this looks like a markdown link [text](url) — leave those alone
            // Otherwise escape the brackets
            if let Some(close) = chars[i..].iter().position(|&c| c == ']') {
                let inner = &line[i + 1..i + close];
                let after_close = i + close + 1;
                if after_close < chars.len() && chars[after_close] == '(' {
                    // Markdown link — don't escape
                    result.push('[');
                    i += 1;
                    continue;
                }
                // Not a link — escape brackets
                result.push('\\');
                result.push('[');
                result.push_str(inner);
                result.push('\\');
                result.push(']');
                i += close + 1;
                continue;
            }
        }
        // Escape bare URLs: http:// or https://
        if i + 8 < chars.len()
            && (line[i..].starts_with("http://") || line[i..].starts_with("https://"))
        {
            // Find end of URL (space, comma, or end of line)
            let url_start = i;
            while i < chars.len() && !chars[i].is_whitespace() && chars[i] != ',' {
                i += 1;
            }
            let url = &line[url_start..i];
            result.push('<');
            result.push_str(url);
            result.push('>');
            continue;
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}

fn needs_serde_rename(original: &str, snake: &str) -> bool {
    // If the original camelCase doesn't match what rename_all = "camelCase" would produce
    // from the snake_case name, we need an explicit rename.
    // Common case: acronyms like "DOM", "URL", "CSS" in the original.
    let reconstructed = snake_to_camel(snake);
    reconstructed != original
}

fn snake_to_camel(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;
    for (i, c) in s.chars().enumerate() {
        if c == '_' {
            capitalize_next = true;
        } else if i == 0 {
            result.push(c);
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    result
}

/// Convert PascalCase or camelCase to snake_case.
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    let chars: Vec<char> = s.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_ascii_uppercase() {
            // Insert underscore before uppercase, unless:
            // - It's the first character
            // - Previous char was already uppercase AND next is also uppercase (acronym middle)
            if i > 0 {
                let prev_upper = chars[i - 1].is_ascii_uppercase();
                let next_upper = chars.get(i + 1).is_none_or(|n| n.is_ascii_uppercase());
                if !prev_upper || !next_upper {
                    result.push('_');
                }
            }
            result.push(c.to_ascii_lowercase());
        } else if c == '-' {
            result.push('_');
        } else {
            result.push(c);
        }
    }
    // Handle Rust keywords
    match result.as_str() {
        "type" => "r#type".to_string(),
        "match" => "r#match".to_string(),
        "mod" => "r#mod".to_string(),
        "ref" => "r#ref".to_string(),
        "self" => "self_".to_string(),
        "super" => "super_".to_string(),
        _ => result,
    }
}

/// Convert snake_case or camelCase to PascalCase.
fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;
    for c in s.chars() {
        if c == '_' || c == '-' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    result
}

/// Convert a CDP enum variant string to a valid Rust identifier.
fn enum_variant_name(s: &str) -> String {
    if s.is_empty() {
        return "Empty".to_string();
    }
    // Handle kebab-case, spaces, dots
    let mut result = String::new();
    let mut capitalize_next = true;
    for c in s.chars() {
        if c == '-' || c == '_' || c == ' ' || c == '.' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    // Ensure starts with letter
    if result.chars().next().is_none_or(|c| c.is_ascii_digit()) {
        result.insert(0, 'V');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snake_case() {
        assert_eq!(to_snake_case("nodeId"), "node_id");
        assert_eq!(to_snake_case("DOM"), "dom");
        assert_eq!(to_snake_case("getDocument"), "get_document");
        assert_eq!(to_snake_case("backendDOMNodeId"), "backend_dom_node_id");
        assert_eq!(
            to_snake_case("CSSStyleSheetHeader"),
            "css_style_sheet_header"
        );
        assert_eq!(to_snake_case("type"), "r#type");
        assert_eq!(to_snake_case("XMLHttpRequest"), "xml_http_request");
    }

    #[test]
    fn test_pascal_case() {
        assert_eq!(to_pascal_case("navigate"), "Navigate");
        assert_eq!(to_pascal_case("get_document"), "GetDocument");
        assert_eq!(to_pascal_case("capture-screenshot"), "CaptureScreenshot");
    }

    #[test]
    fn test_enum_variant() {
        assert_eq!(enum_variant_name("link"), "Link");
        assert_eq!(enum_variant_name("auto-subframe"), "AutoSubframe");
        assert_eq!(enum_variant_name("3d"), "V3d");
        assert_eq!(enum_variant_name(""), "Empty");
    }
}
