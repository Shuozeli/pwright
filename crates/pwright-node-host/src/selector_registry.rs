//! Bounded Playwright-scoped custom selector-engine registration.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use pwright_backend_cdp_selector_runtime::compile_selector;
use pwright_cap_locator::Selector;
pub use pwright_cap_locator::TestIdAttributes;

use crate::HostError;
use crate::locator_query::{PLAYWRIGHT_BUILTIN_ENGINES, SelectorPart, playwright_selector_part};

pub const MAX_SELECTOR_ENGINES: usize = 64;
pub const MAX_SELECTOR_ENGINE_NAME_BYTES: usize = 64;
pub const MAX_SELECTOR_ENGINE_SOURCE_BYTES: usize = 1024 * 1024;
pub const MAX_SELECTOR_REGISTRY_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_REGISTERED_SELECTOR_PARTS: usize = 256;
pub const MAX_REGISTERED_SELECTOR_PROGRAM_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_TEST_ID_ATTRIBUTE_BYTES: usize = 1024;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SelectorRegistryMetrics {
    pub engines: usize,
    pub retained_bytes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegisteredSelectorEngine {
    name: Arc<str>,
    source: Arc<str>,
    content_script: bool,
    retained_bytes: usize,
}

impl RegisteredSelectorEngine {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub const fn content_script(&self) -> bool {
        self.content_script
    }

    pub const fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum RegisteredSelectorPart {
    WithinStandard(Selector),
    WithinCustom {
        engine: RegisteredSelectorEngine,
        body: String,
    },
    And(Selector),
    Nth(i32),
    Visible(bool),
}

/// Immutable selector plan that borrows engine source ownership through
/// registry-backed `Arc`s. The public selector text is copied at most once
/// into bounded part bodies; engine source is not duplicated per query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegisteredSelectorProgram {
    parts: Arc<[RegisteredSelectorPart]>,
    selector_bytes: usize,
    requires_main_world: bool,
}

impl RegisteredSelectorProgram {
    pub const fn selector_bytes(&self) -> usize {
        self.selector_bytes
    }

    pub fn part_count(&self) -> usize {
        self.parts.len()
    }

    pub const fn requires_main_world(&self) -> bool {
        self.requires_main_world
    }

    /// Add Page's default first-match selection to the same registered
    /// browser program. The engine query and the eventual action remain in
    /// one browser callback, with no second selector query or microtask gap.
    pub(crate) fn first(&self) -> Result<Self, HostError> {
        if self.parts.len() >= MAX_REGISTERED_SELECTOR_PARTS {
            return Err(HostError::ResourceExhausted(format!(
                "registered selector exceeds {MAX_REGISTERED_SELECTOR_PARTS} parts"
            )));
        }
        let mut parts = Vec::with_capacity(self.parts.len() + 1);
        parts.extend(self.parts.iter().cloned());
        parts.push(RegisteredSelectorPart::Nth(0));
        Ok(Self {
            parts: parts.into(),
            selector_bytes: self.selector_bytes,
            requires_main_world: self.requires_main_world,
        })
    }

    /// Compile one browser-world program. Every distinct registered engine is
    /// emitted once, even when the selector references it repeatedly.
    pub fn compile(&self, root: &str, query_all: bool) -> Result<String, HostError> {
        let mut engines = BTreeMap::<&str, &RegisteredSelectorEngine>::new();
        for part in self.parts.iter() {
            if let RegisteredSelectorPart::WithinCustom { engine, .. } = part {
                engines.entry(engine.name()).or_insert(engine);
            }
        }

        let mut declarations = String::new();
        let mut identifiers = BTreeMap::<&str, String>::new();
        for (index, (name, engine)) in engines.into_iter().enumerate() {
            let identifier = format!("__pwEngine{index}");
            declarations.push_str(&format!("const {identifier} = ({});\n", engine.source()));
            identifiers.insert(name, identifier);
        }

        let mut parts = self.parts.iter();
        let first = parts.next().ok_or_else(|| {
            HostError::InvalidArgument("registered selector plan is empty".into())
        })?;
        let mut remaining = parts.len();
        let mut expression = compile_part(first, root, &identifiers, query_all || remaining > 0)?;
        for part in parts {
            remaining = remaining.saturating_sub(1);
            expression = match part {
                RegisteredSelectorPart::And(selector) => {
                    let right = compile_selector(selector, root).map_err(HostError::backend)?;
                    format!(
                        "(() => {{ const __pwRight = new Set({right}); return ({expression}).filter(__pwElement => __pwRight.has(__pwElement)); }})()"
                    )
                }
                RegisteredSelectorPart::Nth(index) => {
                    format!(
                        "(() => {{ const values = {expression}; let index = {index}; if (index < 0) index += values.length; return index >= 0 && index < values.length ? [values[index]] : []; }})()"
                    )
                }
                RegisteredSelectorPart::Visible(visible) => {
                    format!(
                        "({expression}).filter(__pwElement => __pw.visible(__pwElement) === {visible})"
                    )
                }
                _ => {
                    let descendant =
                        compile_part(part, "__pwRoot", &identifiers, query_all || remaining > 0)?;
                    format!("__pw.within({expression}, __pwRoot => {descendant})")
                }
            };
        }

        let program = format!(
            r#"(() => {{
  const __pwRegisteredQuery = (engine, queryRoot, body, all) => {{
    const result = all && typeof engine.queryAll === 'function'
      ? engine.queryAll(queryRoot, body)
      : [engine.query(queryRoot, body)];
    return Array.from(result || []).filter(Boolean);
  }};
  {declarations}return {expression};
}})()"#
        );
        if program.len() > MAX_REGISTERED_SELECTOR_PROGRAM_BYTES {
            return Err(HostError::ResourceExhausted(format!(
                "registered selector program exceeds {MAX_REGISTERED_SELECTOR_PROGRAM_BYTES} UTF-8 bytes"
            )));
        }
        Ok(program)
    }
}

