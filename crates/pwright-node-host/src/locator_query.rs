//! Immutable, browser-free Locator values with caller-owned storage admission.

pub mod semantic;

use std::sync::{Arc, Mutex};

#[cfg(any(feature = "handles", test))]
use pwright_backend_cdp_selector_runtime::{MAX_COMPILED_SELECTOR_BYTES, compile_selector};
use pwright_backend_cdp_selector_runtime::{
    MAX_SELECTOR_DEPTH, MAX_SELECTOR_NODES, MAX_SELECTOR_STRING_BYTES, measure_selector,
};
use pwright_cap_locator::{RegexMatcher, Selector};

use crate::HostError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QueryLimits {
    pub recipes: usize,
    pub bytes: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct QueryMetrics {
    pub recipes: usize,
    pub bytes: usize,
}

struct BudgetState {
    limits: QueryLimits,
    usage: QueryMetrics,
}

#[derive(Clone)]
pub struct QueryBudget(Arc<Mutex<BudgetState>>);

impl QueryBudget {
    pub fn new(limits: QueryLimits) -> Result<Self, HostError> {
        if limits.recipes == 0 || limits.bytes == 0 {
            return Err(HostError::InvalidArgument(
                "Locator query limits must be nonzero".into(),
            ));
        }
        Ok(Self(Arc::new(Mutex::new(BudgetState {
            limits,
            usage: QueryMetrics::default(),
        }))))
    }

    pub fn metrics(&self) -> Result<QueryMetrics, HostError> {
        Ok(self.0.lock().map_err(HostError::backend)?.usage)
    }

    /// Call before converting borrowed input to an owned Rust string. The
    /// caller declares aggregate string capacity, including regex flags/names.
    pub fn prepare_leaf(&self, string_bytes: usize) -> Result<QueryDraft, HostError> {
        self.prepare(Shape {
            depth: 1,
            nodes: 1,
            strings: string_bytes,
        })
    }

    fn prepare(&self, shape: Shape) -> Result<QueryDraft, HostError> {
        shape.validate()?;
        let bytes = shape.storage_bytes()?;
        let mut state = self.0.lock().map_err(HostError::backend)?;
        if state.usage.recipes >= state.limits.recipes
            || bytes > state.limits.bytes.saturating_sub(state.usage.bytes)
        {
            return Err(exhausted("Locator recipe count or byte capacity exceeded"));
        }
        state.usage.recipes += 1;
        state.usage.bytes += bytes;
        Ok(QueryDraft {
            shape,
            lease: QueryLease {
                budget: self.clone(),
                bytes,
            },
        })
    }
}

fn exhausted(message: &str) -> HostError {
    HostError::ResourceExhausted(message.to_owned())
}

#[derive(Clone, Copy)]
struct Shape {
    depth: usize,
    nodes: usize,
    strings: usize,
}

impl Shape {
    fn validate(self) -> Result<(), HostError> {
        if self.depth > MAX_SELECTOR_DEPTH
            || self.nodes > MAX_SELECTOR_NODES
            || self.strings > MAX_SELECTOR_STRING_BYTES
        {
            return Err(exhausted(
                "Locator recipe exceeds selector depth, node or string limit",
            ));
        }
        Ok(())
    }

    fn storage_bytes(self) -> Result<usize, HostError> {
        self.nodes
            .checked_mul(std::mem::size_of::<Selector>())
            // Includes one temporary conversion/copy overlap, not only the
            // final string. N-API's UTF-8 adapter owns a terminated buffer
            // while making the final String; fixed metadata covers its NUL.
            .and_then(|bytes| bytes.checked_add(self.strings.checked_mul(2)?))
            .and_then(|bytes| {
                bytes.checked_add(
                    std::mem::size_of::<QueryInner>() + 2 * std::mem::size_of::<usize>(),
                )
            })
            .ok_or_else(|| exhausted("Locator recipe storage calculation overflowed"))
    }

    fn unary(self, strings: usize) -> Result<Self, HostError> {
        self.binary(Self {
            depth: 0,
            nodes: 0,
            strings,
        })
    }

    fn binary(self, other: Self) -> Result<Self, HostError> {
        let shape = Self {
            depth: self.depth.max(other.depth).saturating_add(1),
            nodes: self.nodes.saturating_add(other.nodes).saturating_add(1),
            strings: self.strings.saturating_add(other.strings),
        };
        shape.validate()?;
        Ok(shape)
    }
}

pub(crate) struct QueryLease {
    budget: QueryBudget,
    bytes: usize,
}

impl Drop for QueryLease {
    fn drop(&mut self) {
        let mut state = self
            .budget
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.usage.recipes -= 1;
        state.usage.bytes -= self.bytes;
    }
}

/// Validation discovered while constructing a lazy value, surfaced on use.
/// These fixed variants retain no unbounded error message or selector copy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QueryIssue {
    EmptyCss,
    CapturedSelector,
    MultipleCaptures,
    NthAfterCapture,
    NthMatchMissingSelectors,
    NthMatchInvalidIndex,
    UnsupportedCssExtension,
    InvalidRole,
    InvalidRoleOptions,
    UnknownEngine {
        selector: String,
        engine_start: usize,
        engine_end: usize,
    },
    #[cfg(feature = "layout-selectors")]
    Layout(crate::layout_selectors::LayoutSelectorIssue),
}

impl QueryIssue {
    fn error(&self) -> HostError {
        match self {
            Self::EmptyCss => HostError::Backend("Locator CSS selector must not be empty".into()),
            Self::CapturedSelector => HostError::UnsupportedOperation(
                "captured selector execution is not installed".into(),
            ),
            Self::MultipleCaptures => {
                HostError::Backend("Only one of the selectors can capture using * modifier".into())
            }
            Self::NthAfterCapture => {
                HostError::Backend("Can't query n-th element in a request with the capture.".into())
            }
            Self::NthMatchMissingSelectors => HostError::Backend(
                "\"nth-match\" engine expects non-empty selector list and an index argument".into(),
            ),
            Self::NthMatchInvalidIndex => HostError::Backend(
                "\"nth-match\" engine expects a one-based index as the last argument".into(),
            ),
            Self::UnsupportedCssExtension => HostError::UnsupportedOperation(
                "unsupported Playwright CSS extension combination".into(),
            ),
            Self::InvalidRole => HostError::Backend("unknown ARIA role".into()),
            Self::InvalidRoleOptions => {
                HostError::Backend("ARIA role does not support the requested filters".into())
            }
            Self::UnknownEngine {
                selector,
                engine_start,
                engine_end,
            } => HostError::Backend(format!(
                "Unknown engine \"{}\" while parsing selector {selector}",
                &selector[*engine_start..*engine_end]
            )),
            #[cfg(feature = "layout-selectors")]
            Self::Layout(issue) => HostError::Backend(issue.message()),
        }
    }
}

pub(crate) const PLAYWRIGHT_BUILTIN_ENGINES: &[&str] = &[
    "css",
    "css:light",
    "xpath",
    "xpath:light",
    "_react",
    "_vue",
    "text",
    "text:light",
    "id",
    "id:light",
    "data-testid",
    "data-testid:light",
    "data-test-id",
    "data-test-id:light",
    "data-test",
    "data-test:light",
    "nth",
    "visible",
    "internal:control",
    "internal:has",
    "internal:has-not",
    "internal:has-text",
    "internal:has-not-text",
    "internal:and",
    "internal:or",
    "internal:chain",
    "role",
    "internal:attr",
    "internal:label",
    "internal:text",
    "internal:role",
    "internal:testid",
    "internal:describe",
    "aria-ref",
];

#[derive(Clone, Copy)]
struct SelectorPartSpan<'a> {
    value: &'a str,
    start: usize,
}

struct PlaywrightSelectorParts<'a> {
    selector: &'a str,
    start: usize,
    index: usize,
    finished: bool,
}

fn playwright_selector_parts(selector: &str) -> PlaywrightSelectorParts<'_> {
    PlaywrightSelectorParts {
        selector,
        start: 0,
        index: 0,
        finished: false,
    }
}

fn text_quote_belongs_to_body(selector: &str, start: usize, quote: usize) -> bool {
    let prefix = selector[start..quote].trim_start();
    // The upstream tokenizer applies this exception to literal `text=` only.
    // A captured `*text=` starts quote tracking even when text precedes it.
    let Some(after_engine) = prefix.strip_prefix("text") else {
        return false;
    };
    let Some(body) = after_engine.trim_start().strip_prefix('=') else {
        return false;
    };
    !body.is_empty()
}

impl<'a> Iterator for PlaywrightSelectorParts<'a> {
    type Item = SelectorPartSpan<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        let bytes = self.selector.as_bytes();
        let mut quote = None;
        while self.index < bytes.len() {
            let byte = bytes[self.index];
            if byte == b'\\' && self.index + 1 < bytes.len() {
                self.index += 2;
            } else if quote == Some(byte) {
                quote = None;
                self.index += 1;
            } else if quote.is_none()
                && matches!(byte, b'\'' | b'"' | b'`')
                && !text_quote_belongs_to_body(self.selector, self.start, self.index)
            {
                quote = Some(byte);
                self.index += 1;
            } else if quote.is_none() && byte == b'>' && bytes.get(self.index + 1) == Some(&b'>') {
                let part = SelectorPartSpan {
                    value: &self.selector[self.start..self.index],
                    start: self.start,
                };
                self.index += 2;
                self.start = self.index;
                return Some(part);
            } else {
                self.index += 1;
            }
        }
        self.finished = true;
        Some(SelectorPartSpan {
            value: &self.selector[self.start..],
            start: self.start,
        })
    }
}

fn capture_part_index(selector: &str) -> Result<Option<usize>, QueryIssue> {
    let mut captured = None;
    for (index, part) in playwright_selector_parts(selector).enumerate() {
        let trimmed = part.value.trim();
        let Some((engine, _body)) = trimmed.split_once('=') else {
            continue;
        };
        if engine.trim().starts_with('*')
            && engine.trim().bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'+' | b':' | b'*')
            })
            && captured.replace(index).is_some()
        {
            return Err(QueryIssue::MultipleCaptures);
        }
    }
    Ok(captured)
}

fn capture_selector(source: &str, capture_index: usize) -> Result<CapturedSelector, QueryIssue> {
    let mut prefix: Option<Selector> = None;
    let mut suffix: Option<Selector> = None;
    for (index, part) in playwright_selector_parts(source).enumerate() {
        let value = part.value.trim();
        if value.starts_with("nth=") || value.starts_with("*nth=") {
            return Err(QueryIssue::NthAfterCapture);
        }
        let value = if index == capture_index {
            value.strip_prefix('*').expect("capture index has a marker")
        } else {
            value
        };
        if selector_part_requires_browser_program(value) {
            return Err(QueryIssue::CapturedSelector);
        }
        if css_has_supported_pseudo(value) {
            return Err(QueryIssue::CapturedSelector);
        }
        let parsed = playwright_selector_part(value).map_err(|_| QueryIssue::EmptyCss)?;
        // A leading visible engine in the post-capture suffix examines the
        // candidate itself. Keep that predicate on the captured prefix; later
        // descendants still belong to the suffix, not to the returned element.
        if index > capture_index
            && suffix.is_none()
            && let SelectorPart::Visible(visible) = &parsed
        {
            prefix = Some(
                prefix
                    .take()
                    .expect("capture prefix was parsed")
                    .filter_visible(*visible),
            );
            continue;
        }
        let current = if index <= capture_index {
            &mut prefix
        } else {
            &mut suffix
        };
        let previous = current.take();
        *current = Some(match (previous, parsed) {
            (None, SelectorPart::Within(selector)) => selector,
            (Some(base), SelectorPart::Within(selector)) => base.within(selector),
            (Some(base), SelectorPart::And(selector)) => base.and(selector),
            (Some(base), SelectorPart::Visible(visible)) => base.filter_visible(visible),
            (None, _) => return Err(QueryIssue::CapturedSelector),
            (_, SelectorPart::Nth(_)) => return Err(QueryIssue::NthAfterCapture),
        });
    }
    Ok(CapturedSelector {
        prefix: prefix.expect("capture has a parsed prefix"),
        suffix,
    })
}

fn unknown_engine(selector: &str) -> Option<(usize, usize)> {
    for part in playwright_selector_parts(selector) {
        let leading = part
            .value
            .len()
            .saturating_sub(part.value.trim_start().len());
        let trimmed = part.value.trim();
        let Some(equals) = trimmed.find('=') else {
            continue;
        };
        let engine = trimmed[..equals].trim();
        if !engine.is_empty()
            && engine.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'+' | b':' | b'*')
            })
        {
            let engine = engine.strip_prefix('*').unwrap_or(engine);
            if !PLAYWRIGHT_BUILTIN_ENGINES.contains(&engine) {
                let start = part.start + leading + usize::from(trimmed.starts_with('*'));
                return Some((start, start + engine.len()));
            }
        }
    }
    None
}

fn unquote_text_selector(value: &str) -> Option<String> {
    let quote = value.as_bytes().first().copied()?;
    if value.len() <= 1
        || !matches!(quote, b'\'' | b'"')
        || value.as_bytes().last().copied() != Some(quote)
    {
        return None;
    }
    let mut unquoted = String::with_capacity(value.len().saturating_sub(2));
    let mut escaped = false;
    for character in value[1..value.len() - 1].chars() {
        if escaped {
            unquoted.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else {
            unquoted.push(character);
        }
    }
    if escaped {
        unquoted.push('\\');
    }
    Some(unquoted)
}

// Playwright's built-in attribute engines compare the complete attribute,
// not a CSS token. CSS string quoting preserves punctuation and whitespace;
// control characters need CSS hex escapes rather than JSON escapes.
fn attribute_css_bytes(attribute: &str, value: &str) -> Option<usize> {
    let fixed = attribute.len().checked_add(5)?; // [attribute="value"]
    value.chars().try_fold(fixed, |bytes, character| {
        let character_bytes = match character {
            '\0' => return None,
            '"' | '\\' => 2,
            '\u{1}'..='\u{1f}' | '\u{7f}' => 3 + usize::from((character as u32) >= 0x10),
            _ => character.len_utf8(),
        };
        bytes.checked_add(character_bytes)
    })
}

fn attribute_css_selector(attribute: &str, value: &str) -> Result<Selector, ()> {
    let bytes = attribute_css_bytes(attribute, value).ok_or(())?;
    if bytes > MAX_SELECTOR_STRING_BYTES {
        return Err(());
    }
    let mut css = String::with_capacity(bytes);
    css.push('[');
    css.push_str(attribute);
    css.push_str("=\"");
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for character in value.chars() {
        match character {
            '"' | '\\' => {
                css.push('\\');
                css.push(character);
            }
            '\u{1}'..='\u{1f}' | '\u{7f}' => {
                let code = character as u32;
                css.push('\\');
                if code >= 0x10 {
                    css.push(HEX[(code >> 4) as usize] as char);
                }
                css.push(HEX[(code & 0xf) as usize] as char);
                css.push(' ');
            }
            _ => css.push(character),
        }
    }
    css.push_str("\"]");
    Selector::css(css).map_err(|_| ())
}

fn attribute_engine_body(part: &str) -> Option<(&'static str, &str)> {
    let (name, body) = part.split_once('=')?;
    let attribute = match name.trim_end() {
        "id" => "id",
        "data-test" => "data-test",
        "data-testid" => "data-testid",
        "data-test-id" => "data-test-id",
        _ => return None,
    };
    Some((attribute, body))
}

#[cfg(feature = "light-selectors")]
fn light_attribute_engine_body(part: &str) -> Option<(&'static str, &str)> {
    let (name, body) = part.split_once('=')?;
    let attribute = match name.trim_end() {
        "id:light" => "id",
        "data-testid:light" => "data-testid",
        "data-test-id:light" => "data-test-id",
        "data-test:light" => "data-test",
        _ => return None,
    };
    Some((attribute, body))
}

fn expanded_attribute_engine_body(part: &str) -> Option<(&'static str, &str)> {
    #[cfg(feature = "light-selectors")]
    if let Some(light) = light_attribute_engine_body(part) {
        return Some(light);
    }
    attribute_engine_body(part)
}

fn attribute_selector_expansion(source: &str) -> Result<(usize, bool), HostError> {
    playwright_selector_parts(source).try_fold((0usize, false), |(extra, has_id), part| {
        let trimmed = part.value.trim();
        let Some((attribute, body)) = expanded_attribute_engine_body(trimmed) else {
            return Ok((extra, has_id));
        };
        let has_id = has_id || attribute == "id";
        let css_bytes = attribute_css_bytes(attribute, body).ok_or_else(|| {
            HostError::InvalidArgument(format!(
                "{attribute}= selector contains an unrepresentable NUL"
            ))
        })?;
        extra
            .checked_add(css_bytes.saturating_sub(trimmed.len()))
            .map(|expanded| (expanded, has_id))
            .ok_or_else(|| {
                exhausted(if has_id {
                    "id= selector storage calculation overflowed"
                } else {
                    "attribute selector storage calculation overflowed"
                })
            })
    })
}

fn terminal_css_has_text(source: &str) -> Option<Result<(String, String), ()>> {
    // An attribute-engine body is one exact value, even if it happens to
    // contain text resembling a CSS :has-text() suffix.
    if playwright_selector_parts(source)
        .last()
        .is_some_and(|part| expanded_attribute_engine_body(part.value.trim()).is_some())
    {
        return None;
    }
    const MARKER: &str = ":has-text(";
    let start = source.rfind(MARKER)?;
    if !source.ends_with(')') {
        return Some(Err(()));
    }
    let base = source[..start].trim();
    let body = source[start + MARKER.len()..source.len() - 1].trim();
    if base.is_empty() {
        return Some(Err(()));
    }
    let text = match unquote_text_selector(body) {
        Some(text) => text,
        None => return Some(Err(())),
    };
    Some(Ok((base.to_owned(), text)))
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum CssPseudoError {
    Issue(QueryIssue),
    Limit,
}

impl From<QueryIssue> for CssPseudoError {
    fn from(issue: QueryIssue) -> Self {
        Self::Issue(issue)
    }
}

fn css_pseudo_body(part: &str) -> Option<&str> {
    let trimmed = part.trim();
    if let Some((engine, body)) = trimmed.split_once('=') {
        let engine = engine.trim();
        if engine == "css" {
            return Some(body);
        }
        if !engine.is_empty()
            && engine.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'+' | b':' | b'*')
            })
        {
            return None;
        }
    }
    if trimmed.starts_with("//") || trimmed.starts_with("..") {
        return None;
    }
    Some(trimmed)
}

