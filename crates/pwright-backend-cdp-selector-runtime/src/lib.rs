//! Command-free Chromium selector compiler and injected-page runtime.
//!
//! This crate owns selector validation, bounded compilation, and the shared
//! web-platform runtime source. It owns no Pwright command, capability,
//! transport, browser process, or public evaluation surface.

use pwright_core::{FailureKind, OperationError};
use pwright_selector_model::{
    AriaTriState, LocatorErrorDetails, RegexMatcher, RoleOptions, Selector, TestIdAttributes,
    TextMatcher,
};

pub const MAX_SELECTOR_DEPTH: usize = 32;
pub const MAX_SELECTOR_NODES: usize = 256;
pub const MAX_SELECTOR_STRING_BYTES: usize = 64 * 1024;
pub const SELECTOR_RUNTIME_VERSION: u32 = 6;
pub const SELECTOR_RUNTIME_SYMBOL: &str = "pwright.selector-runtime/6";
const MAX_ESCAPED_BYTES_PER_INPUT_BYTE: usize = 6;
const MAX_GENERATED_BYTES_PER_SELECTOR_NODE: usize = 1024;
const MAX_ACTION_WRAPPER_BYTES: usize = 32 * 1024;
const MAX_INSTALL_WRAPPER_BYTES: usize = 512;

pub const MAX_COMPILED_SELECTOR_BYTES: usize = MAX_SELECTOR_STRING_BYTES
    * MAX_ESCAPED_BYTES_PER_INPUT_BYTE
    + MAX_SELECTOR_NODES * MAX_GENERATED_BYTES_PER_SELECTOR_NODE
    + MAX_ACTION_WRAPPER_BYTES;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SelectorStorage {
    string_capacity: usize,
}

impl SelectorStorage {
    pub const fn string_capacity(self) -> usize {
        self.string_capacity
    }
}

pub fn measure_selector(
    selector: &Selector,
) -> Result<SelectorStorage, OperationError<LocatorErrorDetails>> {
    let mut stats = SelectorStats::default();
    measure_selector_storage_inner(selector, 0, &mut stats)?;
    Ok(SelectorStorage {
        string_capacity: stats.string_capacity,
    })
}

pub fn compile_selector(
    selector: &Selector,
    root: &str,
) -> Result<String, OperationError<LocatorErrorDetails>> {
    let mut stats = SelectorStats::default();
    let compiled = compile_selector_inner(selector, root, 0, &mut stats)?;
    if compiled.len() > MAX_COMPILED_SELECTOR_BYTES {
        Err(selector_resource_error(
            "compiled selector exceeds its declared backend bound",
        ))
    } else {
        Ok(compiled)
    }
}

pub fn selector_runtime_install_script() -> String {
    format!(
        r#"(() => {{
  const key = Symbol.for({runtime_symbol:?});
  const current = globalThis[key];
  if (!current || current.version !== {runtime_version}) {{
    Object.defineProperty(globalThis, key, {{
      value: {SELECTOR_RUNTIME},
      writable: false,
      enumerable: false,
      configurable: true
    }});
  }}
  return true;
}})()"#,
        runtime_symbol = SELECTOR_RUNTIME_SYMBOL,
        runtime_version = SELECTOR_RUNTIME_VERSION,
    )
}