fn compile_part(
    part: &RegisteredSelectorPart,
    root: &str,
    identifiers: &BTreeMap<&str, String>,
    query_all: bool,
) -> Result<String, HostError> {
    match part {
        RegisteredSelectorPart::WithinStandard(selector) => {
            compile_selector(selector, root).map_err(HostError::backend)
        }
        RegisteredSelectorPart::WithinCustom { engine, body } => {
            let identifier = identifiers.get(engine.name()).ok_or_else(|| {
                HostError::Backend("registered selector engine declaration is missing".into())
            })?;
            let body = serde_json::to_string(body).map_err(HostError::backend)?;
            Ok(format!(
                "__pwRegisteredQuery({identifier}, {root}, {body}, {query_all})"
            ))
        }
        RegisteredSelectorPart::And(_)
        | RegisteredSelectorPart::Nth(_)
        | RegisteredSelectorPart::Visible(_) => Err(HostError::InvalidArgument(
            "registered selector cannot begin with a filter".into(),
        )),
    }
}

#[derive(Default)]
struct RegistryState {
    engines: BTreeMap<Arc<str>, RegisteredSelectorEngine>,
    retained_bytes: usize,
    test_id_attributes: TestIdAttributes,
}

#[derive(Clone, Default)]
pub struct SelectorRegistry(Arc<Mutex<RegistryState>>);

impl SelectorRegistry {
    /// Replace the Playwright-scoped test-id attribute after validating and
    /// bounding it. Semantic query construction takes an immutable snapshot;
    /// strict-error diagnostics read the current value at action time.
    pub fn set_test_id_attribute(&self, name: &str) -> Result<(), HostError> {
        if name.len() > MAX_TEST_ID_ATTRIBUTE_BYTES {
            return Err(HostError::ResourceExhausted(format!(
                "test-id attribute exceeds {MAX_TEST_ID_ATTRIBUTE_BYTES} UTF-8 bytes"
            )));
        }
        let attributes = TestIdAttributes::new(name)
            .map_err(|error| HostError::InvalidArgument(error.to_string()))?;
        self.0
            .lock()
            .map_err(HostError::backend)?
            .test_id_attributes = attributes;
        Ok(())
    }

    pub fn test_id_attributes(&self) -> Result<TestIdAttributes, HostError> {
        Ok(self
            .0
            .lock()
            .map_err(HostError::backend)?
            .test_id_attributes
            .clone())
    }