fn css_pseudo_named_at(bytes: &[u8], index: usize, name: &[u8]) -> bool {
    let Some(end) = index.checked_add(name.len() + 1) else {
        return false;
    };
    bytes.get(index) == Some(&b':')
        && bytes
            .get(index + 1..end)
            .is_some_and(|candidate| candidate.eq_ignore_ascii_case(name))
        && !bytes
            .get(end)
            .is_some_and(|next| next.is_ascii_alphanumeric() || matches!(next, b'_' | b'-'))
}

fn css_comment_end(bytes: &[u8], index: usize) -> Option<usize> {
    if bytes.get(index..index.checked_add(2)?)? != b"/*" {
        return None;
    }
    bytes[index + 2..]
        .windows(2)
        .position(|pair| pair == b"*/")
        .map(|offset| index + 2 + offset + 2)
}

fn css_has_named_pseudo(source: &str, names: &[&[u8]]) -> bool {
    let bytes = source.as_bytes();
    let mut quote = None;
    let mut brackets = 0usize;
    let mut index = 0usize;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'\\' {
            index = index.saturating_add(2);
            continue;
        }
        if quote.is_none()
            && let Some(end) = css_comment_end(bytes, index)
        {
            index = end;
            continue;
        }
        if quote == Some(byte) {
            quote = None;
        } else if quote.is_none() {
            match byte {
                b'\'' | b'"' => quote = Some(byte),
                b'[' => brackets += 1,
                b']' => brackets = brackets.saturating_sub(1),
                b':' if brackets == 0
                    && names
                        .iter()
                        .any(|name| css_pseudo_named_at(bytes, index, name)) =>
                {
                    return true;
                }
                _ => {}
            }
        }
        index += 1;
    }
    false
}

const SUPPORTED_CSS_PSEUDOS: &[&[u8]] = &[b"visible", b"nth-match"];
const OTHER_PLAYWRIGHT_CSS_PSEUDOS: &[&[u8]] = &[
    b"has-text",
    b"has-not-text",
    b"text",
    b"text-is",
    b"text-matches",
    b"right-of",
    b"left-of",
    b"above",
    b"below",
    b"near",
];

fn css_has_supported_pseudo(part: &str) -> bool {
    css_pseudo_body(part).is_some_and(|body| css_has_named_pseudo(body, SUPPORTED_CSS_PSEUDOS))
}

fn css_split_arguments(source: &str) -> Result<Vec<&str>, CssPseudoError> {
    let bytes = source.as_bytes();
    let mut parts = Vec::new();
    let mut quote = None;
    let mut brackets = 0usize;
    let mut parentheses = 0usize;
    let mut start = 0usize;
    let mut index = 0usize;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'\\' {
            index = index.saturating_add(2);
            continue;
        }
        if quote.is_none()
            && let Some(end) = css_comment_end(bytes, index)
        {
            index = end;
            continue;
        }
        if quote == Some(byte) {
            quote = None;
        } else if quote.is_none() {
            match byte {
                b'\'' | b'"' => quote = Some(byte),
                b'[' => brackets += 1,
                b']' => {
                    brackets = brackets
                        .checked_sub(1)
                        .ok_or(QueryIssue::UnsupportedCssExtension)?;
                }
                b'(' if brackets == 0 => parentheses += 1,
                b')' if brackets == 0 => {
                    parentheses = parentheses
                        .checked_sub(1)
                        .ok_or(QueryIssue::UnsupportedCssExtension)?;
                }
                b',' if brackets == 0 && parentheses == 0 => {
                    if parts.len() >= MAX_SELECTOR_NODES {
                        return Err(CssPseudoError::Limit);
                    }
                    parts.push(source[start..index].trim());
                    start = index + 1;
                }
                _ => {}
            }
        }
        index += 1;
    }
    if quote.is_some() || brackets != 0 || parentheses != 0 {
        return Err(QueryIssue::UnsupportedCssExtension.into());
    }
    if parts.len() >= MAX_SELECTOR_NODES {
        return Err(CssPseudoError::Limit);
    }
    parts.push(source[start..].trim());
    Ok(parts)
}

fn strip_ascii_case_prefix<'a>(source: &'a str, prefix: &str) -> Option<&'a str> {
    source
        .get(..prefix.len())
        .filter(|head| head.eq_ignore_ascii_case(prefix))
        .and_then(|_| source.get(prefix.len()..))
}

fn strip_ascii_case_suffix<'a>(source: &'a str, suffix: &str) -> Option<&'a str> {
    let prefix_len = source.len().checked_sub(suffix.len())?;
    let prefix = source.get(..prefix_len)?;
    source[prefix_len..]
        .eq_ignore_ascii_case(suffix)
        .then_some(prefix)
}

fn css_nth_match_index(source: &str) -> Option<i32> {
    // Match one CSS number token before converting its JavaScript number value.
    // A bounded typed selector cannot represent indices above i32::MAX.
    let bytes = source.as_bytes();
    let mut index = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));
    let integer_start = index;
    while bytes.get(index).is_some_and(u8::is_ascii_digit) {
        index += 1;
    }
    let integer_digits = index > integer_start;
    let mut fractional_digits = false;
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        let fraction_start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        fractional_digits = index > fraction_start;
        if !fractional_digits {
            return None;
        }
    }
    if !integer_digits && !fractional_digits {
        return None;
    }
    if matches!(bytes.get(index), Some(b'e' | b'E')) {
        index += 1;
        if matches!(bytes.get(index), Some(b'+' | b'-')) {
            index += 1;
        }
        let exponent_start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if index == exponent_start {
            return None;
        }
    }
    if index != bytes.len() {
        return None;
    }
    let value = source.parse::<f64>().ok()?;
    if !value.is_finite() || value.fract() != 0.0 || value < 1.0 || value > f64::from(i32::MAX) {
        return None;
    }
    Some(value as i32)
}

fn css_pseudo_leaf(
    source: &str,
    materialize: bool,
) -> Result<(Option<Selector>, Shape), CssPseudoError> {
    if css_has_named_pseudo(source, OTHER_PLAYWRIGHT_CSS_PSEUDOS) {
        return Err(QueryIssue::UnsupportedCssExtension.into());
    }
    let source = source.trim();
    let shape = Shape {
        depth: 1,
        nodes: 1,
        strings: source.len(),
    };
    shape.validate().map_err(|_| CssPseudoError::Limit)?;
    if source.is_empty() {
        return Err(QueryIssue::UnsupportedCssExtension.into());
    }
    let selector = materialize
        .then(|| Selector::css(source.to_owned()).expect("preflight checked non-empty CSS"));
    Ok((selector, shape))
}

fn parse_css_pseudo(
    source: &str,
    materialize: bool,
) -> Result<Option<(Option<Selector>, Shape)>, CssPseudoError> {
    if !css_has_named_pseudo(source, SUPPORTED_CSS_PSEUDOS) {
        return Ok(None);
    }
    let source = source.trim();
    if let Some(base) = strip_ascii_case_suffix(source, ":visible") {
        if css_has_named_pseudo(base, SUPPORTED_CSS_PSEUDOS)
            || base.as_bytes().last().is_some_and(|last| {
                last.is_ascii_whitespace() || matches!(last, b'>' | b'+' | b'~' | b',')
            })
        {
            return Err(QueryIssue::UnsupportedCssExtension.into());
        }
        let (selector, shape) =
            css_pseudo_leaf(if base.is_empty() { "*" } else { base }, materialize)?;
        let shape = shape.unary(0).map_err(|_| CssPseudoError::Limit)?;
        return Ok(Some((
            selector.map(|selector| selector.filter_visible(true)),
            shape,
        )));
    }
    if let Some(body) =
        strip_ascii_case_prefix(source, ":nth-match(").and_then(|body| body.strip_suffix(')'))
    {
        let parts = css_split_arguments(body)?;
        if parts.len() < 2 || parts[..parts.len() - 1].iter().any(|part| part.is_empty()) {
            return Err(QueryIssue::NthMatchMissingSelectors.into());
        }
        let index =
            css_nth_match_index(parts[parts.len() - 1]).ok_or(QueryIssue::NthMatchInvalidIndex)?;
        let mut selectors = parts[..parts.len() - 1].iter();
        let first = selectors
            .next()
            .expect("nth-match has a non-empty selector list");
        if css_has_named_pseudo(first, SUPPORTED_CSS_PSEUDOS) {
            return Err(QueryIssue::UnsupportedCssExtension.into());
        }
        let (mut selector, mut shape) = css_pseudo_leaf(first, materialize)?;
        for part in selectors {
            if css_has_named_pseudo(part, SUPPORTED_CSS_PSEUDOS) {
                return Err(QueryIssue::UnsupportedCssExtension.into());
            }
            let (right, right_shape) = css_pseudo_leaf(part, materialize)?;
            shape = shape
                .binary(right_shape)
                .map_err(|_| CssPseudoError::Limit)?;
            selector =
                selector.map(|selector| selector.or(right.expect("materialized right CSS arm")));
        }
        shape = shape.unary(0).map_err(|_| CssPseudoError::Limit)?;
        return Ok(Some((
            selector.map(|selector| selector.nth(index - 1)),
            shape,
        )));
    }
    if let Some(body) =
        strip_ascii_case_prefix(source, ":is(").and_then(|body| body.strip_suffix(')'))
    {
        let parts = css_split_arguments(body)?;
        if parts.len() < 2 || parts.iter().any(|part| part.is_empty()) {
            return Err(QueryIssue::UnsupportedCssExtension.into());
        }
        let mut selectors = parts.iter();
        let first = selectors
            .next()
            .expect("CSS :is has at least two arguments");
        let (mut selector, mut shape) =
            parse_css_pseudo(first, materialize)?.ok_or(QueryIssue::UnsupportedCssExtension)?;
        for part in selectors {
            let (right, right_shape) =
                parse_css_pseudo(part, materialize)?.ok_or(QueryIssue::UnsupportedCssExtension)?;
            shape = shape
                .binary(right_shape)
                .map_err(|_| CssPseudoError::Limit)?;
            selector =
                selector.map(|selector| selector.or(right.expect("materialized right CSS branch")));
        }
        return Ok(Some((selector, shape)));
    }
    Err(QueryIssue::UnsupportedCssExtension.into())
}

fn css_pseudo_part(
    part: &str,
    materialize: bool,
) -> Result<Option<(Option<Selector>, Shape)>, CssPseudoError> {
    css_pseudo_body(part).map_or(Ok(None), |body| parse_css_pseudo(body, materialize))
}

enum CssPseudoChainPart {
    Within(Option<Selector>),
    And(Option<Selector>),
    Nth(i32),
    Visible(bool),
}

fn css_pseudo_chain(
    source: &str,
    materialize: bool,
) -> Result<Option<(Option<Selector>, Shape)>, CssPseudoError> {
    if !playwright_selector_parts(source).any(|part| css_has_supported_pseudo(part.value)) {
        return Ok(None);
    }
    let mut parts = playwright_selector_parts(source);
    let first = parts.next().expect("selector source has one part").value;
    let (mut selector, mut shape) = match css_pseudo_part(first, materialize)? {
        Some(parsed) => parsed,
        None => {
            let selector = playwright_selector_part(first)
                .and_then(SelectorPart::into_initial)
                .map_err(|_| QueryIssue::EmptyCss)?;
            let strings = measure_selector(&selector)
                .map_err(|_| CssPseudoError::Limit)?
                .string_capacity();
            (
                materialize.then_some(selector),
                Shape {
                    depth: 1,
                    nodes: 1,
                    strings,
                },
            )
        }
    };
    for part in parts {
        let parsed = match css_pseudo_part(part.value, materialize)? {
            Some((selector, shape)) => (CssPseudoChainPart::Within(selector), shape),
            None => {
                let part =
                    playwright_selector_part(part.value).map_err(|_| QueryIssue::EmptyCss)?;
                let strings = match &part {
                    SelectorPart::Within(selector) | SelectorPart::And(selector) => {
                        measure_selector(selector)
                            .map_err(|_| CssPseudoError::Limit)?
                            .string_capacity()
                    }
                    SelectorPart::Nth(_) | SelectorPart::Visible(_) => 0,
                };
                let part = match part {
                    SelectorPart::Within(selector) => {
                        CssPseudoChainPart::Within(materialize.then_some(selector))
                    }
                    SelectorPart::And(selector) => {
                        CssPseudoChainPart::And(materialize.then_some(selector))
                    }
                    SelectorPart::Nth(index) => CssPseudoChainPart::Nth(index),
                    SelectorPart::Visible(visible) => CssPseudoChainPart::Visible(visible),
                };
                (
                    part,
                    Shape {
                        depth: 1,
                        nodes: 1,
                        strings,
                    },
                )
            }
        };
        let (part, part_shape) = parsed;
        selector = match part {
            CssPseudoChainPart::Within(descendant) => {
                shape = shape
                    .binary(part_shape)
                    .map_err(|_| CssPseudoError::Limit)?;
                selector.map(|selector| {
                    selector.within(descendant.expect("materialized descendant selector"))
                })
            }
            CssPseudoChainPart::And(other) => {
                shape = shape
                    .binary(part_shape)
                    .map_err(|_| CssPseudoError::Limit)?;
                selector.map(|selector| selector.and(other.expect("materialized And selector")))
            }
            CssPseudoChainPart::Nth(index) => {
                shape = shape.unary(0).map_err(|_| CssPseudoError::Limit)?;
                selector.map(|selector| selector.nth(index))
            }
            CssPseudoChainPart::Visible(visible) => {
                shape = shape.unary(0).map_err(|_| CssPseudoError::Limit)?;
                selector.map(|selector| selector.filter_visible(visible))
            }
        };
    }
    Ok(Some((selector, shape)))
}

pub(crate) enum SelectorPart {
    Within(Selector),
    And(Selector),
    Nth(i32),
    Visible(bool),
}

impl SelectorPart {
    fn into_initial(self) -> Result<Selector, ()> {
        match self {
            Self::Within(selector) => Ok(selector),
            Self::And(_) | Self::Nth(_) | Self::Visible(_) => Err(()),
        }
    }
}