// This runtime is backend-owned rather than part of the portable capability.
// It deliberately uses only web-platform APIs and traverses open shadow roots.
pub const SELECTOR_RUNTIME: &str = r#"(() => {
  const normalize = value => String(value ?? '').replace(/\s+/g, ' ').trim();
  const matchesText = (actual, expected, exact) => {
    actual = normalize(actual);
    expected = normalize(expected);
    return exact ? actual === expected : actual.toLowerCase().includes(expected.toLowerCase());
  };
  const queryRoots = root => {
    const roots = [root];
    for (let i = 0; i < roots.length; ++i) {
      const current = roots[i];
      if (current.nodeType === Node.ELEMENT_NODE && current.shadowRoot)
        roots.push(current.shadowRoot);
      for (const element of current.querySelectorAll('*')) {
        if (element.shadowRoot)
          roots.push(element.shadowRoot);
      }
    }
    return roots;
  };
  const all = root => queryRoots(root).flatMap(current => Array.from(current.querySelectorAll('*')));
  const composedContains = (ancestor, node) => {
    for (let current = node; current;) {
      if (current === ancestor)
        return true;
      current = current.parentElement || (current.getRootNode && current.getRootNode().host) || null;
    }
    return false;
  };
  const css = (root, selector) => queryRoots(root).flatMap(current => Array.from(current.querySelectorAll(selector)));
  const text = (root, expected, exact) => {
    const matches = all(root).filter(element =>
      !['SCRIPT', 'STYLE', 'NOSCRIPT', 'TEMPLATE'].includes(element.tagName) &&
      matchesText(element.textContent, expected, exact));
    return matches.filter(element => !matches.some(other => other !== element && composedContains(element, other)));
  };
  const textRegex = (root, source, flags) => {
    const matcher = new RegExp(source, flags);
    const matches = all(root).filter(element =>
      !['SCRIPT', 'STYLE', 'NOSCRIPT', 'TEMPLATE'].includes(element.tagName) &&
      matcher.test(String(element.textContent ?? '')));
    return matches.filter(element => !matches.some(other => other !== element && composedContains(element, other)));
  };
  const referencedText = (element, attribute) => normalize((element.getAttribute(attribute) || '')
    .split(/\s+/).filter(Boolean).map(id => element.ownerDocument.getElementById(id)?.textContent || '').join(' '));
  const labelTexts = element => {
    const values = [];
    const ariaLabelledBy = referencedText(element, 'aria-labelledby');
    if (ariaLabelledBy) values.push(ariaLabelledBy);
    if (element.hasAttribute('aria-label')) values.push(normalize(element.getAttribute('aria-label')));
    if (element.labels) {
      for (const label of element.labels)
        values.push(normalize(label.textContent));
    }
    return values;
  };
  const label = (root, expected, exact) => all(root).filter(element =>
    labelTexts(element).some(value => matchesText(value, expected, exact)));
  const labelRegex = (root, source, flags) => {
    const matcher = new RegExp(source, flags);
    return all(root).filter(element => labelTexts(element).some(value => matcher.test(value)));
  };
  const matchesAttribute = (actual, expected, exact) => exact
    ? actual === expected
    : actual.toLowerCase().includes(expected.toLowerCase());
  const attribute = (root, name, expected, exact) => all(root).filter(element => {
    const actual = element.getAttribute(name);
    return actual !== null && matchesAttribute(actual, expected, exact);
  });
  const attributeRegex = (root, name, source, flags) => {
    const matcher = new RegExp(source, flags);
    return all(root).filter(element => {
      const actual = element.getAttribute(name);
      return actual !== null && String(actual).match(matcher);
    });
  };
  const testId = (root, names, expected) => all(root).filter(element => names.some(name =>
    element.getAttribute(name) === expected));
  const testIdRegex = (root, names, source, flags) => {
    const matcher = new RegExp(source, flags);
    return all(root).filter(element => names.some(name => {
      const actual = element.getAttribute(name);
      return actual !== null && String(actual).match(matcher);
    }));
  };
  const validRoles = new Set(['alert','alertdialog','application','article','banner','blockquote','button','caption','cell','checkbox','code','columnheader','combobox','complementary','contentinfo','definition','deletion','dialog','directory','document','emphasis','feed','figure','form','generic','grid','gridcell','group','heading','img','insertion','link','list','listbox','listitem','log','main','marquee','math','meter','menu','menubar','menuitem','menuitemcheckbox','menuitemradio','navigation','none','note','option','paragraph','presentation','progressbar','radio','radiogroup','region','row','rowgroup','rowheader','scrollbar','search','searchbox','separator','slider','spinbutton','status','strong','subscript','superscript','switch','tab','table','tablist','tabpanel','term','textbox','time','timer','toolbar','tooltip','tree','treegrid','treeitem']);
  const implicitRole = element => {
    const tag = element.tagName;
    if (tag === 'A' || tag === 'AREA') return element.hasAttribute('href') ? 'link' : null;
    if (/^H[1-6]$/.test(tag)) return 'heading';
    if (tag === 'INPUT') {
      const type = (element.type || 'text').toLowerCase();
      if (['button', 'image', 'reset', 'submit'].includes(type)) return 'button';
      if (type === 'checkbox') return 'checkbox';
      if (type === 'radio') return 'radio';
      if (type === 'range') return 'slider';
      if (type === 'number') return 'spinbutton';
      if (type === 'search') return element.hasAttribute('list') ? 'combobox' : 'searchbox';
      if (type === 'hidden') return null;
      return element.hasAttribute('list') ? 'combobox' : 'textbox';
    }
    if (tag === 'SELECT') return element.multiple || element.size > 1 ? 'listbox' : 'combobox';
    const roles = { ARTICLE:'article', ASIDE:'complementary', BLOCKQUOTE:'blockquote', BUTTON:'button', CAPTION:'caption', CODE:'code', DIALOG:'dialog', DL:'list', EM:'emphasis', FIGURE:'figure', FOOTER:'contentinfo', HEADER:'banner', IMG:'img', LI:'listitem', MAIN:'main', MATH:'math', MENU:'list', METER:'meter', NAV:'navigation', OL:'list', OPTGROUP:'group', OPTION:'option', OUTPUT:'status', P:'paragraph', PROGRESS:'progressbar', SEARCH:'search', STRONG:'strong', SUB:'subscript', SUP:'superscript', SVG:'img', TABLE:'table', TBODY:'rowgroup', TEXTAREA:'textbox', TFOOT:'rowgroup', THEAD:'rowgroup', TIME:'time', TR:'row', UL:'list' };
    if (tag === 'TD') return 'cell';
    if (tag === 'TH') return element.getAttribute('scope')?.startsWith('row') ? 'rowheader' : 'columnheader';
    if (tag === 'FORM') return accessibleName(element) ? 'form' : null;
    if (tag === 'SECTION') return accessibleName(element) ? 'region' : null;
    return roles[tag] || null;
  };
  const ariaRole = element => {
    const explicit = (element.getAttribute('role') || '').toLowerCase().split(/\s+/)
      .find(role => validRoles.has(role));
    return explicit || implicitRole(element);
  };
  const accessibleName = (element, seen = new Set()) => {
    if (seen.has(element)) return '';
    seen.add(element);
    const labelled = (element.getAttribute('aria-labelledby') || '').split(/\s+/).filter(Boolean)
      .map(id => element.ownerDocument.getElementById(id)).filter(Boolean)
      .map(ref => accessibleName(ref, seen)).join(' ');
    if (normalize(labelled)) return normalize(labelled);
    if (element.hasAttribute('aria-label')) return normalize(element.getAttribute('aria-label'));
    const labels = labelTexts(element);
    if (labels.length) return normalize(labels.join(' '));
    if ((element.tagName === 'IMG' || element.tagName === 'AREA') && element.hasAttribute('alt'))
      return normalize(element.getAttribute('alt'));
    if (element.tagName === 'INPUT' && ['button', 'reset', 'submit'].includes(element.type))
      return normalize(element.value);
    if (element.hasAttribute('title')) return normalize(element.getAttribute('title'));
    return normalize(element.textContent);
  };
  const accessibleDescription = element => {
    const described = referencedText(element, 'aria-describedby');
    if (described) return described;
    if (element.hasAttribute('aria-description')) return normalize(element.getAttribute('aria-description'));
    return normalize(element.getAttribute('title'));
  };
  const composedParent = element => element.parentElement || element.getRootNode()?.host || null;
  const hidden = element => {
    for (let current = element; current; current = composedParent(current)) {
      if (current.getAttribute('aria-hidden') === 'true') return true;
      const style = getComputedStyle(current);
      if (style.display === 'none' || style.visibility === 'hidden') return true;
    }
    return false;
  };
  const triState = (element, attribute, nativeName) => {
    const value = element.getAttribute(attribute);
    if (value === 'mixed') return 'mixed';
    if (value === 'true') return true;
    if (value === 'false') return false;
    return nativeName in element ? Boolean(element[nativeName]) : false;
  };
  const booleanState = (element, attribute, nativeName) => {
    const value = element.getAttribute(attribute);
    if (value !== null) return value.toLowerCase() === 'true';
    return nativeName && nativeName in element ? Boolean(element[nativeName]) : false;
  };
  const ariaDisabledRoles = new Set(['application','button','composite','gridcell','group','input','link','menuitem','scrollbar','separator','tab','checkbox','columnheader','combobox','grid','listbox','menu','menubar','menuitemcheckbox','menuitemradio','option','radio','radiogroup','row','rowheader','searchbox','select','slider','spinbutton','switch','tablist','textbox','toolbar','tree','treegrid','treeitem']);
  const disabled = element => {
    const native = new Set(['BUTTON','INPUT','SELECT','TEXTAREA','OPTION','OPTGROUP']);
    if (native.has(element.tagName) && element.matches(':disabled')) return true;
    if (!ariaDisabledRoles.has(ariaRole(element))) return false;
    for (let current = element; current; current = composedParent(current)) {
      const value = (current.getAttribute('aria-disabled') || '').toLowerCase();
      if (value === 'true') return true;
      if (value === 'false') return false;
    }
    return false;
  };
  const retarget = node => {
    let element = node?.nodeType === Node.ELEMENT_NODE ? node : node?.parentElement;
    if (!element) return null;
    if (!element.matches('input, textarea, select') && !element.isContentEditable)
      element = element.closest('button, [role=button], [role=checkbox], [role=radio]') || element;
    if (!element.matches('a, input, textarea, button, select, [role=link], [role=button], [role=checkbox], [role=radio]') && !element.isContentEditable) {
      const label = element.closest('label');
      if (label?.control) element = label.control;
    }
    return element;
  };
  const readonlyRoles = new Set(['checkbox','combobox','grid','gridcell','listbox','radiogroup','slider','spinbutton','textbox','columnheader','rowheader','searchbox','switch','treegrid']);
  const readonly = element => {
    if (['INPUT','TEXTAREA','SELECT'].includes(element.tagName)) return element.hasAttribute('readonly');
    if (readonlyRoles.has(ariaRole(element))) return element.getAttribute('aria-readonly') === 'true';
    if (element.isContentEditable) return false;
    return null;
  };
  const checkedRoles = new Set(['checkbox','menuitemcheckbox','option','radio','switch','menuitemradio','treeitem']);
  const checked = element => {
    if (element.tagName === 'INPUT' && ['checkbox','radio'].includes(element.type)) return element.checked;
    if (checkedRoles.has(ariaRole(element))) return element.getAttribute('aria-checked') === 'true';
    return null;
  };
  const elementState = (node, state) => {
    const element = retarget(node);
    if (!element || !element.isConnected) return { status: 'retry' };
    if (state === 'enabled') return { status: 'done', value: !disabled(element) };
    if (state === 'editable') {
      const value = readonly(element);
      if (value === null) return { status: 'invalid-target', message: 'Element is not an <input>, <textarea>, <select> or [contenteditable] and does not have a role allowing [aria-readonly]' };
      return { status: 'done', value: !disabled(element) && !value };
    }
    if (state === 'checked') {
      const value = checked(element);
      if (value === null) return { status: 'invalid-target', message: 'Not a checkbox or radio button' };
      return { status: 'done', value };
    }
    return { status: 'invalid-target', message: `Unexpected element state "${state}"` };
  };
  const level = element => {
    const aria = Number(element.getAttribute('aria-level'));
    if (Number.isInteger(aria) && aria > 0) return aria;
    const match = /^H([1-6])$/.exec(element.tagName);
    return match ? Number(match[1]) : 0;
  };
  const compilePattern = pattern => {
    if (!pattern) return null;
    if (pattern.kind === 'regex')
      return { kind: 'regex', matcher: new RegExp(pattern.source, pattern.flags) };
    return pattern;
  };
  const matchesPattern = (actual, pattern) => pattern.kind === 'regex'
    ? pattern.matcher.test(String(actual ?? ''))
    : matchesText(actual, pattern.value, pattern.exact);
  const role = (root, expectedRole, options) => {
    const name = compilePattern(options.name);
    const description = compilePattern(options.description);
    return all(root).filter(element => {
      if (ariaRole(element) !== expectedRole) return false;
      if (!options.includeHidden && hidden(element)) return false;
      if (name && !matchesPattern(accessibleName(element), name)) return false;
      if (description && !matchesPattern(accessibleDescription(element), description)) return false;
      if (options.checked !== null && triState(element, 'aria-checked', 'checked') !== options.checked) return false;
      if (options.pressed !== null && triState(element, 'aria-pressed', 'pressed') !== options.pressed) return false;
      if (options.selected !== null && booleanState(element, 'aria-selected', 'selected') !== options.selected) return false;
      if (options.expanded !== null && booleanState(element, 'aria-expanded', null) !== options.expanded) return false;
      if (options.disabled !== null && disabled(element) !== options.disabled) return false;
      if (options.level !== null && level(element) !== options.level) return false;
      return true;
    });
  };
  const domOrder = values => {
    const order = new Map(all(document).map((element, index) => [element, index]));
    return values.sort((left, right) => (order.get(left) ?? Number.MAX_SAFE_INTEGER) - (order.get(right) ?? Number.MAX_SAFE_INTEGER));
  };
  const union = (left, right) => domOrder(Array.from(new Set([...left, ...right])));
  const within = (bases, query) => {
    const result = [];
    const seen = new Set();
    for (const base of bases) {
      for (const element of query(base)) {
        if (!seen.has(element)) {
          seen.add(element);
          result.push(element);
        }
      }
    }
    return domOrder(result);
  };
  const visible = element => {
    const style = getComputedStyle(element);
    if (!style) return true;
    if (style.display === 'contents') {
      for (let child = element.firstChild; child; child = child.nextSibling) {
        if (child.nodeType === Node.ELEMENT_NODE && visible(child)) return true;
        if (child.nodeType === Node.TEXT_NODE) {
          const range = element.ownerDocument.createRange();
          range.selectNode(child);
          const rect = range.getBoundingClientRect();
          if (rect.width > 0 && rect.height > 0) return true;
        }
      }
      return false;
    }
    if (Element.prototype.checkVisibility && !element.checkVisibility()) return false;
    if (style.visibility !== 'visible') return false;
    const rect = element.getBoundingClientRect();
    return rect.width > 0 && rect.height > 0;
  };
  return Object.freeze({ version: 6, css, text, textRegex, label, labelRegex, attribute, attributeRegex, testId, testIdRegex, role, union, within, visible, elementState, retarget, matchesText });
})()"#;