    /// Validates and reserves aggregate capacity before copying either input.
    pub fn register(
        &self,
        name: &str,
        source: &str,
        content_script: bool,
    ) -> Result<(), HostError> {
        validate_name(name)?;
        if source.is_empty() {
            return Err(registration_error(
                "selector engine source must not be empty",
            ));
        }
        if source.len() > MAX_SELECTOR_ENGINE_SOURCE_BYTES {
            return Err(HostError::ResourceExhausted(format!(
                "selector engine source exceeds {MAX_SELECTOR_ENGINE_SOURCE_BYTES} UTF-8 bytes"
            )));
        }

        let retained_bytes = name
            .len()
            .checked_add(source.len())
            .and_then(|bytes| bytes.checked_add(std::mem::size_of::<RegisteredSelectorEngine>()))
            .ok_or_else(|| {
                HostError::ResourceExhausted("selector registry byte accounting overflowed".into())
            })?;
        let mut state = self.0.lock().map_err(HostError::backend)?;
        if state.engines.contains_key(name) {
            return Err(registration_error(&format!(
                "\"{name}\" selector engine has been already registered"
            )));
        }
        if PLAYWRIGHT_BUILTIN_ENGINES.contains(&name) || matches!(name, "zs" | "zs:light") {
            return Err(registration_error(&format!(
                "\"{name}\" is a predefined selector engine"
            )));
        }
        if state.engines.len() >= MAX_SELECTOR_ENGINES
            || retained_bytes > MAX_SELECTOR_REGISTRY_BYTES.saturating_sub(state.retained_bytes)
        {
            return Err(HostError::ResourceExhausted(
                "selector registry engine or byte capacity exceeded".into(),
            ));
        }

        let name: Arc<str> = Arc::from(name);
        let engine = RegisteredSelectorEngine {
            name: Arc::clone(&name),
            source: Arc::from(source),
            content_script,
            retained_bytes,
        };
        state.retained_bytes += retained_bytes;
        state.engines.insert(name, engine);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Result<Option<RegisteredSelectorEngine>, HostError> {
        Ok(self
            .0
            .lock()
            .map_err(HostError::backend)?
            .engines
            .get(name)
            .cloned())
    }

    /// Parse only when at least one chain part names a registered engine.
    /// Returning `None` leaves builtin/unknown handling to the ordinary
    /// selector parser, preserving its exact error behavior.
    pub fn parse_program(
        &self,
        selector: &str,
    ) -> Result<Option<RegisteredSelectorProgram>, HostError> {
        let state = self.0.lock().map_err(HostError::backend)?;
        let raw_parts = selector.split(">>").map(str::trim).collect::<Vec<_>>();
        if raw_parts.is_empty() || raw_parts.len() > MAX_REGISTERED_SELECTOR_PARTS {
            return Err(HostError::ResourceExhausted(format!(
                "registered selector exceeds {MAX_REGISTERED_SELECTOR_PARTS} parts"
            )));
        }
        let has_registered = raw_parts.iter().any(|part| {
            explicit_engine(part)
                .and_then(|(name, _)| state.engines.get(name))
                .is_some()
        });
        if !has_registered {
            return Ok(None);
        }

        let mut parts = Vec::with_capacity(raw_parts.len());
        let mut requires_main_world = false;
        for raw in raw_parts {
            if let Some((name, body)) = explicit_engine(raw)
                && let Some(engine) = state.engines.get(name)
            {
                requires_main_world |= !engine.content_script();
                parts.push(RegisteredSelectorPart::WithinCustom {
                    engine: engine.clone(),
                    body: body.to_owned(),
                });
                continue;
            }
            match playwright_selector_part(raw) {
                Ok(SelectorPart::Within(selector)) => {
                    parts.push(RegisteredSelectorPart::WithinStandard(selector));
                }
                Ok(SelectorPart::And(selector)) => {
                    parts.push(RegisteredSelectorPart::And(selector));
                }
                Ok(SelectorPart::Nth(index)) => {
                    parts.push(RegisteredSelectorPart::Nth(index));
                }
                Ok(SelectorPart::Visible(visible)) => {
                    parts.push(RegisteredSelectorPart::Visible(visible));
                }
                Err(()) => {
                    return Err(HostError::InvalidArgument(
                        "registered selector contains an invalid builtin part".into(),
                    ));
                }
            }
        }
        Ok(Some(RegisteredSelectorProgram {
            parts: parts.into(),
            selector_bytes: selector.len(),
            requires_main_world,
        }))
    }

    pub fn metrics(&self) -> Result<SelectorRegistryMetrics, HostError> {
        let state = self.0.lock().map_err(HostError::backend)?;
        Ok(SelectorRegistryMetrics {
            engines: state.engines.len(),
            retained_bytes: state.retained_bytes,
        })
    }

    pub fn clear(&self) -> Result<(), HostError> {
        let mut state = self.0.lock().map_err(HostError::backend)?;
        state.engines.clear();
        state.retained_bytes = 0;
        Ok(())
    }
}

fn explicit_engine(part: &str) -> Option<(&str, &str)> {
    let (name, body) = part.split_once('=')?;
    let name = name.trim().strip_prefix('*').unwrap_or(name.trim());
    if name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return None;
    }
    Some((name, body.trim()))
}