pub(crate) fn playwright_selector_part(part: &str) -> Result<SelectorPart, ()> {
    let trimmed = part.trim();
    if let Some(body) = trimmed.strip_prefix("internal:and=") {
        let nested = serde_json::from_str::<String>(body).map_err(|_| ())?;
        if playwright_selector_parts(&nested).count() > 1 || css_has_supported_pseudo(&nested) {
            return Err(());
        }
        return Ok(SelectorPart::And(
            playwright_selector_part(&nested)?.into_initial()?,
        ));
    }
    if let Some(body) = trimmed.strip_prefix("internal:text=") {
        let (encoded, exact) = body
            .strip_suffix('s')
            .map(|encoded| (encoded, true))
            .or_else(|| body.strip_suffix('i').map(|encoded| (encoded, false)))
            .ok_or(())?;
        let text = serde_json::from_str::<String>(encoded).map_err(|_| ())?;
        return Ok(SelectorPart::Within(Selector::text(text, exact)));
    }
    if let Some(body) = trimmed
        .strip_prefix(":text(")
        .and_then(|body| body.strip_suffix(')'))
    {
        let text = unquote_text_selector(body).ok_or(())?;
        return Ok(SelectorPart::Within(Selector::text(text, true)));
    }
    if let Some(text) = unquote_text_selector(trimmed) {
        return Ok(SelectorPart::Within(Selector::legacy_text(text)));
    }
    if let Some(body) = trimmed.strip_prefix("text=") {
        if let Some(regex) = body.strip_prefix('/') {
            let delimiter = regex.rfind('/').ok_or(())?;
            let source = &regex[..delimiter];
            let flags = &regex[delimiter + 1..];
            let matcher = RegexMatcher::new(source.to_owned(), flags.to_owned()).map_err(|_| ())?;
            return Ok(SelectorPart::Within(Selector::text_regex(matcher)));
        }
        return Ok(SelectorPart::Within(match unquote_text_selector(body) {
            Some(text) => Selector::legacy_text(text),
            None => Selector::text(body, false),
        }));
    }
    if let Some(body) = trimmed.strip_prefix("xpath=./") {
        if body.is_empty()
            || !body
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(());
        }
        let css = format!(":scope>{body}").into_boxed_str().into_string();
        return Selector::css(css).map(SelectorPart::Within).map_err(|_| ());
    }
    if trimmed.starts_with("xpath=") {
        return Err(());
    }
    if let Some((name, body)) = trimmed.split_once('=')
        && name.trim_end() == "css"
    {
        if let Some((Some(selector), _shape)) = parse_css_pseudo(body, true).map_err(|_| ())? {
            return Ok(SelectorPart::Within(selector));
        }
        return Selector::css(body.to_owned())
            .map(SelectorPart::Within)
            .map_err(|_| ());
    }
    if let Some((attribute, body)) = attribute_engine_body(trimmed) {
        return attribute_css_selector(attribute, body).map(SelectorPart::Within);
    }
    if let Some(body) = trimmed.strip_prefix("visible=") {
        return match body {
            "true" => Ok(SelectorPart::Visible(true)),
            "false" => Ok(SelectorPart::Visible(false)),
            _ => Err(()),
        };
    }
    if let Some(body) = trimmed.strip_prefix("nth=") {
        return body.parse::<i32>().map(SelectorPart::Nth).map_err(|_| ());
    }
    if let Some((Some(selector), _shape)) = parse_css_pseudo(trimmed, true).map_err(|_| ())? {
        return Ok(SelectorPart::Within(selector));
    }
    Selector::css(trimmed.to_owned())
        .map(SelectorPart::Within)
        .map_err(|_| ())
}

fn xpath_expression(part: &str) -> Option<&str> {
    let trimmed = part.trim();
    trimmed.strip_prefix("xpath=").or_else(|| {
        (trimmed.starts_with("//") || trimmed.starts_with("..") || trimmed.starts_with('('))
            .then_some(trimmed)
    })
}

fn selector_part_requires_browser_program(part: &str) -> bool {
    let trimmed = part.trim();
    #[cfg(feature = "light-selectors")]
    if light_attribute_engine_body(trimmed).is_some() {
        return true;
    }
    #[cfg(feature = "react-selectors")]
    if trimmed.starts_with("_react=") {
        return true;
    }
    let Some(expression) = xpath_expression(trimmed) else {
        return false;
    };
    if let Some(child) = trimmed.strip_prefix("xpath=./")
        && !child.is_empty()
        && child
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return false;
    }
    !expression.is_empty()
}

#[cfg(all(feature = "light-selectors", any(feature = "handles", test)))]
fn compile_light_attribute_selector(part: &str, root: &str) -> Result<Option<String>, HostError> {
    let Some((attribute, body)) = light_attribute_engine_body(part) else {
        return Ok(None);
    };
    let Selector::Css(css) = attribute_css_selector(attribute, body).map_err(|()| {
        HostError::InvalidArgument(format!(
            "{attribute}:light= selector is not representable as CSS"
        ))
    })?
    else {
        unreachable!("attribute expansion produces a CSS selector");
    };
    let encoded = serde_json::to_string(css.as_str()).map_err(HostError::backend)?;
    // A single-root query excludes descendant shadow roots by construction.
    // Keep this expression inside the opt-in browser-program feature instead
    // of changing the shared selector runtime or the portable wire schema.
    let compiled = format!("[...{root}.querySelectorAll({encoded})]");
    if compiled.len() > MAX_COMPILED_SELECTOR_BYTES {
        return Err(exhausted("compiled light selector exceeds backend bound"));
    }
    Ok(Some(compiled))
}

#[cfg(any(feature = "handles", test))]
pub(crate) fn compile_browser_selector(source: &str, root: &str) -> Result<String, HostError> {
    let mut parts = playwright_selector_parts(source);
    let first = parts
        .next()
        .ok_or_else(|| HostError::Backend("Locator CSS selector must not be empty".into()))?;
    let mut compiled = compile_browser_selector_part(first.value, root, true)?;
    for part in parts {
        let trimmed = part.value.trim();
        #[cfg(feature = "light-selectors")]
        let light_attribute = compile_light_attribute_selector(trimmed, "__pwRoot")?;
        #[cfg(not(feature = "light-selectors"))]
        let light_attribute: Option<String> = None;
        #[cfg(feature = "react-selectors")]
        if let Some(light_attribute) = light_attribute {
            compiled = format!("__pw.within({compiled}, __pwRoot => {light_attribute})");
        } else if let Some(body) = trimmed.strip_prefix("_react=") {
            let body = serde_json::to_string(body).map_err(HostError::backend)?;
            compiled = format!(
                "__pw.within({compiled}, __pwRoot => globalThis[Symbol.for({:?})].query(__pwRoot, {body}))",
                crate::react_selector::SYMBOL,
            );
        } else if let Some(expression) = xpath_expression(trimmed) {
            let expression = encode_xpath(expression)?;
            compiled =
                format!("__pw.within({compiled}, __pwRoot => __pw.xpath(__pwRoot, {expression}))");
        } else {
            match playwright_selector_part(trimmed) {
                Ok(SelectorPart::Within(selector)) => {
                    let descendant =
                        compile_selector(&selector, "__pwRoot").map_err(HostError::backend)?;
                    compiled = format!("__pw.within({compiled}, __pwRoot => {descendant})");
                }
                Ok(SelectorPart::And(selector)) => {
                    let intersection =
                        compile_selector(&selector, root).map_err(HostError::backend)?;
                    compiled = format!(
                        "({compiled}).filter(element => ({intersection}).includes(element))"
                    );
                }
                Ok(SelectorPart::Nth(index)) => {
                    compiled = format!(
                        "(() => {{ const values = {compiled}; let index = {index}; if (index < 0) index += values.length; return index >= 0 && index < values.length ? [values[index]] : []; }})()"
                    );
                }
                Ok(SelectorPart::Visible(visible)) => {
                    compiled = format!(
                        "({compiled}).filter(element => __pw.visible(element) === {visible})"
                    );
                }
                Err(()) => {
                    return Err(HostError::Backend(
                        "Locator CSS selector must not be empty".into(),
                    ));
                }
            }
        }
        #[cfg(not(feature = "react-selectors"))]
        if let Some(light_attribute) = light_attribute {
            compiled = format!("__pw.within({compiled}, __pwRoot => {light_attribute})");
        } else if let Some(expression) = xpath_expression(trimmed) {
            let expression = encode_xpath(expression)?;
            compiled =
                format!("__pw.within({compiled}, __pwRoot => __pw.xpath(__pwRoot, {expression}))");
        } else {
            match playwright_selector_part(trimmed) {
                Ok(SelectorPart::Within(selector)) => {
                    let descendant =
                        compile_selector(&selector, "__pwRoot").map_err(HostError::backend)?;
                    compiled = format!("__pw.within({compiled}, __pwRoot => {descendant})");
                }
                Ok(SelectorPart::And(selector)) => {
                    let intersection =
                        compile_selector(&selector, root).map_err(HostError::backend)?;
                    compiled = format!(
                        "({compiled}).filter(element => ({intersection}).includes(element))"
                    );
                }
                Ok(SelectorPart::Nth(index)) => {
                    compiled = format!(
                        "(() => {{ const values = {compiled}; let index = {index}; if (index < 0) index += values.length; return index >= 0 && index < values.length ? [values[index]] : []; }})()"
                    );
                }
                Ok(SelectorPart::Visible(visible)) => {
                    compiled = format!(
                        "({compiled}).filter(element => __pw.visible(element) === {visible})"
                    );
                }
                Err(()) => {
                    return Err(HostError::Backend(
                        "Locator CSS selector must not be empty".into(),
                    ));
                }
            }
        }
        if compiled.len() > MAX_COMPILED_SELECTOR_BYTES {
            return Err(exhausted(
                "compiled browser selector exceeds its declared backend bound",
            ));
        }
    }
    Ok(compiled)
}

#[cfg(any(feature = "handles", test))]
fn compile_browser_selector_part(
    part: &str,
    root: &str,
    initial: bool,
) -> Result<String, HostError> {
    #[cfg(feature = "light-selectors")]
    if let Some(compiled) = compile_light_attribute_selector(part.trim(), root)? {
        return Ok(compiled);
    }
    #[cfg(feature = "react-selectors")]
    if let Some(body) = part.trim().strip_prefix("_react=") {
        let body = serde_json::to_string(body).map_err(HostError::backend)?;
        return Ok(format!(
            "globalThis[Symbol.for({:?})].query({root}, {body})",
            crate::react_selector::SYMBOL,
        ));
    }
    if let Some(expression) = xpath_expression(part) {
        return Ok(format!("__pw.xpath({root}, {})", encode_xpath(expression)?));
    }
    let part = playwright_selector_part(part.trim())
        .map_err(|()| HostError::Backend("Locator CSS selector must not be empty".into()))?;
    let selector = if initial {
        part.into_initial()
            .map_err(|()| HostError::Backend("Locator CSS selector must not be empty".into()))?
    } else {
        unreachable!("non-initial browser selector parts are compiled by the caller")
    };
    compile_selector(&selector, root).map_err(HostError::backend)
}

#[cfg(any(feature = "handles", test))]
fn encode_xpath(expression: &str) -> Result<String, HostError> {
    if expression.is_empty() {
        return Err(HostError::InvalidArgument(
            "XPath selector must not be empty".into(),
        ));
    }
    serde_json::to_string(expression)
        .map_err(|error| HostError::backend(format!("could not encode XPath: {error}")))
}

#[derive(Clone)]
enum BrowserSelectorPart {
    Typed(Selector),
    Source(String),
}

// Shape::storage_bytes reserves one Selector-sized metadata slot per recipe
// node. Keep browser-program metadata inside that existing declaration rather
// than adding an unaccounted representation at the FFI boundary.
const _: () =
    assert!(std::mem::size_of::<BrowserSelectorPart>() <= std::mem::size_of::<Selector>());

#[derive(Clone)]
pub(crate) struct BrowserSelectorProgram(Box<[BrowserSelectorPart]>);

impl BrowserSelectorProgram {
    fn source(source: String) -> Self {
        Self(Box::from([BrowserSelectorPart::Source(source)]))
    }

    #[cfg(feature = "react-selectors")]
    pub(crate) fn contains_react(&self) -> bool {
        self.0.iter().any(|part| match part {
            BrowserSelectorPart::Source(source) => playwright_selector_parts(source)
                .any(|part| part.value.trim().starts_with("_react=")),
            BrowserSelectorPart::Typed(_) => false,
        })
    }

    fn within(left: &Plan, right: &Plan) -> Option<Self> {
        fn append(parts: &mut Vec<BrowserSelectorPart>, plan: &Plan) -> Option<()> {
            match plan {
                Plan::Selector(selector) => {
                    parts.push(BrowserSelectorPart::Typed(selector.clone()))
                }
                Plan::Browser(program) => parts.extend(program.0.iter().cloned()),
                _ => return None,
            }
            Some(())
        }

        let capacity = match (left, right) {
            (Plan::Browser(left), Plan::Browser(right)) => left.0.len() + right.0.len(),
            (Plan::Browser(left), Plan::Selector(_)) => left.0.len() + 1,
            (Plan::Selector(_), Plan::Browser(right)) => right.0.len() + 1,
            _ => return None,
        };
        let mut parts = Vec::with_capacity(capacity);
        append(&mut parts, left)?;
        append(&mut parts, right)?;
        Some(Self(parts.into_boxed_slice()))
    }

    fn retained_string_capacity(&self) -> Result<usize, HostError> {
        self.0.iter().try_fold(0usize, |total, part| {
            let capacity = match part {
                BrowserSelectorPart::Typed(selector) => measure_selector(selector)
                    .map_err(|error| exhausted(error.message()))?
                    .string_capacity(),
                BrowserSelectorPart::Source(source) => source.capacity(),
            };
            total.checked_add(capacity).ok_or_else(|| {
                exhausted("browser selector retained storage calculation overflowed")
            })
        })
    }

    fn validate_storage(&self, shape: Shape) -> Result<(), HostError> {
        if self.0.len() > shape.nodes || self.retained_string_capacity()? > shape.strings {
            return Err(exhausted(
                "browser selector storage exceeds its reservation",
            ));
        }
        Ok(())
    }

    #[cfg(feature = "handles")]
    pub(crate) fn compile(&self, root: &str) -> Result<String, HostError> {
        fn compile_part(part: &BrowserSelectorPart, root: &str) -> Result<String, HostError> {
            match part {
                BrowserSelectorPart::Typed(selector) => {
                    compile_selector(selector, root).map_err(HostError::backend)
                }
                BrowserSelectorPart::Source(source) => compile_browser_selector(source, root),
            }
        }

        let mut parts = self.0.iter();
        let first = parts.next().ok_or_else(|| {
            HostError::Backend("browser selector program must contain one part".into())
        })?;
        let mut compiled = compile_part(first, root)?;
        if compiled.len() > MAX_COMPILED_SELECTOR_BYTES {
            return Err(exhausted(
                "compiled browser selector exceeds its declared backend bound",
            ));
        }
        for part in parts {
            let descendant = compile_part(part, "__pwRoot")?;
            compiled = format!("__pw.within({compiled}, __pwRoot => {descendant})");
            if compiled.len() > MAX_COMPILED_SELECTOR_BYTES {
                return Err(exhausted(
                    "compiled browser selector exceeds its declared backend bound",
                ));
            }
        }
        Ok(compiled)
    }
}

#[derive(Clone, PartialEq)]
struct CapturedSelector {
    prefix: Selector,
    suffix: Option<Selector>,
}

impl CapturedSelector {
    fn materialize(&self) -> Selector {
        match &self.suffix {
            Some(suffix) => self.prefix.clone().filter_has(suffix.clone()),
            None => self.prefix.clone(),
        }
    }

    fn map_active(mut self, compose: impl FnOnce(Selector) -> Selector) -> Self {
        if let Some(suffix) = self.suffix.take() {
            self.suffix = Some(compose(suffix));
        } else {
            self.prefix = compose(self.prefix);
        }
        self
    }

    fn append_descendant(mut self, descendant: Selector) -> Self {
        self.suffix = Some(match self.suffix.take() {
            Some(suffix) => suffix.within(descendant),
            None => descendant,
        });
        self
    }
}

#[derive(Clone)]
enum Plan {
    Selector(Selector),
    Captured(CapturedSelector),
    Browser(BrowserSelectorProgram),
    #[cfg(feature = "selector-registration")]
    Registered(crate::selector_registry::RegisteredSelectorProgram),
    #[cfg(feature = "layout-selectors")]
    Layout(crate::layout_selectors::LayoutSelectorProgram),
    Issue(QueryIssue),
}

fn is_browser_program(plan: &Plan) -> bool {
    if matches!(plan, Plan::Browser(_)) {
        return true;
    }
    #[cfg(feature = "selector-registration")]
    if matches!(plan, Plan::Registered(_)) {
        return true;
    }
    #[cfg(feature = "layout-selectors")]
    if matches!(plan, Plan::Layout(_)) {
        return true;
    }
    false
}

struct QueryInner {
    shape: Shape,
    plan: Plan,
    lease: QueryLease,
}

/// Preparing a value reserves capacity before owned strings or trees exist.
/// Dropping an unfinished draft releases that capacity.
pub struct QueryDraft {
    shape: Shape,
    lease: QueryLease,
}

/// One-shot ElementHandle selector admission. Ordinary typed selectors retain
/// the shared Locator recipe; full XPath remains a separately tagged browser
/// program instead of being mislabeled as CSS.
#[cfg(any(
    feature = "element-handle-dom",
    feature = "element-handle-wait-for-selector"
))]
pub struct ElementHandleQuery(ElementHandleQueryInner);

#[cfg(any(
    feature = "element-handle-dom",
    feature = "element-handle-wait-for-selector"
))]
enum ElementHandleQueryInner {
    Locator(LocatorQuery),
    XPath {
        expression: String,
        lease: QueryLease,
    },
}

#[cfg(any(
    feature = "element-handle-dom",
    feature = "element-handle-wait-for-selector"
))]
pub(crate) enum ElementHandleQueryPlan {
    Selector(Selector),
    XPath(String),
}