#[derive(Default)]
struct SelectorStats {
    nodes: usize,
    string_bytes: usize,
    string_capacity: usize,
}

fn measure_selector_storage_inner(
    selector: &Selector,
    depth: usize,
    stats: &mut SelectorStats,
) -> Result<(), OperationError<LocatorErrorDetails>> {
    if depth >= MAX_SELECTOR_DEPTH {
        return Err(selector_resource_error("selector depth exceeds 32"));
    }
    stats.nodes = stats.nodes.saturating_add(1);
    if stats.nodes > MAX_SELECTOR_NODES {
        return Err(selector_resource_error("selector node count exceeds 256"));
    }
    match selector {
        Selector::Css(css) => {
            claim_selector_storage(stats, css.as_str(), css.retained_string_capacity())
        }
        Selector::Text(matcher) | Selector::Label(matcher) => {
            claim_selector_storage(stats, matcher.value(), matcher.retained_string_capacity())
        }
        Selector::TextRegex(matcher) | Selector::LabelRegex(matcher) => {
            claim_regex_storage(stats, matcher)
        }
        Selector::AttributeText { matcher, .. } => {
            claim_selector_storage(stats, matcher.value(), matcher.retained_string_capacity())
        }
        Selector::AttributeRegex { matcher, .. } => claim_regex_storage(stats, matcher),
        Selector::TestIdText { attributes, value } => {
            claim_selector_storage(
                stats,
                attributes.as_csv(),
                attributes.retained_string_capacity(),
            )?;
            claim_selector_storage(stats, value, value.capacity())
        }
        Selector::TestIdRegex {
            attributes,
            matcher,
        } => {
            claim_selector_storage(
                stats,
                attributes.as_csv(),
                attributes.retained_string_capacity(),
            )?;
            claim_regex_storage(stats, matcher)
        }
        Selector::Role(role) => claim_role_options_storage(stats, role.options()),
        Selector::RolePattern(role) => claim_role_options_storage(stats, role.options()),
        Selector::Nth { base, .. } => measure_selector_storage_inner(base, depth + 1, stats),
        Selector::FilterText { base, text, .. } | Selector::FilterNotText { base, text, .. } => {
            measure_selector_storage_inner(base, depth + 1, stats)?;
            claim_selector_storage(stats, text, text.capacity())
        }
        Selector::FilterTextRegex { base, matcher }
        | Selector::FilterNotTextRegex { base, matcher } => {
            measure_selector_storage_inner(base, depth + 1, stats)?;
            claim_regex_storage(stats, matcher)
        }
        Selector::And { left, right } | Selector::Or { left, right } => {
            measure_selector_storage_inner(left, depth + 1, stats)?;
            measure_selector_storage_inner(right, depth + 1, stats)
        }
        Selector::Within { base, descendant }
        | Selector::FilterHas { base, descendant }
        | Selector::FilterHasNot { base, descendant } => {
            measure_selector_storage_inner(base, depth + 1, stats)?;
            measure_selector_storage_inner(descendant, depth + 1, stats)
        }
        Selector::FilterVisible { base, .. } => {
            measure_selector_storage_inner(base, depth + 1, stats)
        }
    }
}