fn validate_name(name: &str) -> Result<(), HostError> {
    if name.is_empty()
        || name.len() > MAX_SELECTOR_ENGINE_NAME_BYTES
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(registration_error(
            "Selector engine name may only contain [a-zA-Z0-9_] characters",
        ));
    }
    Ok(())
}

fn registration_error(message: &str) -> HostError {
    HostError::Backend(format!("Selectors.register: {message}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registration_is_case_sensitive_and_rejects_an_exact_duplicate() {
        // Arrange
        let registry = SelectorRegistry::default();
        registry.register("dummy", "() => ({})", false).unwrap();

        // Act
        registry.register("duMMy", "() => ({})", true).unwrap();
        let error = registry
            .register("dummy", "() => ({})", false)
            .expect_err("an exact duplicate must fail");

        // Assert
        assert!(matches!(
            error,
            HostError::Backend(message)
                if message == "Selectors.register: \"dummy\" selector engine has been already registered"
        ));
        assert_eq!(registry.metrics().unwrap().engines, 2);
        assert!(registry.get("duMMy").unwrap().unwrap().content_script());
    }

    #[test]
    fn invalid_and_predefined_names_preserve_playwright_errors() {
        // Arrange
        let registry = SelectorRegistry::default();

        // Act
        let invalid = registry
            .register("$", "() => ({})", false)
            .expect_err("invalid name must fail");
        let predefined = registry
            .register("css", "() => ({})", false)
            .expect_err("predefined name must fail");

        // Assert
        assert!(matches!(
            invalid,
            HostError::Backend(message)
                if message == "Selectors.register: Selector engine name may only contain [a-zA-Z0-9_] characters"
        ));
        assert!(matches!(
            predefined,
            HostError::Backend(message)
                if message == "Selectors.register: \"css\" is a predefined selector engine"
        ));
        assert_eq!(
            registry.metrics().unwrap(),
            SelectorRegistryMetrics::default()
        );
    }

    #[test]
    fn capacity_rejection_does_not_retain_the_rejected_source() {
        // Arrange
        let registry = SelectorRegistry::default();
        let source = "x".repeat(MAX_SELECTOR_ENGINE_SOURCE_BYTES + 1);

        // Act
        let error = registry
            .register("oversized", &source, false)
            .expect_err("oversized source must fail before retention");

        // Assert
        assert!(matches!(error, HostError::ResourceExhausted(_)));
        assert_eq!(
            registry.metrics().unwrap(),
            SelectorRegistryMetrics::default()
        );
    }

    #[test]
    fn clear_releases_all_registry_ownership() {
        // Arrange
        let registry = SelectorRegistry::default();
        registry.register("tag", "() => ({})", false).unwrap();
        assert_ne!(
            registry.metrics().unwrap(),
            SelectorRegistryMetrics::default()
        );

        // Act
        registry.clear().unwrap();

        // Assert
        assert_eq!(
            registry.metrics().unwrap(),
            SelectorRegistryMetrics::default()
        );
    }

    #[test]
    fn test_id_attribute_is_bounded_and_replaces_the_environment_value() {
        // Arrange
        let registry = SelectorRegistry::default();

        // Act
        registry.set_test_id_attribute("data-custom-id").unwrap();
        let attributes = registry.test_id_attributes().unwrap();
        let oversized = registry
            .set_test_id_attribute(&"x".repeat(MAX_TEST_ID_ATTRIBUTE_BYTES.saturating_add(1)));

        // Assert
        assert_eq!(attributes.as_csv(), "data-custom-id");
        assert!(matches!(oversized, Err(HostError::ResourceExhausted(_))));
        assert_eq!(
            registry.test_id_attributes().unwrap().as_csv(),
            "data-custom-id"
        );
    }

    #[test]
    fn registered_program_compiles_each_engine_once_and_preserves_chain_order() {
        // Arrange
        let registry = SelectorRegistry::default();
        registry
            .register(
                "tag",
                "({ queryAll(root, selector) { return root.querySelectorAll(selector); } })",
                false,
            )
            .unwrap();

        // Act
        let program = registry
            .parse_program("tag=DIV >> visible=true >> css=span >> tag=SECTION")
            .unwrap()
            .expect("registered engine should select the registered plan");
        let source = program.compile("document", true).unwrap();

        // Assert
        assert_eq!(program.part_count(), 4);
        assert_eq!(
            program.selector_bytes(),
            "tag=DIV >> visible=true >> css=span >> tag=SECTION".len()
        );
        assert!(program.requires_main_world());
        assert_eq!(source.matches("const __pwEngine0 =").count(), 1);
        assert_eq!(source.matches("__pwRegisteredQuery(__pwEngine0").count(), 2);
        assert!(source.contains("__pw.visible(__pwElement) === true"));
        assert!(source.contains("__pw.css(__pwRoot, \"span\")"));
        assert!(source.len() <= MAX_REGISTERED_SELECTOR_PROGRAM_BYTES);
    }

    #[test]
    fn registered_program_applies_playwright_nth_to_the_current_recipe() {
        // Arrange
        let registry = SelectorRegistry::default();
        registry
            .register(
                "tag",
                "({ queryAll(root, selector) { return root.querySelectorAll(selector); } })",
                false,
            )
            .unwrap();

        // Act
        let program = registry
            .parse_program("tag=OPTION >> nth=-1")
            .unwrap()
            .expect("registered engine should select the registered plan");
        let source = program.compile("document", true).unwrap();

        // Assert
        assert_eq!(program.part_count(), 2);
        assert!(source.contains("let index = -1"));
        assert!(source.contains("index += values.length"));
        assert!(source.contains("[values[index]]"));
    }

    #[test]
    fn registered_first_keeps_query_and_selection_in_one_program() {
        // Arrange
        let registry = SelectorRegistry::default();
        registry
            .register(
                "tag",
                "({ query(root, selector) { return root.querySelector(selector); }, queryAll(root, selector) { return root.querySelectorAll(selector); } })",
                false,
            )
            .unwrap();
        let original = registry
            .parse_program("tag=button")
            .unwrap()
            .expect("registered engine should produce a plan");

        // Act
        let first = original.first().unwrap();
        let source = first.compile("document", false).unwrap();

        // Assert
        assert_eq!(original.part_count(), 1);
        assert_eq!(first.part_count(), 2);
        assert_eq!(first.selector_bytes(), original.selector_bytes());
        assert!(first.requires_main_world());
        assert_eq!(source.matches("__pwRegisteredQuery(__pwEngine0").count(), 1);
        assert!(source.contains("\"button\", true)"));
        assert!(source.contains("let index = 0"));
        assert!(!source.contains("await"));
    }

    #[test]
    fn content_script_program_stays_eligible_for_the_utility_world() {
        // Arrange
        let registry = SelectorRegistry::default();
        registry
            .register("isolated", "({ queryAll() { return []; } })", true)
            .unwrap();

        // Act
        let program = registry
            .parse_program("isolated=ignored")
            .unwrap()
            .expect("registered engine should produce a plan");

        // Assert
        assert!(!program.requires_main_world());
        assert!(
            program
                .compile("document", false)
                .unwrap()
                .contains("ignored")
        );
    }

    #[test]
    fn any_main_world_engine_forces_both_chain_orders_into_main() {
        // Arrange
        let registry = SelectorRegistry::default();
        registry
            .register("main", "({ queryAll() { return []; } })", false)
            .unwrap();
        registry
            .register("isolated", "({ queryAll() { return []; } })", true)
            .unwrap();

        // Act
        let main_first = registry
            .parse_program("main=ignored >> isolated=ignored")
            .unwrap()
            .unwrap();
        let isolated_first = registry
            .parse_program("isolated=ignored >> main=ignored")
            .unwrap()
            .unwrap();

        // Assert
        assert!(main_first.requires_main_world());
        assert!(isolated_first.requires_main_world());
    }

    #[test]
    fn terminal_custom_engine_queries_each_preceding_builtin_match_as_its_root() {
        // Arrange
        let registry = SelectorRegistry::default();
        registry
            .register(
                "__value",
                "({ query(root, selector) { return root.value.includes(selector) ? root : undefined; }, queryAll() { throw new Error('unused'); } })",
                false,
            )
            .unwrap();

        // Act
        let source = registry
            .parse_program("input >> __value=value2")
            .unwrap()
            .unwrap()
            .compile("document", false)
            .unwrap();

        // Assert
        assert!(source.contains("__pw.css(document, \"input\")"));
        assert!(source.contains("__pw.within("));
        assert!(source.contains("__pwRegisteredQuery(__pwEngine0, __pwRoot, \"value2\", false)"));
    }

    #[test]
    fn ordinary_selector_is_left_to_the_builtin_parser() {
        // Arrange
        let registry = SelectorRegistry::default();
        registry
            .register("tag", "({ queryAll() { return []; } })", false)
            .unwrap();

        // Act
        let program = registry.parse_program("css=div").unwrap();

        // Assert
        assert_eq!(program, None);
    }
}