impl QueryDraft {
    #[cfg(any(
        feature = "element-handle-dom",
        feature = "element-handle-wait-for-selector"
    ))]
    pub fn element_handle(self, source: String) -> Result<ElementHandleQuery, HostError> {
        if source.capacity() > self.shape.strings {
            return Err(exhausted(
                "ElementHandle selector capacity exceeds its reservation",
            ));
        }
        let trimmed = source.trim();
        let expression = trimmed.strip_prefix("xpath=").or_else(|| {
            (trimmed.starts_with("//") || trimmed.starts_with("..") || trimmed.starts_with('('))
                .then_some(trimmed)
        });
        if let Some(expression) = expression
            && playwright_selector_parts(trimmed).count() == 1
        {
            if expression.is_empty() {
                return Err(HostError::InvalidArgument(
                    "ElementHandle XPath selector must not be empty".into(),
                ));
            }
            return Ok(ElementHandleQuery(ElementHandleQueryInner::XPath {
                expression: expression.to_owned(),
                lease: self.lease,
            }));
        }
        self.css(source)
            .map(ElementHandleQueryInner::Locator)
            .map(ElementHandleQuery)
    }

    pub fn css(self, source: String) -> Result<LocatorQuery, HostError> {
        if source.capacity() > self.shape.strings {
            return Err(exhausted("Locator input capacity exceeds its reservation"));
        }
        #[cfg(feature = "layout-selectors")]
        match crate::layout_selectors::parse_layout_selector(&source) {
            Ok(Some(program)) => {
                let shape = Shape {
                    depth: 1,
                    nodes: program.part_count(),
                    strings: program.selector_bytes(),
                };
                shape.validate()?;
                let budget = self.lease.budget.clone();
                let final_draft = budget.prepare(shape)?;
                drop(self);
                return final_draft.finish(Plan::Layout(program));
            }
            Ok(None) => {}
            Err(issue) => return self.finish(Plan::Issue(QueryIssue::Layout(issue))),
        }
        let capture_index = match capture_part_index(&source) {
            Ok(index) => index,
            Err(issue) => return self.finish(Plan::Issue(issue)),
        };
        if let Some((engine_start, engine_end)) = unknown_engine(&source) {
            return self.finish(Plan::Issue(QueryIssue::UnknownEngine {
                selector: source,
                engine_start,
                engine_end,
            }));
        }
        if let Some(capture_index) = capture_index {
            let captured = playwright_selector_parts(&source)
                .nth(capture_index)
                .expect("capture index belongs to a selector part")
                .value
                .trim();
            let engine = captured
                .strip_prefix('*')
                .and_then(|part| part.split_once('='))
                .map(|(engine, _body)| engine.trim());
            if !matches!(engine, Some("css" | "text")) {
                return self.finish(Plan::Issue(QueryIssue::CapturedSelector));
            }
            if playwright_selector_parts(&source).any(|part| {
                let part = part.value.trim();
                part.starts_with("nth=") || part.starts_with("*nth=")
            }) {
                return self.finish(Plan::Issue(QueryIssue::NthAfterCapture));
            }
            let (attribute_expansion, _) = attribute_selector_expansion(&source)?;
            let parts = playwright_selector_parts(&source).count();
            let string_bytes = source
                .len()
                .checked_add(attribute_expansion)
                .ok_or_else(|| exhausted("captured selector storage calculation overflowed"))?;
            let shape = Shape {
                depth: parts,
                nodes: parts.saturating_mul(2).saturating_sub(1),
                strings: string_bytes,
            };
            let budget = self.lease.budget.clone();
            let final_draft = budget.prepare(shape)?;
            let plan = match capture_selector(&source, capture_index) {
                Ok(selector) => Plan::Captured(selector),
                Err(issue) => Plan::Issue(issue),
            };
            drop(self);
            return final_draft.finish(plan);
        }
        let browser_program = playwright_selector_parts(&source)
            .any(|part| selector_part_requires_browser_program(part.value));
        if !browser_program {
            match css_pseudo_chain(&source, false) {
                Ok(Some((None, shape))) => {
                    let budget = self.lease.budget.clone();
                    let final_draft = budget.prepare(shape)?;
                    let (Some(selector), built_shape) = css_pseudo_chain(&source, true)
                        .expect("CSS pseudo preflight and materialization use the same source")
                        .expect("CSS pseudo preflight found a selector")
                    else {
                        unreachable!("materialized CSS pseudo must have a selector");
                    };
                    debug_assert_eq!(built_shape.nodes, shape.nodes);
                    debug_assert_eq!(built_shape.depth, shape.depth);
                    debug_assert_eq!(built_shape.strings, shape.strings);
                    drop(self);
                    return final_draft.finish(Plan::Selector(selector));
                }
                Ok(Some((Some(_), _))) => {
                    unreachable!("CSS pseudo preflight must not materialize its selector");
                }
                Ok(None) => {}
                Err(CssPseudoError::Issue(issue)) => return self.finish(Plan::Issue(issue)),
                Err(CssPseudoError::Limit) => {
                    return Err(exhausted("Locator CSS extension exceeds selector limits"));
                }
            }
        }
        if let Some(parsed) = terminal_css_has_text(&source) {
            let selector = parsed
                .and_then(|(base, text)| {
                    Selector::css(base)
                        .map(|selector| selector.filter_text(text, false))
                        .map_err(|_| ())
                })
                .map_err(|()| {
                    HostError::Backend("Locator CSS selector must not be empty".into())
                })?;
            let storage =
                measure_selector(&selector).map_err(|error| exhausted(error.message()))?;
            let shape = Shape {
                depth: 2,
                nodes: 2,
                strings: storage.string_capacity(),
            };
            let budget = self.lease.budget.clone();
            let final_draft = budget.prepare(shape)?;
            drop(self);
            return final_draft.finish(Plan::Selector(selector));
        }
        let (attribute_expansion, has_id_engine) = attribute_selector_expansion(&source)?;
        let string_bytes = source
            .len()
            .checked_add(attribute_expansion)
            .ok_or_else(|| {
                exhausted(if has_id_engine {
                    "id= selector storage calculation overflowed"
                } else {
                    "attribute selector storage calculation overflowed"
                })
            })?;
        if string_bytes > MAX_SELECTOR_STRING_BYTES {
            return Err(exhausted(if has_id_engine {
                "id= selector exceeds selector string limit"
            } else {
                "attribute selector exceeds selector string limit"
            }));
        }
        let parts = playwright_selector_parts(&source).count();
        if parts > 1 {
            let mut shape = Shape {
                depth: 1,
                nodes: 1,
                strings: string_bytes,
            };
            for _ in 1..parts {
                shape = shape.binary(Shape {
                    depth: 1,
                    nodes: 1,
                    strings: 0,
                })?;
            }
            // The original leaf draft remains the scratch reservation for the
            // callback-owned source while the exact final tree is admitted.
            let budget = self.lease.budget.clone();
            let final_draft = budget.prepare(shape)?;
            if browser_program {
                drop(self);
                return final_draft.finish(Plan::Browser(BrowserSelectorProgram::source(source)));
            }
            let mut selectors = playwright_selector_parts(&source);
            let first = selectors
                .next()
                .expect("selector chain has a first part")
                .value
                .trim();
            let mut selector =
                match playwright_selector_part(first).and_then(SelectorPart::into_initial) {
                    Ok(selector) => selector,
                    Err(_) => {
                        drop(self);
                        return final_draft.finish(Plan::Issue(QueryIssue::EmptyCss));
                    }
                };
            for part in selectors {
                let part = match playwright_selector_part(part.value.trim()) {
                    Ok(part) => part,
                    Err(_) => {
                        drop(self);
                        return final_draft.finish(Plan::Issue(QueryIssue::EmptyCss));
                    }
                };
                selector = match part {
                    SelectorPart::Within(descendant) => selector.within(descendant),
                    SelectorPart::And(other) => selector.and(other),
                    SelectorPart::Nth(index) => selector.nth(index),
                    SelectorPart::Visible(visible) => selector.filter_visible(visible),
                };
            }
            drop(self);
            return final_draft.finish(Plan::Selector(selector));
        }
        if browser_program {
            return self.finish(Plan::Browser(BrowserSelectorProgram::source(source)));
        }
        let expanded_draft = if attribute_expansion == 0 {
            None
        } else {
            let shape = Shape {
                strings: string_bytes,
                ..self.shape
            };
            let budget = self.lease.budget.clone();
            Some(budget.prepare(shape)?)
        };
        let plan = match playwright_selector_part(&source).and_then(SelectorPart::into_initial) {
            Ok(selector) => Plan::Selector(selector),
            Err(_) => Plan::Issue(QueryIssue::EmptyCss),
        };
        if let Some(draft) = expanded_draft {
            drop(self);
            draft.finish(plan)
        } else {
            self.finish(plan)
        }
    }

    #[cfg(feature = "selector-registration")]
    pub fn css_registered(
        self,
        source: String,
        registry: &crate::selector_registry::SelectorRegistry,
    ) -> Result<LocatorQuery, HostError> {
        // A registered-engine program cannot represent a capture marker. Use
        // the ordinary parser so a supported builtin capture stays typed and
        // a custom-engine capture fails closed instead of dropping `*`.
        if !matches!(capture_part_index(&source), Ok(None)) {
            return self.css(source);
        }
        let Some(program) = registry.parse_program(&source)? else {
            return self.css(source);
        };
        let shape = Shape {
            depth: program.part_count(),
            nodes: program.part_count(),
            strings: source.len(),
        };
        shape.validate()?;
        if shape.nodes == 1 {
            return self.finish(Plan::Registered(program));
        }
        let budget = self.lease.budget.clone();
        let final_draft = budget.prepare(shape)?;
        drop(self);
        final_draft.finish(Plan::Registered(program))
    }

    fn finish(self, plan: Plan) -> Result<LocatorQuery, HostError> {
        match &plan {
            Plan::Selector(selector) => {
                let storage =
                    measure_selector(selector).map_err(|error| exhausted(error.message()))?;
                if storage.string_capacity() > self.shape.strings {
                    return Err(exhausted("Locator string capacity exceeds its reservation"));
                }
            }
            Plan::Captured(captured) => {
                let storage = measure_selector(&captured.materialize())
                    .map_err(|error| exhausted(error.message()))?;
                if storage.string_capacity() > self.shape.strings {
                    return Err(exhausted("Locator string capacity exceeds its reservation"));
                }
            }
            Plan::Browser(program) => {
                program.validate_storage(self.shape)?;
            }
            _ => {}
        }
        Ok(LocatorQuery(Arc::new(QueryInner {
            shape: self.shape,
            plan,
            lease: self.lease,
        })))
    }
}

#[cfg(any(
    feature = "element-handle-dom",
    feature = "element-handle-wait-for-selector"
))]
impl ElementHandleQuery {
    pub(crate) fn command(self) -> Result<(ElementHandleQueryPlan, QueryLease), HostError> {
        match self.0 {
            ElementHandleQueryInner::Locator(query) => query
                .command_selector()
                .map(|(selector, lease)| (ElementHandleQueryPlan::Selector(selector), lease)),
            ElementHandleQueryInner::XPath { expression, lease } => {
                Ok((ElementHandleQueryPlan::XPath(expression), lease))
            }
        }
    }
}

#[derive(Clone)]
pub struct LocatorQuery(Arc<QueryInner>);

#[derive(Clone, Copy)]
pub enum QueryCombination {
    And,
    Or,
    Within,
    Has,
    HasNot,
}

impl LocatorQuery {
    #[cfg(feature = "react-selectors")]
    pub(crate) fn contains_react(&self) -> bool {
        matches!(&self.0.plan, Plan::Browser(program) if program.contains_react())
    }
    pub fn copy(&self) -> Result<Self, HostError> {
        let draft = self.0.lease.budget.prepare(self.0.shape)?;
        draft.finish(self.0.plan.clone())
    }

    pub fn deferred_issue(&self) -> Option<QueryIssue> {
        match &self.0.plan {
            Plan::Issue(issue) => Some(issue.clone()),
            Plan::Selector(_) | Plan::Captured(_) => None,
            Plan::Browser(_) => None,
            #[cfg(feature = "selector-registration")]
            Plan::Registered(_) => None,
            #[cfg(feature = "layout-selectors")]
            Plan::Layout(_) => None,
        }
    }

    #[cfg(feature = "handles")]
    pub(crate) fn same_recipe(&self, other: &Self) -> bool {
        if Arc::ptr_eq(&self.0, &other.0) {
            return true;
        }
        match (&self.0.plan, &other.0.plan) {
            (Plan::Selector(left), Plan::Selector(right)) => left == right,
            (Plan::Captured(left), Plan::Captured(right)) => left == right,
            (Plan::Issue(left), Plan::Issue(right)) => left == right,
            _ => false,
        }
    }

    pub(crate) fn command_selector(&self) -> Result<(Selector, QueryLease), HostError> {
        match &self.0.plan {
            Plan::Issue(issue) => Err(issue.error()),
            Plan::Selector(selector) => {
                let draft = self.0.lease.budget.prepare(self.0.shape)?;
                Ok((selector.clone(), draft.lease))
            }
            Plan::Captured(captured) => {
                let draft = self.0.lease.budget.prepare(self.0.shape)?;
                Ok((captured.materialize(), draft.lease))
            }
            Plan::Browser(_) => Err(HostError::UnsupportedOperation(
                "browser-program selector execution is unavailable for this operation".into(),
            )),
            #[cfg(feature = "selector-registration")]
            Plan::Registered(_) => Err(HostError::UnsupportedOperation(
                "registered selector execution is unavailable for this operation".into(),
            )),
            #[cfg(feature = "layout-selectors")]
            Plan::Layout(_) => Err(HostError::UnsupportedOperation(
                "layout selector execution is unavailable for this operation".into(),
            )),
        }
    }