fn claim_selector_storage(
    stats: &mut SelectorStats,
    value: &str,
    capacity: usize,
) -> Result<(), OperationError<LocatorErrorDetails>> {
    stats.string_capacity = stats.string_capacity.saturating_add(capacity);
    claim_selector_string(stats, value)
}

fn claim_regex_storage(
    stats: &mut SelectorStats,
    matcher: &RegexMatcher,
) -> Result<(), OperationError<LocatorErrorDetails>> {
    stats.string_capacity = stats
        .string_capacity
        .saturating_add(matcher.retained_string_capacity());
    claim_selector_string(stats, matcher.source())?;
    claim_selector_string(stats, matcher.flags())
}

fn claim_role_options_storage(
    stats: &mut SelectorStats,
    options: &RoleOptions,
) -> Result<(), OperationError<LocatorErrorDetails>> {
    if let Some(name) = &options.name {
        claim_selector_storage(stats, name.value(), name.retained_string_capacity())?;
    }
    if let Some(description) = &options.description {
        claim_selector_storage(
            stats,
            description.value(),
            description.retained_string_capacity(),
        )?;
    }
    if let Some(name) = &options.name_regex {
        claim_regex_storage(stats, name)?;
    }
    if let Some(description) = &options.description_regex {
        claim_regex_storage(stats, description)?;
    }
    Ok(())
}

fn compile_selector_inner(
    selector: &Selector,
    root: &str,
    depth: usize,
    stats: &mut SelectorStats,
) -> Result<String, OperationError<LocatorErrorDetails>> {
    if depth >= MAX_SELECTOR_DEPTH {
        return Err(selector_resource_error("selector depth exceeds 32"));
    }
    stats.nodes = stats.nodes.saturating_add(1);
    if stats.nodes > MAX_SELECTOR_NODES {
        return Err(selector_resource_error("selector node count exceeds 256"));
    }
    match selector {
        Selector::Css(css) => {
            claim_selector_string(stats, css.as_str())?;
            let css = serde_json::to_string(css.as_str()).map_err(|error| {
                invalid_response(&format!("could not encode CSS selector: {error}"))
            })?;
            Ok(format!("__pw.css({root}, {css})"))
        }
        Selector::Text(matcher) => {
            claim_selector_string(stats, matcher.value())?;
            Ok(format!(
                "__pw.text({root}, {}, {})",
                json_string(matcher, "text selector")?,
                matcher.exact()
            ))
        }
        Selector::TextRegex(matcher) => {
            claim_selector_string(stats, matcher.source())?;
            claim_selector_string(stats, matcher.flags())?;
            let (source, flags) = regex_json(matcher, "text regex")?;
            Ok(format!("__pw.textRegex({root}, {source}, {flags})"))
        }
        Selector::Label(matcher) => {
            claim_selector_string(stats, matcher.value())?;
            Ok(format!(
                "__pw.label({root}, {}, {})",
                json_string(matcher, "label selector")?,
                matcher.exact()
            ))
        }
        Selector::LabelRegex(matcher) => {
            claim_selector_string(stats, matcher.source())?;
            claim_selector_string(stats, matcher.flags())?;
            let (source, flags) = regex_json(matcher, "label regex")?;
            Ok(format!("__pw.labelRegex({root}, {source}, {flags})"))
        }
        Selector::AttributeText { attribute, matcher } => {
            claim_selector_string(stats, matcher.value())?;
            let value = json_string(matcher, "attribute selector")?;
            Ok(format!(
                "__pw.attribute({root}, {:?}, {value}, {})",
                attribute.as_str(),
                matcher.exact()
            ))
        }
        Selector::AttributeRegex { attribute, matcher } => {
            claim_selector_string(stats, matcher.source())?;
            claim_selector_string(stats, matcher.flags())?;
            let (source, flags) = regex_json(matcher, "attribute regex")?;
            Ok(format!(
                "__pw.attributeRegex({root}, {:?}, {source}, {flags})",
                attribute.as_str()
            ))
        }
        Selector::TestIdText { attributes, value } => {
            claim_selector_string(stats, attributes.as_csv())?;
            claim_selector_string(stats, value)?;
            let names = test_id_attributes_json(attributes)?;
            let value = serde_json::to_string(value).map_err(|error| {
                invalid_response(&format!("could not encode test-id selector: {error}"))
            })?;
            Ok(format!("__pw.testId({root}, {names}, {value})"))
        }
        Selector::TestIdRegex {
            attributes,
            matcher,
        } => {
            claim_selector_string(stats, attributes.as_csv())?;
            claim_selector_string(stats, matcher.source())?;
            claim_selector_string(stats, matcher.flags())?;
            let names = test_id_attributes_json(attributes)?;
            let (source, flags) = regex_json(matcher, "test-id regex")?;
            Ok(format!(
                "__pw.testIdRegex({root}, {names}, {source}, {flags})"
            ))
        }
        Selector::Role(role) => {
            claim_role_options_strings(stats, role.options())?;
            let options = role_options_json(role.options())?;
            Ok(format!(
                "__pw.role({root}, {:?}, {options})",
                role.role().as_str()
            ))
        }
        Selector::RolePattern(role) => {
            claim_role_options_strings(stats, role.options())?;
            let options = role_options_json(role.options())?;
            Ok(format!(
                "__pw.role({root}, {:?}, {options})",
                role.role().as_str()
            ))
        }
        Selector::Nth { base, index } => {
            let base = compile_selector_inner(base, root, depth + 1, stats)?;
            Ok(format!(
                "(() => {{ const values = {base}; let index = {index}; if (index < 0) index += values.length; return index >= 0 && index < values.length ? [values[index]] : []; }})()"
            ))
        }
        Selector::FilterText { base, text, exact } => {
            let base = compile_selector_inner(base, root, depth + 1, stats)?;
            claim_selector_string(stats, text)?;
            let text = serde_json::to_string(text).map_err(|error| {
                invalid_response(&format!("could not encode text filter: {error}"))
            })?;
            Ok(format!(
                "({base}).filter(element => __pw.matchesText(element.textContent || '', {text}, {exact}))"
            ))
        }
        Selector::FilterNotText { base, text, exact } => {
            let base = compile_selector_inner(base, root, depth + 1, stats)?;
            claim_selector_string(stats, text)?;
            let text = serde_json::to_string(text).map_err(|error| {
                invalid_response(&format!("could not encode negative text filter: {error}"))
            })?;
            Ok(format!(
                "({base}).filter(element => !__pw.matchesText(element.textContent || '', {text}, {exact}))"
            ))
        }
        Selector::FilterTextRegex { base, matcher } => {
            let base = compile_selector_inner(base, root, depth + 1, stats)?;
            claim_selector_string(stats, matcher.source())?;
            claim_selector_string(stats, matcher.flags())?;
            let (source, flags) = regex_json(matcher, "text filter regex")?;
            Ok(format!(
                "(() => {{ const matcher = new RegExp({source}, {flags}); return ({base}).filter(element => matcher.test(String(element.textContent ?? ''))); }})()"
            ))
        }
        Selector::FilterNotTextRegex { base, matcher } => {
            let base = compile_selector_inner(base, root, depth + 1, stats)?;
            claim_selector_string(stats, matcher.source())?;
            claim_selector_string(stats, matcher.flags())?;
            let (source, flags) = regex_json(matcher, "negative text filter regex")?;
            Ok(format!(
                "(() => {{ const matcher = new RegExp({source}, {flags}); return ({base}).filter(element => !matcher.test(String(element.textContent ?? ''))); }})()"
            ))
        }
        Selector::FilterHas { base, descendant } => {
            let base = compile_selector_inner(base, root, depth + 1, stats)?;
            let descendant = compile_selector_inner(descendant, "__candidate", depth + 1, stats)?;
            Ok(format!(
                "({base}).filter(__candidate => ({descendant}).length > 0)"
            ))
        }
        Selector::FilterHasNot { base, descendant } => {
            let base = compile_selector_inner(base, root, depth + 1, stats)?;
            let descendant = compile_selector_inner(descendant, "__candidate", depth + 1, stats)?;
            Ok(format!(
                "({base}).filter(__candidate => ({descendant}).length === 0)"
            ))
        }
        Selector::FilterVisible { base, visible } => {
            let base = compile_selector_inner(base, root, depth + 1, stats)?;
            Ok(format!(
                "({base}).filter(element => __pw.visible(element) === {visible})"
            ))
        }
        Selector::And { left, right } => {
            let left = compile_selector_inner(left, root, depth + 1, stats)?;
            let right = compile_selector_inner(right, root, depth + 1, stats)?;
            Ok(format!(
                "(() => {{ const right = new Set({right}); return ({left}).filter(element => right.has(element)); }})()"
            ))
        }
        Selector::Or { left, right } => {
            let left = compile_selector_inner(left, root, depth + 1, stats)?;
            let right = compile_selector_inner(right, root, depth + 1, stats)?;
            Ok(format!("__pw.union({left}, {right})"))
        }
        Selector::Within { base, descendant } => {
            let base = compile_selector_inner(base, root, depth + 1, stats)?;
            let descendant = compile_selector_inner(descendant, "__root", depth + 1, stats)?;
            Ok(format!("__pw.within({base}, __root => {descendant})"))
        }
    }
}