    #[cfg(any(
        feature = "selector-registration",
        feature = "layout-selectors",
        feature = "handles"
    ))]
    pub(crate) fn command_plan(&self) -> Result<(CommandPlan, QueryLease), HostError> {
        let draft = self.0.lease.budget.prepare(self.0.shape)?;
        let plan = match &self.0.plan {
            Plan::Issue(issue) => return Err(issue.error()),
            Plan::Selector(selector) => CommandPlan::Selector(selector.clone()),
            Plan::Captured(captured) => CommandPlan::Selector(captured.materialize()),
            Plan::Browser(source) => CommandPlan::Browser(source.clone()),
            #[cfg(feature = "selector-registration")]
            Plan::Registered(program) => CommandPlan::Registered(program.clone()),
            #[cfg(feature = "layout-selectors")]
            Plan::Layout(program) => CommandPlan::Layout(program.clone()),
        };
        Ok((plan, draft.lease))
    }

    pub fn nth(&self, index: i32) -> Result<Self, HostError> {
        if matches!(
            self.0.plan,
            Plan::Captured(_) | Plan::Issue(QueryIssue::CapturedSelector)
        ) {
            let draft = self.0.lease.budget.prepare(self.0.shape.unary(0)?)?;
            return draft.finish(Plan::Issue(QueryIssue::NthAfterCapture));
        }
        #[cfg(feature = "selector-registration")]
        if let Plan::Registered(program) = &self.0.plan
            && index == 0
        {
            let draft = self.0.lease.budget.prepare(self.0.shape.unary(0)?)?;
            return draft.finish(Plan::Registered(program.first()?));
        }
        self.unary(0, |selector| selector.nth(index))
    }

    pub fn visible(&self, visible: bool) -> Result<Self, HostError> {
        self.unary(0, |selector| selector.filter_visible(visible))
    }

    pub fn filter_text(&self, text: &str, exclude: bool) -> Result<Self, HostError> {
        // Playwright ignores empty string hasText/hasNotText filters. An empty
        // RegExp remains meaningful and uses the separate regex path.
        if text.is_empty() {
            return Ok(self.clone());
        }
        self.unary(text.len(), |selector| {
            if exclude {
                selector.filter_not_text(text, false)
            } else {
                selector.filter_text(text, false)
            }
        })
    }

    pub fn filter_regex(
        &self,
        source: &str,
        flags: &str,
        exclude: bool,
    ) -> Result<Self, HostError> {
        if is_browser_program(&self.0.plan) {
            return Err(HostError::UnsupportedOperation(
                "typed Locator filters over browser-program selectors are not installed".into(),
            ));
        }
        let string_bytes = source.len().saturating_add(flags.len());
        let draft = self
            .0
            .lease
            .budget
            .prepare(self.0.shape.unary(string_bytes)?)?;
        // Construct the retained matcher only after the complete derived
        // recipe has been admitted. This keeps hostile FFI strings from
        // allocating outside the caller-owned query budget.
        let matcher = RegexMatcher::new(source, flags)
            .map_err(|error| HostError::InvalidArgument(error.to_string()))?;
        let plan = match &self.0.plan {
            Plan::Selector(selector) => {
                let filtered = if exclude {
                    selector.clone().filter_not_text_regex(matcher)
                } else {
                    selector.clone().filter_text_regex(matcher)
                };
                Plan::Selector(filtered)
            }
            Plan::Captured(captured) => Plan::Captured(captured.clone().map_active(|selector| {
                if exclude {
                    selector.filter_not_text_regex(matcher)
                } else {
                    selector.filter_text_regex(matcher)
                }
            })),
            Plan::Browser(_) => unreachable!("browser selector returned before admission"),
            Plan::Issue(issue) => Plan::Issue(issue.clone()),
            #[cfg(feature = "selector-registration")]
            Plan::Registered(_) => unreachable!("registered selector returned before admission"),
            #[cfg(feature = "layout-selectors")]
            Plan::Layout(_) => unreachable!("layout selector returned before admission"),
        };
        draft.finish(plan)
    }

    fn unary(
        &self,
        strings: usize,
        compose: impl FnOnce(Selector) -> Selector,
    ) -> Result<Self, HostError> {
        if is_browser_program(&self.0.plan) {
            return Err(HostError::UnsupportedOperation(
                "typed Locator filters over browser-program selectors are not installed".into(),
            ));
        }
        let draft = self.0.lease.budget.prepare(self.0.shape.unary(strings)?)?;
        let plan = match &self.0.plan {
            Plan::Selector(selector) => Plan::Selector(compose(selector.clone())),
            Plan::Captured(captured) => Plan::Captured(captured.clone().map_active(compose)),
            Plan::Browser(_) => unreachable!("browser selector returned before admission"),
            Plan::Issue(issue) => Plan::Issue(issue.clone()),
            #[cfg(feature = "selector-registration")]
            Plan::Registered(_) => unreachable!("registered selector returned before admission"),
            #[cfg(feature = "layout-selectors")]
            Plan::Layout(_) => unreachable!("layout selector returned before admission"),
        };
        draft.finish(plan)
    }

    pub fn combine(&self, other: &Self, combination: QueryCombination) -> Result<Self, HostError> {
        if !Arc::ptr_eq(&self.0.lease.budget.0, &other.0.lease.budget.0) {
            return Err(HostError::InvalidArgument(
                "Locator query budgets must share one owner".into(),
            ));
        }
        let has_browser =
            matches!(self.0.plan, Plan::Browser(_)) || matches!(other.0.plan, Plan::Browser(_));
        if has_browser {
            if !matches!(combination, QueryCombination::Within) {
                return Err(HostError::UnsupportedOperation(
                    "browser-program selectors only support descendant Locator composition".into(),
                ));
            }
            let compatible = matches!(
                (&self.0.plan, &other.0.plan),
                (Plan::Issue(_), _)
                    | (_, Plan::Issue(_))
                    | (Plan::Selector(_), Plan::Browser(_))
                    | (Plan::Browser(_), Plan::Selector(_))
                    | (Plan::Browser(_), Plan::Browser(_))
            );
            if !compatible {
                return Err(HostError::UnsupportedOperation(
                    "typed Locator composition over this browser-program selector is not installed"
                        .into(),
                ));
            }
            let draft = self
                .0
                .lease
                .budget
                .prepare(self.0.shape.binary(other.0.shape)?)?;
            let plan = match (&self.0.plan, &other.0.plan) {
                (Plan::Issue(issue), _) | (_, Plan::Issue(issue)) => Plan::Issue(issue.clone()),
                (left, right) => BrowserSelectorProgram::within(left, right)
                    .map(Plan::Browser)
                    .expect("browser-program composition was validated before allocation"),
            };
            return draft.finish(plan);
        }
        if is_browser_program(&self.0.plan) || is_browser_program(&other.0.plan) {
            return Err(HostError::UnsupportedOperation(
                "typed Locator composition over browser-program selectors is not installed".into(),
            ));
        }
        let draft = self
            .0
            .lease
            .budget
            .prepare(self.0.shape.binary(other.0.shape)?)?;
        let plan = match (&self.0.plan, &other.0.plan) {
            (Plan::Issue(issue), _) | (_, Plan::Issue(issue)) => Plan::Issue(issue.clone()),
            (Plan::Selector(left), Plan::Selector(right)) => {
                let selector = match combination {
                    QueryCombination::And => left.clone().and(right.clone()),
                    QueryCombination::Or => left.clone().or(right.clone()),
                    QueryCombination::Within => left.clone().within(right.clone()),
                    QueryCombination::Has => left.clone().filter_has(right.clone()),
                    QueryCombination::HasNot => left.clone().filter_has_not(right.clone()),
                };
                Plan::Selector(selector)
            }
            (Plan::Captured(left), Plan::Selector(right)) => {
                let captured = match combination {
                    QueryCombination::Within => left.clone().append_descendant(right.clone()),
                    QueryCombination::And => {
                        left.clone().map_active(|active| active.and(right.clone()))
                    }
                    QueryCombination::Or => {
                        left.clone().map_active(|active| active.or(right.clone()))
                    }
                    QueryCombination::Has => left
                        .clone()
                        .map_active(|active| active.filter_has(right.clone())),
                    QueryCombination::HasNot => left
                        .clone()
                        .map_active(|active| active.filter_has_not(right.clone())),
                };
                Plan::Captured(captured)
            }
            (Plan::Selector(left), Plan::Captured(right)) => {
                if matches!(combination, QueryCombination::Within) {
                    let mut captured = right.clone();
                    captured.prefix = left.clone().within(captured.prefix);
                    Plan::Captured(captured)
                } else {
                    Plan::Issue(QueryIssue::CapturedSelector)
                }
            }
            (Plan::Captured(_), Plan::Captured(_)) => Plan::Issue(QueryIssue::CapturedSelector),
            (Plan::Browser(_), _) | (_, Plan::Browser(_)) => {
                unreachable!("browser selector returned before composition admission")
            }
            #[cfg(feature = "selector-registration")]
            (Plan::Registered(_), _) | (_, Plan::Registered(_)) => {
                unreachable!("registered selector returned before composition admission")
            }
            #[cfg(feature = "layout-selectors")]
            (Plan::Layout(_), _) | (_, Plan::Layout(_)) => {
                unreachable!("layout selector returned before composition admission")
            }
        };
        draft.finish(plan)
    }
}

#[cfg(any(
    feature = "selector-registration",
    feature = "layout-selectors",
    feature = "handles"
))]
pub(crate) enum CommandPlan {
    Selector(Selector),
    Browser(BrowserSelectorProgram),
    #[cfg(feature = "selector-registration")]
    Registered(crate::selector_registry::RegisteredSelectorProgram),
    #[cfg(feature = "layout-selectors")]
    Layout(crate::layout_selectors::LayoutSelectorProgram),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn budget() -> QueryBudget {
        QueryBudget::new(QueryLimits {
            recipes: 1024,
            bytes: 8 * 1024 * 1024,
        })
        .unwrap()
    }

    fn css(budget: &QueryBudget, value: &str) -> LocatorQuery {
        budget
            .prepare_leaf(value.len())
            .unwrap()
            .css(value.to_owned())
            .unwrap()
    }

    #[cfg(feature = "react-selectors")]
    #[test]
    fn react_selector_is_an_opt_in_browser_plan_with_bounded_source() {
        // Arrange
        let budget = budget();
        let source = "_react=BookItem[name *= \"Gatsby\"] >> _react=li";

        // Act
        let query = css(&budget, source);
        let (plan, _lease) = query.command_plan().unwrap();
        let CommandPlan::Browser(program) = plan else {
            panic!("React selectors must not be parsed as CSS");
        };
        let compiled = program.compile("document").unwrap();

        // Assert
        assert!(program.contains_react());
        assert_eq!(compiled.matches("pwright.react-selector/1").count(), 2);
        assert!(compiled.contains("__pw.within"));
        assert!(compiled.len() <= MAX_COMPILED_SELECTOR_BYTES);
    }

    #[cfg(feature = "react-selectors")]
    #[test]
    fn react_selector_composes_under_typed_css_without_losing_scope() {
        // Arrange
        let budget = budget();
        let source = "css=#root2 >> _react=BookItem";

        // Act
        let query = css(&budget, source);
        let (plan, _lease) = query.command_plan().unwrap();
        let CommandPlan::Browser(program) = plan else {
            panic!("React descendant must retain browser plan");
        };
        let compiled = program.compile("document").unwrap();

        // Assert
        assert!(compiled.contains("#root2"));
        assert!(compiled.contains("__pwRoot"));
        assert!(compiled.contains("pwright.react-selector/1"));
    }

    #[test]
    fn abandoned_input_admission_releases_its_count_and_bytes() {
        // Arrange
        let budget = budget();
        let draft = budget.prepare_leaf(1024).unwrap();
        assert_eq!(budget.metrics().unwrap().recipes, 1);
        // Act
        drop(draft);
        // Assert
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[test]
    fn empty_css_is_a_lazy_issue_not_a_fake_valid_selector() {
        // Arrange
        let budget = budget();
        let query = css(&budget, "");
        // Act
        let result = query.command_selector();
        // Assert
        assert_eq!(query.deferred_issue(), Some(QueryIssue::EmptyCss));
        assert!(matches!(result, Err(HostError::Backend(_))));
        assert_eq!(budget.metrics().unwrap().recipes, 1);
    }

    #[test]
    fn unknown_playwright_selector_engine_is_a_lazy_precise_issue() {
        // Arrange
        let budget = budget();
        let query = css(&budget, "hey=what");

        // Act
        let error = match query.command_selector() {
            Ok(_) => panic!("unknown engine must remain a deferred query issue"),
            Err(error) => error,
        };

        // Assert
        assert!(matches!(
            query.deferred_issue(),
            Some(QueryIssue::UnknownEngine {
                ref selector,
                engine_start: 0,
                engine_end: 3,
            }) if selector == "hey=what"
        ));
        assert!(matches!(
            error,
            HostError::Backend(message)
                if message == "Unknown engine \"hey\" while parsing selector hey=what"
        ));
        assert_eq!(budget.metrics().unwrap().recipes, 1);
    }

    #[test]
    fn id_engine_matches_the_complete_attribute_even_with_css_metacharacters() {
        // Arrange
        let budget = budget();
        let query = css(&budget, "id=one.two#three[4]:five");

        // Act
        let (selector, lease) = query.command_selector().unwrap();
        let compiled = compile_selector(&selector, "document").unwrap();

        // Assert
        assert!(matches!(
            selector,
            Selector::Css(ref css) if css.as_str() == r#"[id="one.two#three[4]:five"]"#
        ));
        assert_eq!(
            compiled,
            r#"__pw.css(document, "[id=\"one.two#three[4]:five\"]")"#
        );
        assert!(query.0.shape.strings >= r#"[id="one.two#three[4]:five"]"#.len());
        drop(lease);
        drop(query);
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[cfg(all(feature = "light-selectors", feature = "handles"))]
    #[test]
    fn light_id_engine_compiles_without_shadow_piercing() {
        // Arrange
        let budget = budget();
        let query = css(&budget, "id:light=target");

        // Act
        let (plan, lease) = query.command_plan().unwrap();
        let CommandPlan::Browser(program) = plan else {
            panic!("light selector requires the opt-in browser program");
        };
        let compiled = program.compile("document").unwrap();

        // Assert
        assert_eq!(
            compiled,
            r#"[...document.querySelectorAll("[id=\"target\"]")]"#
        );
        drop((lease, query));
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[cfg(all(feature = "light-selectors", feature = "handles"))]
    #[test]
    fn light_test_id_engine_preserves_exact_value_and_chain_scope() {
        // Arrange
        let budget = budget();
        let query = css(&budget, "section >> data-testid:light=foo");

        // Act
        let (plan, lease) = query.command_plan().unwrap();
        let CommandPlan::Browser(program) = plan else {
            panic!("light selector requires the opt-in browser program");
        };
        let compiled = program.compile("document").unwrap();

        // Assert
        assert!(compiled.contains("__pw.css(document, \"section\")"));
        assert!(compiled.contains("[...__pwRoot.querySelectorAll(\"[data-testid=\\\"foo\\\"]\")]"));
        drop((lease, query));
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[cfg(all(feature = "light-selectors", feature = "handles"))]
    #[test]
    fn light_data_test_id_alias_escapes_css_and_javascript_strings() {
        // Arrange
        let budget = budget();
        let query = css(&budget, "data-test-id:light=a\"b\\c\nz");

        // Act
        let (plan, lease) = query.command_plan().unwrap();
        let CommandPlan::Browser(program) = plan else {
            panic!("light selector requires the opt-in browser program");
        };
        let compiled = program.compile("document").unwrap();

        // Assert
        assert_eq!(
            compiled,
            r#"[...document.querySelectorAll("[data-test-id=\"a\\\"b\\\\c\\a z\"]")]"#
        );
        drop((lease, query));
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[cfg(all(feature = "light-selectors", feature = "handles"))]
    #[test]
    fn light_data_test_alias_escapes_css_and_javascript_strings() {
        // Arrange
        let budget = budget();
        let query = css(&budget, "data-test:light=a\"b\\c\nz");

        // Act
        let (plan, lease) = query.command_plan().unwrap();
        let CommandPlan::Browser(program) = plan else {
            panic!("light selector requires the opt-in browser program");
        };
        let compiled = program.compile("document").unwrap();

        // Assert
        assert_eq!(
            compiled,
            r#"[...document.querySelectorAll("[data-test=\"a\\\"b\\\\c\\a z\"]")]"#
        );
        drop((lease, query));
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[cfg(feature = "light-selectors")]
    #[test]
    fn light_id_engine_rejects_nul_without_retaining_the_recipe() {
        // Arrange
        let budget = budget();
        let source = "id:light=a\0b";
        let draft = budget.prepare_leaf(source.len()).unwrap();

        // Act
        let result = draft.css(source.to_owned());

        // Assert
        assert!(matches!(result, Err(HostError::InvalidArgument(_))));
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[cfg(not(feature = "light-selectors"))]
    #[test]
    fn light_attribute_engine_is_not_enabled_by_default() {
        // Arrange
        let budget = budget();
        let query = css(&budget, "id:light=target");

        // Act
        let (selector, lease) = query.command_selector().unwrap();

        // Assert
        assert!(matches!(selector, Selector::Css(ref css) if css.as_str() == "id:light=target"));
        drop((lease, query));
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[cfg(not(feature = "light-selectors"))]
    #[test]
    fn data_test_id_light_alias_is_not_enabled_by_default() {
        // Arrange
        let budget = budget();
        let query = css(&budget, "data-test-id:light=foo");

        // Act
        let (selector, lease) = query.command_selector().unwrap();

        // Assert
        assert!(
            matches!(selector, Selector::Css(ref css) if css.as_str() == "data-test-id:light=foo")
        );
        drop((lease, query));
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[cfg(not(feature = "light-selectors"))]
    #[test]
    fn data_test_light_alias_is_not_enabled_by_default() {
        // Arrange
        let budget = budget();
        let query = css(&budget, "data-test:light=foo");

        // Act
        let (selector, lease) = query.command_selector().unwrap();

        // Assert
        assert!(
            matches!(selector, Selector::Css(ref css) if css.as_str() == "data-test:light=foo")
        );
        drop((lease, query));
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[test]
    fn id_engine_escapes_quotes_backslashes_and_control_characters_as_css() {
        // Arrange
        let budget = budget();
        let quoted = css(&budget, r#"id=a"b\c"#);
        let control = css(&budget, "id=line\nnext");

        // Act
        let (quoted_selector, quoted_lease) = quoted.command_selector().unwrap();
        let (control_selector, control_lease) = control.command_selector().unwrap();

        // Assert
        assert!(matches!(
            quoted_selector,
            Selector::Css(ref css) if css.as_str() == r#"[id="a\"b\\c"]"#
        ));
        assert!(matches!(
            control_selector,
            Selector::Css(ref css) if css.as_str() == "[id=\"line\\a next\"]"
        ));
        drop((quoted_lease, control_lease));
    }

    #[test]
    fn id_engine_remains_a_typed_descendant_in_a_playwright_chain() {
        // Arrange
        let budget = budget();
        let query = css(&budget, "section >> id=one");

        // Act
        let (selector, lease) = query.command_selector().unwrap();

        // Assert
        assert!(matches!(
            selector,
            Selector::Within { base, descendant }
                if matches!(*base, Selector::Css(ref css) if css.as_str() == "section")
                    && matches!(*descendant, Selector::Css(ref css) if css.as_str() == r#"[id="one"]"#)
        ));
        assert_eq!(query.0.shape.nodes, 3);
        drop(lease);
    }

    #[test]
    fn id_engine_composes_with_an_existing_typed_locator() {
        // Arrange
        let budget = budget();
        let parent = css(&budget, "section");
        let child = css(&budget, "id=one");

        // Act
        let combined = parent.combine(&child, QueryCombination::Within).unwrap();

        // Assert
        assert!(matches!(
            &combined.0.plan,
            Plan::Selector(Selector::Within { base, descendant })
                if matches!(&**base, Selector::Css(css) if css.as_str() == "section")
                    && matches!(&**descendant, Selector::Css(css) if css.as_str() == r#"[id="one"]"#)
        ));
        assert_eq!(budget.metrics().unwrap().recipes, 3);
    }

    #[test]
    fn id_engine_admits_escaped_storage_before_releasing_the_input_draft() {
        // Arrange
        let source = "id=one";
        let input = Shape {
            depth: 1,
            nodes: 1,
            strings: source.len(),
        }
        .storage_bytes()
        .unwrap();
        let retained = Shape {
            depth: 1,
            nodes: 1,
            strings: r#"[id="one"]"#.len(),
        }
        .storage_bytes()
        .unwrap();
        let budget = QueryBudget::new(QueryLimits {
            recipes: 2,
            bytes: input + retained - 1,
        })
        .unwrap();
        let draft = budget.prepare_leaf(source.len()).unwrap();

        // Act
        let result = draft.css(source.to_owned());

        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[test]
    fn id_engine_rejects_expansion_past_selector_string_bound_without_a_leak() {
        // Arrange
        let budget = budget();
        let source = format!("id={}", "x".repeat(MAX_SELECTOR_STRING_BYTES - 6));
        let draft = budget.prepare_leaf(source.len()).unwrap();

        // Act
        let result = draft.css(source);

        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[test]
    fn id_engine_rejects_a_nul_it_cannot_match_exactly() {
        // Arrange
        let budget = budget();
        let draft = budget.prepare_leaf("id=a\0b".len()).unwrap();

        // Act
        let result = draft.css("id=a\0b".to_owned());

        // Assert
        assert!(matches!(result, Err(HostError::InvalidArgument(_))));
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[test]
    fn data_attribute_engines_match_complete_values_as_bounded_css() {
        // Arrange
        let budget = budget();
        let cases = [
            ("data-test=foo", r#"[data-test="foo"]"#),
            ("data-testid=foo", r#"[data-testid="foo"]"#),
            ("data-test-id=foo", r#"[data-test-id="foo"]"#),
        ];

        for (source, expected) in cases {
            // Act
            let query = css(&budget, source);
            let (selector, lease) = query.command_selector().unwrap();

            // Assert
            assert!(matches!(
                selector,
                Selector::Css(ref value) if value.as_str() == expected
            ));
            assert!(query.0.shape.strings >= expected.len());
            drop((lease, query));
        }
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[test]
    fn data_attribute_engine_escapes_quotes_backslashes_and_controls() {
        // Arrange
        let budget = budget();
        let query = css(&budget, "data-test=a\"b\\c\nnext");

        // Act
        let (selector, lease) = query.command_selector().unwrap();

        // Assert
        assert!(matches!(
            selector,
            Selector::Css(ref value) if value.as_str() == "[data-test=\"a\\\"b\\\\c\\a next\"]"
        ));
        drop((lease, query));
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[test]
    fn data_attribute_engine_does_not_interpret_css_filter_text_in_its_value() {
        // Arrange
        let budget = budget();
        let query = css(&budget, r#"data-test=foo:has-text("bar")"#);

        // Act
        let (selector, lease) = query.command_selector().unwrap();

        // Assert
        assert!(matches!(
            selector,
            Selector::Css(ref value)
                if value.as_str() == r#"[data-test="foo:has-text(\"bar\")"]"#
        ));
        drop((lease, query));
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[test]
    fn spaced_css_engine_composes_in_a_playwright_chain() {
        // Arrange
        let budget = budget();
        let query = css(&budget, " css = div >>css=div>>css   = span  ");

        // Act
        let (selector, lease) = query.command_selector().unwrap();
        let Selector::Within { base, descendant } = selector else {
            panic!("third CSS part must remain a typed descendant");
        };
        let Selector::Within {
            base: first,
            descendant: middle,
        } = *base
        else {
            panic!("second CSS part must remain a typed descendant");
        };

        // Assert
        assert!(matches!(*first, Selector::Css(ref value) if value.as_str() == " div"));
        assert!(matches!(*middle, Selector::Css(ref value) if value.as_str() == "div"));
        assert!(matches!(*descendant, Selector::Css(ref value) if value.as_str() == " span"));
        assert_eq!(query.0.shape.nodes, 5);
        drop((lease, query));
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[test]
    fn data_attribute_engine_admits_expanded_storage_before_releasing_input() {
        // Arrange
        let source = "data-testid=one";
        let input = Shape {
            depth: 1,
            nodes: 1,
            strings: source.len(),
        }
        .storage_bytes()
        .unwrap();
        let retained = Shape {
            depth: 1,
            nodes: 1,
            strings: r#"[data-testid="one"]"#.len(),
        }
        .storage_bytes()
        .unwrap();
        let budget = QueryBudget::new(QueryLimits {
            recipes: 2,
            bytes: input + retained - 1,
        })
        .unwrap();
        let draft = budget.prepare_leaf(source.len()).unwrap();

        // Act
        let result = draft.css(source.to_owned());

        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[test]
    fn data_attribute_engine_rejects_overlong_expansion_and_nul_without_leaks() {
        // Arrange
        let budget = budget();
        let source = format!(
            "data-test={}",
            "x".repeat(MAX_SELECTOR_STRING_BYTES - "data-test=".len())
        );
        let draft = budget.prepare_leaf(source.len()).unwrap();
        let nul = "data-test=a\0b";
        let nul_draft = budget.prepare_leaf(nul.len()).unwrap();

        // Act
        let overlong = draft.css(source);
        let unrepresentable = nul_draft.css(nul.to_owned());

        // Assert
        assert!(matches!(overlong, Err(HostError::ResourceExhausted(_))));
        assert!(matches!(
            unrepresentable,
            Err(HostError::InvalidArgument(_))
        ));
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[test]
    fn nonstandard_spaced_engine_remains_an_unknown_engine() {
        // Arrange
        let budget = budget();
        let query = css(&budget, "cssish = div");

        // Act
        let result = query.command_selector();

        // Assert
        assert!(matches!(
            query.deferred_issue(),
            Some(QueryIssue::UnknownEngine { .. })
        ));
        assert!(matches!(result, Err(HostError::Backend(_))));
    }

    #[test]
    fn nth_after_capture_is_a_lazy_playwright_shaped_issue() {
        // Arrange
        let budget = budget();
        let captured = css(&budget, "*css=div >> p");

        // Act
        let nth = captured.nth(1).unwrap();
        let error = match nth.command_selector() {
            Ok(_) => panic!("captured nth must remain a deferred query issue"),
            Err(error) => error,
        };

        // Assert
        assert_eq!(nth.deferred_issue(), Some(QueryIssue::NthAfterCapture));
        assert!(error.to_string().contains("Can't query n-th element"));
    }

    #[test]
    fn capture_compiles_the_matching_prefix_for_first_and_all_commands() {
        // Arrange
        let budget = budget();
        let sources = [
            "*css=div >> \"b\"",
            "section >> *css=div >> \"b\"",
            "css=div >> *text=\"b\"",
        ];

        // Act
        let selectors = sources
            .into_iter()
            .map(|source| css(&budget, source).command_selector().unwrap().0)
            .collect::<Vec<_>>();

        // Assert
        assert!(matches!(selectors[0], Selector::FilterHas { .. }));
        assert!(matches!(selectors[1], Selector::FilterHas { .. }));
        assert!(matches!(selectors[2], Selector::Within { .. }));
        let first = compile_selector(&selectors[0], "document").unwrap();
        let scoped = compile_selector(&selectors[1], "document").unwrap();
        let terminal = compile_selector(&selectors[2], "document").unwrap();
        assert!(first.contains("__pw.css(document, \"div\")"));
        assert!(first.contains(".filter(__candidate =>"));
        assert!(first.contains("__pw.textLegacy(__candidate, \"b\")"));
        assert!(scoped.contains("__pw.within(__pw.css(document, \"section\")"));
        assert!(scoped.contains(".filter(__candidate =>"));
        assert!(terminal.contains("__pw.textLegacy(__root, \"b\")"));
        assert!(!terminal.contains(".filter(__candidate =>"));
    }

    #[test]
    fn captured_text_body_keeps_embedded_chain_delimiters_quoted() {
        // Arrange
        let budget = budget();

        // Act
        let query = css(&budget, "css=div >> *text=\"a >> b\"");
        let (selector, _lease) = query.command_selector().unwrap();

        // Assert
        assert_eq!(
            playwright_selector_parts("css=div >> *text=\"a >> b\"").count(),
            2
        );
        assert!(matches!(selector, Selector::Within { .. }));
        assert!(
            compile_selector(&selector, "document")
                .unwrap()
                .contains("__pw.textLegacy(__root, \"a >> b\")")
        );
    }

    #[test]
    fn captured_text_with_unquoted_prefix_tracks_the_next_quote_like_upstream() {
        // Arrange
        let source = "*text=foo\" >> \"bar";

        // Act
        let parts = playwright_selector_parts(source).collect::<Vec<_>>();

        // Assert
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].value, source);
    }

    #[test]
    fn capture_visible_suffix_filters_the_returned_element_before_descendants() {
        // Arrange
        let budget = budget();
        let visible = css(&budget, "*css=div >> visible=true");
        let descendant = css(&budget, "*css=div >> visible=true >> span");

        // Act
        let (visible_selector, _visible_lease) = visible.command_selector().unwrap();
        let (descendant_selector, _descendant_lease) = descendant.command_selector().unwrap();

        // Assert
        assert!(matches!(
            visible_selector,
            Selector::FilterVisible { base, visible: true }
                if matches!(*base, Selector::Css(ref css) if css.as_str() == "div")
        ));
        assert!(matches!(
            descendant_selector,
            Selector::FilterHas { base, descendant }
                if matches!(*base, Selector::FilterVisible { visible: true, .. })
                    && matches!(*descendant, Selector::Css(ref css) if css.as_str() == "span")
        ));
    }

    #[test]
    fn capture_compiles_an_ancestor_filter_for_descendant_matches() {
        // Arrange
        let budget = budget();

        // Act
        let query = css(&budget, "*css=div >> span");
        let (selector, _lease) = query.command_selector().unwrap();
        let compiled = compile_selector(&selector, "document").unwrap();

        // Assert
        assert!(matches!(query.0.plan, Plan::Captured(_)));
        assert!(matches!(selector, Selector::FilterHas { .. }));
        assert!(compiled.starts_with("(__pw.css(document, \"div\")).filter(__candidate =>"));
        assert!(compiled.contains("__pw.css(__candidate, \"span\")"));
        assert!(!compiled.contains("__pw.within("));
    }

    #[test]
    fn captured_filter_text_applies_to_the_uncaptured_suffix() {
        // Arrange
        let budget = budget();
        let captured = css(&budget, "*css=div >> span");

        // Act
        let filtered = captured.filter_text("needle", false).unwrap();
        let (selector, _lease) = filtered.command_selector().unwrap();

        // Assert
        let Selector::FilterHas { base, descendant } = selector else {
            panic!("capture must return the ancestor filtered by its suffix");
        };
        assert!(matches!(*base, Selector::Css(_)));
        assert!(matches!(
            *descendant,
            Selector::FilterText { base, text, .. }
                if matches!(*base, Selector::Css(_)) && text == "needle"
        ));
    }

    #[test]
    fn captured_descendant_locator_extends_the_suffix_not_the_returned_ancestor() {
        // Arrange
        let budget = budget();
        let captured = css(&budget, "*css=div >> span");
        let descendant = css(&budget, "p");

        // Act
        let combined = captured
            .combine(&descendant, QueryCombination::Within)
            .unwrap();
        let (selector, _lease) = combined.command_selector().unwrap();

        // Assert
        let Selector::FilterHas { base, descendant } = selector else {
            panic!("capture must return the ancestor filtered by its suffix");
        };
        assert!(matches!(*base, Selector::Css(_)));
        assert!(matches!(*descendant, Selector::Within { .. }));
    }

    #[test]
    fn captured_has_filter_applies_inside_the_uncaptured_suffix() {
        // Arrange
        let budget = budget();
        let captured = css(&budget, "*css=div >> span");
        let nested = css(&budget, "button");

        // Act
        let combined = captured.combine(&nested, QueryCombination::Has).unwrap();
        let (selector, _lease) = combined.command_selector().unwrap();

        // Assert
        let Selector::FilterHas { base, descendant } = selector else {
            panic!("capture must return the ancestor filtered by its suffix");
        };
        assert!(matches!(*base, Selector::Css(_)));
        assert!(matches!(*descendant, Selector::FilterHas { .. }));
    }

    #[test]
    fn ancestor_before_capture_remains_in_the_returned_prefix() {
        // Arrange
        let budget = budget();
        let ancestor = css(&budget, "section");
        let captured = css(&budget, "*css=div >> span");

        // Act
        let combined = ancestor
            .combine(&captured, QueryCombination::Within)
            .unwrap();
        let (selector, _lease) = combined.command_selector().unwrap();

        // Assert
        let Selector::FilterHas { base, descendant } = selector else {
            panic!("capture must retain its pre-capture ancestor chain");
        };
        assert!(matches!(*base, Selector::Within { .. }));
        assert!(matches!(*descendant, Selector::Css(_)));
    }

    #[test]
    fn nested_capture_composition_fails_closed_without_returning_the_wrong_element() {
        // Arrange
        let budget = budget();
        let ancestor = css(&budget, "section");
        let captured = css(&budget, "*css=div >> span");

        // Act
        let combined = ancestor.combine(&captured, QueryCombination::Has).unwrap();
        let error = combined.command_selector().err().unwrap();

        // Assert
        assert_eq!(
            combined.deferred_issue(),
            Some(QueryIssue::CapturedSelector)
        );
        assert!(matches!(error, HostError::UnsupportedOperation(_)));
    }

    #[test]
    fn plain_star_remains_css_and_a_descendant_wildcard() {
        // Arrange
        let budget = budget();

        // Act
        let star = css(&budget, "*");
        let descendants = css(&budget, "section >> *");

        // Assert
        assert!(matches!(star.0.plan, Plan::Selector(Selector::Css(_))));
        assert!(matches!(
            descendants.0.plan,
            Plan::Selector(Selector::Within { .. })
        ));
        let (selector, _lease) = descendants.command_selector().unwrap();
        assert!(
            compile_selector(&selector, "document")
                .unwrap()
                .contains("__pw.css(__root, \"*\")")
        );
    }

    #[test]
    fn multiple_capture_markers_report_the_upstream_parser_error() {
        // Arrange
        let budget = budget();
        let query = css(&budget, "*css=div >> *css=span");

        // Act
        let error = query.command_selector().err().unwrap();

        // Assert
        assert_eq!(query.deferred_issue(), Some(QueryIssue::MultipleCaptures));
        assert!(
            error
                .to_string()
                .contains("Only one of the selectors can capture using * modifier")
        );
    }

    #[test]
    fn malformed_capture_reports_the_empty_engine_name() {
        // Arrange
        let budget = budget();
        let query = css(&budget, "*=div");

        // Act
        let error = query.command_selector().err().unwrap();

        // Assert
        assert!(matches!(
            query.deferred_issue(),
            Some(QueryIssue::UnknownEngine { .. })
        ));
        assert_eq!(
            error.to_string(),
            "Unknown engine \"\" while parsing selector *=div"
        );
    }

    #[test]
    fn capture_rejects_nth_inside_the_selector_and_on_locator_composition() {
        // Arrange
        let budget = budget();
        let inline = css(&budget, "*css=div >> nth=0");
        let composed = css(&budget, "*css=div >> span");

        // Act
        let inline_error = inline.command_selector().err().unwrap();
        let nth = composed.nth(0).unwrap();
        let composed_error = nth.command_selector().err().unwrap();

        // Assert
        let expected = "Can't query n-th element in a request with the capture.";
        assert!(inline_error.to_string().contains(expected));
        assert!(composed_error.to_string().contains(expected));
    }

    #[cfg(feature = "selector-registration")]
    #[test]
    fn registered_parser_keeps_builtin_capture_and_rejects_custom_capture() {
        // Arrange
        let budget = budget();
        let registry = crate::selector_registry::SelectorRegistry::default();
        registry
            .register(
                "tag",
                "({ query(root, selector) { return root.querySelector(selector); }, queryAll(root, selector) { return root.querySelectorAll(selector); } })",
                false,
            )
            .unwrap();
        let builtin = "*css=div >> span";
        let custom = "*tag=div";

        // Act
        let supported = budget
            .prepare_leaf(builtin.len())
            .unwrap()
            .css_registered(builtin.to_owned(), &registry)
            .unwrap();
        let unsupported = budget
            .prepare_leaf(custom.len())
            .unwrap()
            .css_registered(custom.to_owned(), &registry)
            .unwrap();

        // Assert
        assert!(matches!(supported.0.plan, Plan::Captured(_)));
        assert!(matches!(
            unsupported.deferred_issue(),
            Some(QueryIssue::UnknownEngine { .. })
        ));
    }

    #[cfg(feature = "selector-registration")]
    #[test]
    fn registered_first_retains_one_browser_query_and_budgeted_recipe() {
        // Arrange
        let budget = budget();
        let registry = crate::selector_registry::SelectorRegistry::default();
        registry
            .register(
                "tag",
                "({ query(root, selector) { return root.querySelector(selector); }, queryAll(root, selector) { return root.querySelectorAll(selector); } })",
                false,
            )
            .unwrap();
        let original = budget
            .prepare_leaf("tag=button".len())
            .unwrap()
            .css_registered("tag=button".to_owned(), &registry)
            .unwrap();
        let original_usage = budget.metrics().unwrap();

        // Act
        let first = original.nth(0).unwrap();
        let (plan, lease) = first.command_plan().unwrap();
        let CommandPlan::Registered(program) = plan else {
            panic!("first match must retain the registered selector plan");
        };
        let source = program.compile("document", false).unwrap();

        // Assert
        assert_eq!(source.matches("__pwRegisteredQuery(__pwEngine0").count(), 1);
        assert!(source.contains("\"button\", true)"));
        assert!(source.contains("let index = 0"));
        assert_eq!(budget.metrics().unwrap().recipes, 3);
        drop(lease);
        drop(first);
        assert_eq!(budget.metrics().unwrap(), original_usage);
        drop(original);
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[cfg(feature = "selector-registration")]
    #[test]
    fn registered_first_rejects_depth_overflow_without_reserving_a_recipe() {
        // Arrange
        let budget = budget();
        let registry = crate::selector_registry::SelectorRegistry::default();
        registry
            .register(
                "tag",
                "({ queryAll(root) { return root.querySelectorAll('div'); } })",
                false,
            )
            .unwrap();
        let source = std::iter::once("tag=div")
            .chain(std::iter::repeat_n("css=div", MAX_SELECTOR_DEPTH - 1))
            .collect::<Vec<_>>()
            .join(" >> ");
        let original = budget
            .prepare_leaf(source.len())
            .unwrap()
            .css_registered(source, &registry)
            .unwrap();
        let original_usage = budget.metrics().unwrap();

        // Act
        let result = original.nth(0);

        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        assert_eq!(budget.metrics().unwrap(), original_usage);
        drop(original);
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[cfg(feature = "selector-registration")]
    #[test]
    fn registered_nonfirst_nth_stays_outside_the_new_capability() {
        // Arrange
        let budget = budget();
        let registry = crate::selector_registry::SelectorRegistry::default();
        registry
            .register("tag", "({ query() { return null; } })", false)
            .unwrap();
        let original = budget
            .prepare_leaf("tag=div".len())
            .unwrap()
            .css_registered("tag=div".to_owned(), &registry)
            .unwrap();
        let original_usage = budget.metrics().unwrap();

        // Act
        let result = original.nth(1);

        // Assert
        assert!(matches!(result, Err(HostError::UnsupportedOperation(_))));
        assert_eq!(budget.metrics().unwrap(), original_usage);
    }

    #[test]
    fn builtin_playwright_selector_engine_is_not_rejected_as_unknown() {
        // Arrange
        let budget = budget();
        let query = css(&budget, "text=ready");

        // Act
        let result = query.command_selector();

        // Assert
        assert!(matches!(
            result,
            Ok((Selector::Text(matcher), _)) if !matcher.exact() && matcher.value() == "ready"
        ));
        assert_eq!(query.deferred_issue(), None);
    }

    #[cfg(feature = "element-handle-dom")]
    #[test]
    fn element_handle_xpath_shorthand_stays_a_tagged_xpath_program() {
        // Arrange
        let budget = budget();
        let source = "//article";
        let query = budget
            .prepare_leaf(source.len())
            .unwrap()
            .element_handle(source.to_owned())
            .unwrap();

        // Act
        let (plan, lease) = query.command().unwrap();

        // Assert
        assert!(matches!(
            plan,
            ElementHandleQueryPlan::XPath(expression) if expression == source
        ));
        drop(lease);
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[cfg(feature = "element-handle-dom")]
    #[test]
    fn empty_element_handle_xpath_releases_its_admission() {
        // Arrange
        let budget = budget();
        let source = "xpath=";
        let draft = budget.prepare_leaf(source.len()).unwrap();

        // Act
        let result = draft.element_handle(source.to_owned());

        // Assert
        assert!(matches!(result, Err(HostError::InvalidArgument(_))));
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[test]
    fn builtin_playwright_text_regex_is_a_typed_regex_leaf() {
        // Arrange
        let budget = budget();

        // Act
        let insensitive = css(&budget, r"text=/Ya/i");
        let escaped_delimiter = css(&budget, r"text=/a\/b/m");

        // Assert
        assert!(matches!(
            insensitive.command_selector(),
            Ok((Selector::TextRegex(matcher), _))
                if matcher.source() == "Ya" && matcher.flags() == "i"
        ));
        assert!(matches!(
            escaped_delimiter.command_selector(),
            Ok((Selector::TextRegex(matcher), _))
                if matcher.source() == r"a\/b" && matcher.flags() == "m"
        ));
    }

    #[test]
    fn builtin_playwright_text_regex_rejects_missing_delimiter_and_invalid_flags() {
        // Arrange
        let budget = budget();

        // Act
        let missing_delimiter = css(&budget, "text=/unterminated");
        let duplicate_flags = css(&budget, "text=/ready/ii");

        // Assert
        assert!(missing_delimiter.command_selector().is_err());
        assert!(duplicate_flags.command_selector().is_err());
    }

    #[test]
    fn simple_relative_xpath_is_one_typed_child_selector() {
        // Arrange
        let budget = budget();
        let query = css(&budget, "xpath=./span");

        // Act
        let (selector, _lease) = query.command_selector().unwrap();

        // Assert
        assert_eq!(selector.to_string(), ":scope>span");
    }

    #[cfg(feature = "handles")]
    #[test]
    fn full_xpath_is_retained_as_a_browser_program() {
        // Arrange
        let budget = budget();
        let query = css(&budget, "xpath=//article");

        // Act
        let (plan, _lease) = query.command_plan().unwrap();

        // Assert
        assert!(matches!(
            plan,
            CommandPlan::Browser(program)
                if program.compile("document").unwrap() == "__pw.xpath(document, \"//article\")"
        ));
        assert!(query.command_selector().is_err());
    }

    #[cfg(feature = "handles")]
    #[test]
    fn typed_parent_composes_with_xpath_child_in_one_browser_program() {
        // Arrange
        let budget = budget();
        let parent = css(&budget, "html");
        let child = css(&budget, "xpath=./body/div[contains(@class, 'second')]");

        // Act
        let query = parent.combine(&child, QueryCombination::Within).unwrap();
        let (plan, _lease) = query.command_plan().unwrap();
        let CommandPlan::Browser(program) = plan else {
            panic!("typed parent plus XPath child must use a browser program");
        };
        let compiled = program.compile("document").unwrap();

        // Assert
        assert!(compiled.starts_with("__pw.within(__pw.css(document, \"html\")"));
        assert!(compiled.contains(
            "__pwRoot => __pw.xpath(__pwRoot, \"./body/div[contains(@class, 'second')]\")"
        ));
    }

    #[cfg(feature = "handles")]
    #[test]
    fn nested_xpath_descendants_flatten_into_one_bounded_browser_program() {
        // Arrange
        let budget = budget();
        let html = css(&budget, "html");
        let second = css(&budget, "xpath=./body/div[contains(@class, 'second')]");
        let inner = css(&budget, "xpath=./div[contains(@class, 'inner')]");

        // Act
        let query = html
            .combine(&second, QueryCombination::Within)
            .unwrap()
            .combine(&inner, QueryCombination::Within)
            .unwrap();
        let (plan, _lease) = query.command_plan().unwrap();
        let CommandPlan::Browser(program) = plan else {
            panic!("nested XPath descendants must use a browser program");
        };
        let compiled = program.compile("document").unwrap();

        // Assert
        assert_eq!(program.0.len(), 3);
        assert_eq!(compiled.matches("__pw.within(").count(), 2);
        assert!(compiled.contains("./div[contains(@class, 'inner')]"));
        assert!(program.retained_string_capacity().unwrap() <= query.0.shape.strings);
        assert!(program.0.len() <= query.0.shape.nodes);
    }

    #[test]
    fn non_descendant_browser_composition_rejects_without_budget_growth() {
        // Arrange
        let budget = budget();
        let xpath = css(&budget, "xpath=//article");
        let typed = css(&budget, "article");
        let before = budget.metrics().unwrap();

        // Act
        let result = xpath.combine(&typed, QueryCombination::And);

        // Assert
        assert!(matches!(result, Err(HostError::UnsupportedOperation(_))));
        assert_eq!(budget.metrics().unwrap(), before);
    }

    #[test]
    fn browser_selector_compiler_supports_all_xpath_shorthands() {
        // Arrange
        let selectors = ["xpath=//article", "//article", "(//article)[1]"];

        // Act
        let compiled =
            selectors.map(|selector| compile_browser_selector(selector, "document").unwrap());

        // Assert
        assert_eq!(compiled[0], "__pw.xpath(document, \"//article\")");
        assert_eq!(compiled[1], "__pw.xpath(document, \"//article\")");
        assert_eq!(compiled[2], "__pw.xpath(document, \"(//article)[1]\")");
    }

    #[test]
    fn browser_selector_compiler_composes_text_with_relative_xpath() {
        // Arrange
        let source = r#""test" >> ../span"#;

        // Act
        let compiled = compile_browser_selector(source, "document").unwrap();

        // Assert
        assert_eq!(
            compiled,
            "__pw.within(__pw.textLegacy(document, \"test\"), __pwRoot => __pw.xpath(__pwRoot, \"../span\"))"
        );
    }

    #[test]
    fn browser_selector_compiler_applies_nth_after_xpath() {
        // Arrange
        let source = "xpath=//option >> nth=-1";

        // Act
        let compiled = compile_browser_selector(source, "document").unwrap();

        // Assert
        assert!(compiled.contains("const values = __pw.xpath(document, \"//option\")"));
        assert!(compiled.contains("let index = -1"));
        assert!(compiled.contains("index += values.length"));
    }

    #[test]
    fn internal_text_selector_preserves_json_text_and_match_mode() {
        // Arrange
        let budget = budget();

        // Act
        let substring = css(&budget, r#"internal:text="a\n\"b"i"#);
        let exact = css(&budget, r#"internal:text="save"s"#);

        // Assert
        assert!(matches!(
            substring.command_selector(),
            Ok((Selector::Text(matcher), _))
                if !matcher.exact() && matcher.value() == "a\n\"b"
        ));
        assert!(matches!(
            exact.command_selector(),
            Ok((Selector::Text(matcher), _)) if matcher.exact() && matcher.value() == "save"
        ));
    }

    #[test]
    fn playwright_css_text_pseudo_is_a_typed_exact_text_leaf() {
        // Arrange
        let budget = budget();

        // Act
        let query = css(&budget, r#":text("button1")"#);

        // Assert
        assert!(matches!(
            query.command_selector(),
            Ok((Selector::Text(matcher), _))
                if matcher.exact() && matcher.value() == "button1"
        ));
    }

    #[test]
    fn terminal_css_has_text_pseudo_is_a_typed_substring_filter() {
        // Arrange
        let budget = budget();

        // Act
        let query = css(&budget, r#"button:has-text("Hello nested iframe")"#);

        // Assert
        assert!(matches!(
            query.command_selector(),
            Ok((Selector::FilterText { base, text, exact }, _))
                if matches!(*base, Selector::Css(_))
                    && text == "Hello nested iframe"
                    && !exact
        ));
    }

    #[test]
    fn quoted_playwright_selector_is_a_legacy_immediate_text_leaf() {
        // Arrange
        let budget = budget();
        let query = css(&budget, r#""Click \"me\"""#);

        // Act
        let (selector, lease) = query.command_selector().unwrap();

        // Assert
        assert!(matches!(
            selector,
            Selector::LegacyText(matcher)
                if matcher.exact() && matcher.value() == "Click \"me\""
        ));
        drop(lease);
        drop(query);
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[test]
    fn quoted_text_keeps_embedded_combinators_inside_one_legacy_leaf() {
        // Arrange
        let budget = budget();
        let query = css(&budget, r#"text="Hi''>>foo=bar""#);

        // Act
        let (selector, lease) = query.command_selector().unwrap();

        // Assert
        assert!(matches!(
            selector,
            Selector::LegacyText(matcher)
                if matcher.exact() && matcher.value() == "Hi''>>foo=bar"
        ));
        assert_eq!(query.deferred_issue(), None);
        drop(lease);
    }

    #[test]
    fn escaped_regex_combinator_still_allows_a_real_following_combinator() {
        // Arrange
        let budget = budget();
        let query = css(&budget, r"text=/Hi\>\>/ >> span");

        // Act
        let (selector, lease) = query.command_selector().unwrap();

        // Assert
        assert!(matches!(
            selector,
            Selector::Within { base, descendant }
                if matches!(
                    *base,
                    Selector::TextRegex(ref matcher)
                        if matcher.source() == r"Hi\>\>" && matcher.flags().is_empty()
                ) && matches!(*descendant, Selector::Css(ref css) if css.as_str() == "span")
        ));
        drop(lease);
    }

    #[test]
    fn internal_and_intersects_the_current_match_set() {
        // Arrange
        let budget = budget();
        let query = css(&budget, r#"div >> internal:and=".foo""#);

        // Act
        let (selector, lease) = query.command_selector().unwrap();

        // Assert
        assert!(matches!(
            selector,
            Selector::And { left, right }
                if matches!(*left, Selector::Css(ref css) if css.as_str() == "div")
                    && matches!(*right, Selector::Css(ref css) if css.as_str() == ".foo")
        ));
        drop(lease);
        drop(query);
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[test]
    fn internal_and_requires_one_json_encoded_nested_leaf() {
        // Arrange
        let budget = budget();
        let malformed = css(&budget, r#"div >> internal:and=.foo"#);
        let nested_chain = css(&budget, r#"div >> internal:and="span >> .foo""#);

        // Act
        let malformed_result = malformed.command_selector();
        let nested_result = nested_chain.command_selector();

        // Assert
        assert!(matches!(malformed_result, Err(HostError::Backend(_))));
        assert!(matches!(nested_result, Err(HostError::Backend(_))));
    }

    #[test]
    fn playwright_visible_parts_filter_the_current_recipe_before_chaining() {
        // Arrange
        let budget = budget();

        // Act
        let visible = css(&budget, ".item >> visible=true >> text=data3");
        let hidden = css(&budget, ".item >> visible=false");

        // Assert
        assert!(matches!(
            visible.command_selector(),
            Ok((Selector::Within { base, descendant }, _))
                if matches!(
                    &*base,
                    Selector::FilterVisible { base, visible: true }
                        if matches!(&**base, Selector::Css(css) if css.as_str() == ".item")
                ) && matches!(
                    &*descendant,
                    Selector::Text(matcher)
                        if !matcher.exact() && matcher.value() == "data3"
                )
        ));
        assert!(matches!(
            hidden.command_selector(),
            Ok((Selector::FilterVisible { base, visible: false }, _))
                if matches!(&*base, Selector::Css(css) if css.as_str() == ".item")
        ));
    }

    #[test]
    fn playwright_visible_parts_reject_invalid_values_and_initial_use() {
        // Arrange
        let budget = budget();
        let invalid = css(&budget, ".item >> visible=maybe");
        let initial = css(&budget, "visible=true");

        // Act
        let invalid_result = invalid.command_selector();
        let initial_result = initial.command_selector();

        // Assert
        assert!(matches!(invalid_result, Err(HostError::Backend(_))));
        assert!(matches!(initial_result, Err(HostError::Backend(_))));
    }

    #[test]
    fn css_visible_pseudo_is_a_typed_visible_filter() {
        // Arrange
        let budget = budget();

        // Act
        let query = css(&budget, "css=section > div:visible");
        let (selector, lease) = query.command_selector().unwrap();

        // Assert
        assert!(matches!(
            selector,
            Selector::FilterVisible { base, visible: true }
                if matches!(*base, Selector::Css(ref css) if css.as_str() == "section > div")
        ));
        assert_eq!(query.0.shape.nodes, 2);
        assert_eq!(query.0.shape.depth, 2);
        drop((lease, query));
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[test]
    fn css_visible_pseudo_name_is_ascii_case_insensitive() {
        // Arrange
        let budget = budget();

        // Act
        let query = css(&budget, "css=section > DIV:ViSiBlE");
        let (selector, _lease) = query.command_selector().unwrap();

        // Assert
        assert!(matches!(
            selector,
            Selector::FilterVisible { base, visible: true }
                if matches!(*base, Selector::Css(ref css) if css.as_str() == "section > DIV")
        ));
    }

    #[test]
    fn css_nth_match_uses_one_based_index_after_typed_dom_order_union() {
        // Arrange
        let budget = budget();

        // Act
        let query = css(&budget, ":nth-match(div, section, 3)");
        let (selector, lease) = query.command_selector().unwrap();
        let compiled = compile_selector(&selector, "document").unwrap();

        // Assert
        assert!(matches!(
            selector,
            Selector::Nth { base, index: 2 }
                if matches!(*base, Selector::Or { .. })
        ));
        assert!(compiled.contains("__pw.union("));
        assert!(compiled.contains("let index = 2"));
        assert_eq!(query.0.shape.nodes, 4);
        assert_eq!(query.0.shape.depth, 3);
        drop((lease, query));
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[test]
    fn css_nth_match_pseudo_name_is_ascii_case_insensitive() {
        // Arrange
        let budget = budget();

        // Act
        let query = css(&budget, ":NtH-MaTcH(div, 2)");
        let (selector, _lease) = query.command_selector().unwrap();

        // Assert
        assert!(matches!(
            selector,
            Selector::Nth { base, index: 1 }
                if matches!(*base, Selector::Css(ref css) if css.as_str() == "div")
        ));
    }

    #[test]
    fn css_nth_match_accepts_integer_valued_css_numbers() {
        // Arrange
        let budget = budget();
        let sources = [
            (":nth-match(div, 1.0)", 0),
            (":nth-match(div, 1e0)", 0),
            (":nth-match(div, .10e1)", 0),
            (":nth-match(div, +2.0E+0)", 1),
            (":nth-match(div, 2147483647)", i32::MAX - 1),
        ];

        for (source, expected_index) in sources {
            // Act
            let query = css(&budget, source);
            let (selector, _lease) = query.command_selector().unwrap();

            // Assert
            assert!(matches!(
                selector,
                Selector::Nth { base, index }
                    if index == expected_index
                        && matches!(*base, Selector::Css(ref css) if css.as_str() == "div")
            ));
        }
    }

    #[test]
    fn css_nth_match_rejects_invalid_or_unbounded_css_numbers() {
        // Arrange
        let budget = budget();
        let sources = [
            ":nth-match(div, 0.0)",
            ":nth-match(div, 1.5)",
            ":nth-match(div, 1e-3)",
            ":nth-match(div, 1e)",
            ":nth-match(div, 1.)",
            ":nth-match(div, NaN)",
            ":nth-match(div, Infinity)",
            ":nth-match(div, 1e309)",
            ":nth-match(div, 2147483648)",
        ];

        for source in sources {
            // Act
            let query = css(&budget, source);
            let result = query.command_selector();

            // Assert
            assert_eq!(
                query.deferred_issue(),
                Some(QueryIssue::NthMatchInvalidIndex)
            );
            assert!(matches!(result, Err(HostError::Backend(_))));
        }
    }

    #[test]
    fn css_is_of_nth_match_branches_uses_typed_union() {
        // Arrange
        let budget = budget();

        // Act
        let query = css(&budget, ":is(:nth-match(div, 1), :nth-match(div, 2))");
        let (selector, lease) = query.command_selector().unwrap();
        let compiled = compile_selector(&selector, "document").unwrap();

        // Assert
        assert!(matches!(selector, Selector::Or { .. }));
        assert!(compiled.contains("__pw.union("));
        assert_eq!(compiled.matches("__pw.css(document, \"div\")").count(), 2);
        assert_eq!(query.0.shape.nodes, 5);
        assert_eq!(query.0.shape.depth, 3);
        drop((lease, query));
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[test]
    fn css_is_root_pseudo_name_is_ascii_case_insensitive() {
        // Arrange
        let budget = budget();

        // Act
        let query = css(&budget, ":IS(:NTH-MATCH(div, 1), :nth-match(div, 2))");
        let (selector, _lease) = query.command_selector().unwrap();

        // Assert
        assert!(matches!(selector, Selector::Or { .. }));
    }

    #[test]
    fn css_nth_match_arguments_respect_quotes_escapes_and_nested_parentheses() {
        // Arrange
        let budget = budget();
        let source = r#":nth-match(div[data-x="a,b"], section:not(.a,.b), #a\,b, 2)"#;

        // Act
        let query = css(&budget, source);
        let (selector, lease) = query.command_selector().unwrap();
        let compiled = compile_selector(&selector, "document").unwrap();

        // Assert
        assert!(matches!(selector, Selector::Nth { index: 1, .. }));
        assert!(compiled.contains(r#"div[data-x=\"a,b\"]"#));
        assert!(compiled.contains("section:not(.a,.b)"));
        assert!(compiled.contains(r"#a\\,b"));
        assert_eq!(query.0.shape.nodes, 6);
        drop((lease, query));
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[test]
    fn literal_or_escaped_pseudo_text_remains_native_css() {
        // Arrange
        let budget = budget();
        let quoted = r#"div[data-name=":visible, :nth-match(div, 1)"]"#;
        let escaped = r"#a\:visible";

        // Act
        let quoted_query = css(&budget, quoted);
        let escaped_query = css(&budget, escaped);

        // Assert
        assert!(matches!(
            quoted_query.command_selector(),
            Ok((Selector::Css(value), _)) if value.as_str() == quoted
        ));
        assert!(matches!(
            escaped_query.command_selector(),
            Ok((Selector::Css(value), _)) if value.as_str() == escaped
        ));
        assert_eq!(quoted_query.0.shape.nodes, 1);
        assert_eq!(escaped_query.0.shape.nodes, 1);
    }

    #[test]
    fn css_comment_pseudo_text_remains_native_css() {
        // Arrange
        let budget = budget();
        let source = "div/* :visible, :nth-match(div, 1) */";

        // Act
        let query = css(&budget, source);
        let (selector, _lease) = query.command_selector().unwrap();

        // Assert
        assert!(matches!(selector, Selector::Css(css) if css.as_str() == source));
        assert_eq!(query.0.shape.nodes, 1);
    }

    #[test]
    fn css_nth_match_comments_do_not_split_arguments_or_activate_pseudos() {
        // Arrange
        let budget = budget();
        let source = ":nth-match(div/*, :visible */, span, 2)";

        // Act
        let query = css(&budget, source);
        let (selector, _lease) = query.command_selector().unwrap();

        // Assert
        assert!(matches!(
            selector,
            Selector::Nth { base, index: 1 }
                if matches!(
                    &*base,
                    Selector::Or { left, right }
                        if matches!(&**left, Selector::Css(css) if css.as_str() == "div/*, :visible */")
                            && matches!(&**right, Selector::Css(css) if css.as_str() == "span")
                )
        ));
    }

    #[test]
    fn css_pseudo_composes_with_playwright_chain_and_admits_full_shape() {
        // Arrange
        let budget = budget();

        // Act
        let query = css(&budget, "section >> div:visible >> nth=0");
        let (selector, lease) = query.command_selector().unwrap();

        // Assert
        assert!(matches!(selector, Selector::Nth { index: 0, .. }));
        assert_eq!(query.0.shape.nodes, 5);
        assert_eq!(query.0.shape.depth, 4);
        drop((lease, query));
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[test]
    fn malformed_nth_match_preserves_official_error_messages() {
        // Arrange
        let budget = budget();
        let missing = css(&budget, ":nth-match(2)");
        let zero = css(&budget, ":nth-match(div, bar, 0)");
        let text_index = css(&budget, ":nth-match(div, bar, foo)");

        // Act
        let missing_error = missing.command_selector().err().unwrap();
        let zero_error = zero.command_selector().err().unwrap();
        let text_error = text_index.command_selector().err().unwrap();

        // Assert
        assert!(missing_error.to_string().contains(
            "\"nth-match\" engine expects non-empty selector list and an index argument"
        ));
        for error in [zero_error, text_error] {
            assert!(
                error.to_string().contains(
                    "\"nth-match\" engine expects a one-based index as the last argument"
                )
            );
        }
    }

    #[test]
    fn unsupported_nested_css_extensions_fail_closed() {
        // Arrange
        let budget = budget();
        let sources = [
            "div:not(:visible)",
            "div:visible > span",
            ":nth-match(div:visible, 2)",
            "div:visible:has-text('ready')",
        ];

        for source in sources {
            // Act
            let query = css(&budget, source);
            let result = query.command_selector();

            // Assert
            assert_eq!(
                query.deferred_issue(),
                Some(QueryIssue::UnsupportedCssExtension)
            );
            assert!(matches!(result, Err(HostError::UnsupportedOperation(_))));
        }
    }

    #[test]
    fn css_pseudo_final_recipe_must_fit_the_shared_budget() {
        // Arrange
        let source = "div:visible";
        let input_bytes = Shape {
            depth: 1,
            nodes: 1,
            strings: source.len(),
        }
        .storage_bytes()
        .unwrap();
        let retained_bytes = Shape {
            depth: 2,
            nodes: 2,
            strings: "div".len(),
        }
        .storage_bytes()
        .unwrap();
        let budget = QueryBudget::new(QueryLimits {
            recipes: 2,
            bytes: input_bytes + retained_bytes - 1,
        })
        .unwrap();
        let draft = budget.prepare_leaf(source.len()).unwrap();

        // Act
        let result = draft.css(source.to_owned());

        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[test]
    fn css_nth_match_node_limit_rejects_before_retaining_a_recipe() {
        // Arrange
        let budget = budget();
        let source = format!(":nth-match({}, 1)", vec!["a"; 129].join(", "));
        let draft = budget.prepare_leaf(source.len()).unwrap();

        // Act
        let result = draft.css(source);

        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[test]
    fn input_larger_than_its_reservation_fails_without_retention() {
        // Arrange
        let budget = budget();
        let draft = budget.prepare_leaf(1).unwrap();
        // Act
        let result = draft.css("button".to_owned());
        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[test]
    fn full_recipe_capacity_rejects_before_copying_a_second_input() {
        // Arrange
        let budget = QueryBudget::new(QueryLimits {
            recipes: 1,
            bytes: 1024 * 1024,
        })
        .unwrap();
        let _query = css(&budget, "button");
        let before = budget.metrics().unwrap();
        // Act
        let result = budget.prepare_leaf(1);
        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        assert_eq!(budget.metrics().unwrap(), before);
    }

    #[test]
    fn browser_composition_admits_before_cloning_program_storage() {
        // Arrange
        let budget = QueryBudget::new(QueryLimits {
            recipes: 2,
            bytes: 1024 * 1024,
        })
        .unwrap();
        let parent = css(&budget, "html");
        let child = css(&budget, "xpath=//article");
        let before = budget.metrics().unwrap();

        // Act
        let result = parent.combine(&child, QueryCombination::Within);

        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        assert_eq!(budget.metrics().unwrap(), before);
        assert!(matches!(parent.0.plan, Plan::Selector(_)));
        assert!(matches!(child.0.plan, Plan::Browser(_)));
    }

    #[test]
    fn browser_program_copy_owns_independently_admitted_metadata() {
        // Arrange
        let budget = budget();
        let query = css(&budget, "xpath=//article");

        // Act
        let copy = query.copy().unwrap();

        // Assert
        let (Plan::Browser(original), Plan::Browser(copied)) = (&query.0.plan, &copy.0.plan) else {
            panic!("full XPath must remain a browser program");
        };
        assert!(!std::ptr::eq(original.0.as_ptr(), copied.0.as_ptr()));
        assert_eq!(budget.metrics().unwrap().recipes, 2);
    }

    #[test]
    fn byte_capacity_includes_the_string_conversion_overlap() {
        // Arrange
        let shape = Shape {
            depth: 1,
            nodes: 1,
            strings: 100,
        };
        let bytes = shape.storage_bytes().unwrap();
        let budget = QueryBudget::new(QueryLimits { recipes: 10, bytes }).unwrap();
        let _query = css(&budget, &"x".repeat(100));
        // Act
        let result = budget.prepare_leaf(0);
        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        assert_eq!(budget.metrics().unwrap().bytes, bytes);
        assert!(bytes >= 200 + std::mem::size_of::<QueryInner>());
    }

    #[test]
    fn combination_preserves_both_original_recipes() {
        // Arrange
        let budget = budget();
        let left = css(&budget, "section");
        let right = css(&budget, "button");
        // Act
        let combined = left.combine(&right, QueryCombination::Within).unwrap();
        // Assert
        assert!(
            matches!(&left.0.plan, Plan::Selector(Selector::Css(value)) if value.as_str() == "section")
        );
        assert!(
            matches!(&right.0.plan, Plan::Selector(Selector::Css(value)) if value.as_str() == "button")
        );
        assert!(matches!(
            &combined.0.plan,
            Plan::Selector(Selector::Within { .. })
        ));
        assert_eq!(budget.metrics().unwrap().recipes, 3);
    }

    #[test]
    fn playwright_css_chain_is_one_bounded_within_recipe() {
        // Arrange
        let budget = budget();

        // Act
        let query = css(&budget, "div >> p");

        // Assert
        assert!(matches!(
            &query.0.plan,
            Plan::Selector(Selector::Within { .. })
        ));
        assert_eq!(query.0.shape.nodes, 3);
        assert_eq!(query.0.shape.depth, 2);
        assert_eq!(budget.metrics().unwrap().recipes, 1);
    }

    #[test]
    fn playwright_nth_part_filters_the_current_recipe() {
        // Arrange
        let budget = budget();

        // Act
        let query = css(&budget, "optgroup >> nth=0");
        let (selector, _lease) = query.command_selector().unwrap();

        // Assert
        assert!(matches!(
            selector,
            Selector::Nth { base, index: 0 }
                if matches!(*base, Selector::Css(ref css) if css.as_str() == "optgroup")
        ));
        assert_eq!(budget.metrics().unwrap().recipes, 2);
    }

    #[test]
    fn playwright_nth_part_rejects_invalid_and_initial_use() {
        // Arrange
        let budget = budget();
        let invalid = css(&budget, "option >> nth=first");
        let initial = css(&budget, "nth=0");

        // Act
        let invalid_result = invalid.command_selector();
        let initial_result = initial.command_selector();

        // Assert
        assert!(matches!(invalid_result, Err(HostError::Backend(_))));
        assert!(matches!(initial_result, Err(HostError::Backend(_))));
    }

    #[test]
    fn depth_overflow_preserves_the_last_admitted_recipe() {
        // Arrange
        let budget = budget();
        let mut query = css(&budget, "button");
        for _ in 1..MAX_SELECTOR_DEPTH {
            query = query.nth(0).unwrap();
        }
        let before = budget.metrics().unwrap();
        // Act
        let result = query.nth(0);
        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        assert_eq!(budget.metrics().unwrap(), before);
        assert_eq!(before.recipes, 1);
    }

    #[test]
    fn node_overflow_rejects_before_cloning_an_exponential_recipe() {
        // Arrange
        let budget = budget();
        let mut query = css(&budget, "x");
        for _ in 0..7 {
            query = query.combine(&query, QueryCombination::Or).unwrap();
        }
        let before = budget.metrics().unwrap();
        // Act
        let result = query.combine(&query, QueryCombination::Or);
        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        assert_eq!(query.0.shape.nodes, 255);
        assert_eq!(budget.metrics().unwrap(), before);
    }

    #[test]
    fn commands_admit_and_release_their_own_selector_copy() {
        // Arrange
        let budget = budget();
        let query = css(&budget, "button");
        let before = budget.metrics().unwrap();
        // Act
        let copy = query.command_selector().unwrap();
        let during = budget.metrics().unwrap();
        drop(copy);
        // Assert
        assert_eq!(during.recipes, 2);
        assert_eq!(during.bytes, before.bytes * 2);
        assert_eq!(budget.metrics().unwrap(), before);
    }

    #[test]
    fn cross_owner_combinations_do_not_consume_either_budget() {
        // Arrange
        let first = budget();
        let second = budget();
        let left = css(&first, "a");
        let right = css(&second, "b");
        let before = (first.metrics().unwrap(), second.metrics().unwrap());
        // Act
        let result = left.combine(&right, QueryCombination::And);
        // Assert
        assert!(matches!(result, Err(HostError::InvalidArgument(_))));
        assert_eq!(
            (first.metrics().unwrap(), second.metrics().unwrap()),
            before
        );
    }

    #[test]
    fn empty_has_not_text_preserves_the_recipe() {
        // Arrange
        let budget = budget();
        let query = css(&budget, "button");
        // Act
        let filtered = query.filter_text("", true).unwrap();
        // Assert
        assert!(Arc::ptr_eq(&query.0, &filtered.0));
        assert_eq!(budget.metrics().unwrap().recipes, 1);
    }

    #[test]
    fn regex_filter_is_typed_and_releases_failed_flag_admission() {
        // Arrange
        let budget = budget();
        let query = css(&budget, "article");
        let before = budget.metrics().unwrap();

        // Act
        let filtered = query.filter_regex("^title text$", "im", false).unwrap();
        let invalid = query.filter_regex("title", "ii", false);

        // Assert
        let (selector, _lease) = filtered.command_selector().unwrap();
        assert!(matches!(
            selector,
            Selector::FilterTextRegex { matcher, .. }
                if matcher.source() == "^title text$" && matcher.flags() == "im"
        ));
        assert!(matches!(invalid, Err(HostError::InvalidArgument(_))));
        assert_eq!(budget.metrics().unwrap().recipes, before.recipes + 2);
    }

    #[test]
    fn copying_a_recipe_has_independent_storage_admission() {
        // Arrange
        let budget = budget();
        let query = css(&budget, "button");
        // Act
        let copy = query.copy().unwrap();
        // Assert
        assert!(!Arc::ptr_eq(&query.0, &copy.0));
        assert_eq!(budget.metrics().unwrap().recipes, 2);
    }

    #[test]
    fn concurrent_creators_cannot_exceed_the_shared_owner_budget() {
        // Arrange
        let budget = QueryBudget::new(QueryLimits {
            recipes: 32,
            bytes: 1024 * 1024,
        })
        .unwrap();
        let barrier = std::sync::Barrier::new(5);
        // Act
        let (live, admitted) = std::thread::scope(|scope| {
            let workers: Vec<_> = (0..4)
                .map(|_| {
                    scope.spawn(|| {
                        let queries: Vec<_> = (0..16)
                            .filter_map(|_| {
                                budget
                                    .prepare_leaf(1)
                                    .ok()
                                    .map(|draft| draft.css("x".to_owned()).unwrap())
                            })
                            .collect();
                        barrier.wait();
                        barrier.wait();
                        queries.len()
                    })
                })
                .collect();
            barrier.wait();
            let live = budget.metrics().unwrap();
            barrier.wait();
            (
                live,
                workers
                    .into_iter()
                    .map(|worker| worker.join().unwrap())
                    .sum::<usize>(),
            )
        });
        // Assert
        assert_eq!(live.recipes, 32);
        assert_eq!(admitted, 32);
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }
}