fn claim_selector_string(
    stats: &mut SelectorStats,
    value: &str,
) -> Result<(), OperationError<LocatorErrorDetails>> {
    stats.string_bytes = stats.string_bytes.saturating_add(value.len());
    if stats.string_bytes > MAX_SELECTOR_STRING_BYTES {
        Err(selector_resource_error(
            "selector strings exceed 65536 bytes",
        ))
    } else {
        Ok(())
    }
}

fn json_string(
    matcher: &TextMatcher,
    context: &str,
) -> Result<String, OperationError<LocatorErrorDetails>> {
    serde_json::to_string(matcher.value())
        .map_err(|error| invalid_response(&format!("could not encode {context}: {error}")))
}

fn regex_json(
    matcher: &RegexMatcher,
    context: &str,
) -> Result<(String, String), OperationError<LocatorErrorDetails>> {
    let source = serde_json::to_string(matcher.source()).map_err(|error| {
        invalid_response(&format!("could not encode {context} source: {error}"))
    })?;
    let flags = serde_json::to_string(matcher.flags())
        .map_err(|error| invalid_response(&format!("could not encode {context} flags: {error}")))?;
    Ok((source, flags))
}

fn test_id_attributes_json(
    attributes: &TestIdAttributes,
) -> Result<String, OperationError<LocatorErrorDetails>> {
    let names = attributes.names().collect::<Vec<_>>();
    serde_json::to_string(&names)
        .map_err(|error| invalid_response(&format!("could not encode test-id attributes: {error}")))
}

fn claim_role_options_strings(
    stats: &mut SelectorStats,
    options: &RoleOptions,
) -> Result<(), OperationError<LocatorErrorDetails>> {
    if let Some(name) = &options.name {
        claim_selector_string(stats, name.value())?;
    }
    if let Some(description) = &options.description {
        claim_selector_string(stats, description.value())?;
    }
    if let Some(name) = &options.name_regex {
        claim_selector_string(stats, name.source())?;
        claim_selector_string(stats, name.flags())?;
    }
    if let Some(description) = &options.description_regex {
        claim_selector_string(stats, description.source())?;
        claim_selector_string(stats, description.flags())?;
    }
    Ok(())
}

fn role_options_json(options: &RoleOptions) -> Result<String, OperationError<LocatorErrorDetails>> {
    let string_matcher = |value: &TextMatcher| serde_json::json!({"kind": "string", "value": value.value(), "exact": value.exact()});
    let regex_matcher = |value: &RegexMatcher| serde_json::json!({"kind": "regex", "source": value.source(), "flags": value.flags()});
    let pattern = |string: Option<&TextMatcher>, regex: Option<&RegexMatcher>| {
        string
            .map(string_matcher)
            .or_else(|| regex.map(regex_matcher))
    };
    let tri_state = |value: AriaTriState| match value {
        AriaTriState::False => serde_json::json!(false),
        AriaTriState::True => serde_json::json!(true),
        AriaTriState::Mixed => serde_json::json!("mixed"),
    };
    serde_json::to_string(&serde_json::json!({
        "name": pattern(options.name.as_ref(), options.name_regex.as_ref()),
        "description": pattern(options.description.as_ref(), options.description_regex.as_ref()),
        "checked": options.checked.map(tri_state),
        "disabled": options.disabled,
        "expanded": options.expanded,
        "includeHidden": options.include_hidden,
        "level": options.level.map(std::num::NonZeroU32::get),
        "pressed": options.pressed.map(tri_state),
        "selected": options.selected,
    }))
    .map_err(|error| invalid_response(&format!("could not encode role options: {error}")))
}

pub const MAX_SELECTOR_RUNTIME_INSTALL_BYTES: usize =
    SELECTOR_RUNTIME.len() + MAX_INSTALL_WRAPPER_BYTES;

fn selector_resource_error(message: &str) -> OperationError<LocatorErrorDetails> {
    OperationError::new(
        FailureKind::ResourceExhausted,
        message,
        Some(LocatorErrorDetails::Backend(message.to_owned())),
    )
}

fn invalid_response(message: &str) -> OperationError<LocatorErrorDetails> {
    OperationError::new(
        FailureKind::ProtocolViolation,
        message,
        Some(LocatorErrorDetails::InvalidBackendResponse(
            message.to_owned(),
        )),
    )
}

#[cfg(test)]
mod tests {
    use pwright_core::FailureKind;
    use pwright_selector_model::{AriaRole, RegexMatcher, RoleOptions, Selector};

    use super::{
        MAX_COMPILED_SELECTOR_BYTES, MAX_SELECTOR_RUNTIME_INSTALL_BYTES, SELECTOR_RUNTIME,
        SELECTOR_RUNTIME_SYMBOL, SELECTOR_RUNTIME_VERSION, compile_selector, measure_selector,
        selector_runtime_install_script,
    };

    #[test]
    fn install_script_uses_the_exported_runtime_identity_within_its_bound() {
        // Arrange
        let expected_version = format!("version: {SELECTOR_RUNTIME_VERSION}");

        // Act
        let script = selector_runtime_install_script();

        // Assert
        assert!(script.contains(SELECTOR_RUNTIME_SYMBOL));
        assert!(SELECTOR_RUNTIME.contains(&expected_version));
        assert!(script.len() <= MAX_SELECTOR_RUNTIME_INSTALL_BYTES);
    }

    #[test]
    fn semantic_and_relative_selectors_share_one_compiler() {
        // Arrange
        let selector = Selector::role(
            AriaRole::Dialog,
            RoleOptions::new().with_name("Settings", true),
        )
        .expect("role should validate")
        .within(
            Selector::css(".card")
                .expect("CSS should validate")
                .filter_has(Selector::text_regex(
                    RegexMatcher::new("Ready\\s+now", "i").expect("regex should validate"),
                ))
                .filter_visible(true),
        );

        // Act
        let script =
            compile_selector(&selector, "document").expect("bounded selector should compile");

        // Assert
        assert!(script.contains("__pw.role(document, \"dialog\""));
        assert!(script.contains("__pw.css(__root, \".card\")"));
        assert!(script.contains("__pw.textRegex(__candidate"));
        assert!(script.contains("__pw.visible(element) === true"));
        assert!(script.len() <= MAX_COMPILED_SELECTOR_BYTES);
    }

    #[test]
    fn selector_measurement_reports_retained_capacity_not_only_length() {
        // Arrange
        let mut value = String::with_capacity(128);
        value.push_str("button");
        let selector = Selector::css(value).expect("CSS should validate");

        // Act
        let storage = measure_selector(&selector).expect("selector should fit");

        // Assert
        assert_eq!(storage.string_capacity(), 128);
    }

    #[test]
    fn compiler_rejects_depth_and_node_growth_before_script_escape() {
        // Arrange
        let mut too_deep = Selector::css("button").expect("CSS should validate");
        for _ in 0..33 {
            too_deep = too_deep.nth(0);
        }
        let mut too_many = Selector::css("button").expect("CSS should validate");
        for _ in 0..256 {
            too_many = too_many.or(Selector::css("button").expect("CSS should validate"));
        }

        // Act
        let depth_error = compile_selector(&too_deep, "document")
            .expect_err("excessive selector depth should fail");
        let node_error =
            measure_selector(&too_many).expect_err("excessive selector nodes should fail");

        // Assert
        assert_eq!(depth_error.kind(), FailureKind::ResourceExhausted);
        assert_eq!(node_error.kind(), FailureKind::ResourceExhausted);
    }
}
