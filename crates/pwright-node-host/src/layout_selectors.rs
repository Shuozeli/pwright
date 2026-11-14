//! Bounded Playwright layout-selector parsing and browser-world execution.

use std::sync::Arc;

use serde::Serialize;

use crate::HostError;

pub const MAX_LAYOUT_SELECTOR_PARTS: usize = 256;
pub const MAX_LAYOUT_SELECTOR_PROGRAM_BYTES: usize = 256 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LayoutSelectorKind {
    LeftOf,
    RightOf,
    Above,
    Below,
    Near,
}

impl LayoutSelectorKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LeftOf => "left-of",
            Self::RightOf => "right-of",
            Self::Above => "above",
            Self::Below => "below",
            Self::Near => "near",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LayoutSelectorIssue {
    CannotBeFirst(LayoutSelectorKind),
    InvalidArguments(LayoutSelectorKind),
    Malformed,
}

impl LayoutSelectorIssue {
    pub fn message(&self) -> String {
        match self {
            Self::CannotBeFirst(kind) => {
                format!("\"{}\" selector cannot be first", kind.as_str())
            }
            Self::InvalidArguments(kind) => format!(
                "\"{}\" engine expects a selector list and optional maximum distance in pixels",
                kind.as_str()
            ),
            Self::Malformed => "Malformed layout selector".to_owned(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct LayoutClause {
    kind: LayoutSelectorKind,
    selector: String,
    #[serde(rename = "maxDistance")]
    max_distance: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
struct LayoutSimple {
    css: String,
    clauses: Vec<LayoutClause>,
}

#[derive(Clone, Debug)]
pub struct LayoutSelectorProgram {
    simples: Arc<[LayoutSimple]>,
    selector_bytes: usize,
    parts: usize,
}

impl LayoutSelectorProgram {
    pub const fn selector_bytes(&self) -> usize {
        self.selector_bytes
    }

    pub const fn part_count(&self) -> usize {
        self.parts
    }

    pub fn compile(&self, root: &str) -> Result<String, HostError> {
        let plan = serde_json::to_string(self.simples.as_ref()).map_err(HostError::backend)?;
        let program = format!(
            r#"(() => {{
  const __pwPlan = {plan};
  const __pwScores = new Map();
  const __pwRightOf = (a, b, max) => {{
    const distance = a.left - b.right;
    if (distance < 0 || (max !== null && distance > max)) return undefined;
    return distance + Math.max(b.bottom - a.bottom, 0) + Math.max(a.top - b.top, 0);
  }};
  const __pwLeftOf = (a, b, max) => {{
    const distance = b.left - a.right;
    if (distance < 0 || (max !== null && distance > max)) return undefined;
    return distance + Math.max(b.bottom - a.bottom, 0) + Math.max(a.top - b.top, 0);
  }};
  const __pwAbove = (a, b, max) => {{
    const distance = b.top - a.bottom;
    if (distance < 0 || (max !== null && distance > max)) return undefined;
    return distance + Math.max(a.left - b.left, 0) + Math.max(b.right - a.right, 0);
  }};
  const __pwBelow = (a, b, max) => {{
    const distance = a.top - b.bottom;
    if (distance < 0 || (max !== null && distance > max)) return undefined;
    return distance + Math.max(a.left - b.left, 0) + Math.max(b.right - a.right, 0);
  }};
  const __pwNear = (a, b, max) => {{
    const threshold = max === null ? 50 : max;
    let score = 0;
    if (a.left - b.right >= 0) score += a.left - b.right;
    if (b.left - a.right >= 0) score += b.left - a.right;
    if (b.top - a.bottom >= 0) score += b.top - a.bottom;
    if (a.top - b.bottom >= 0) score += a.top - b.bottom;
    return score > threshold ? undefined : score;
  }};
  const __pwScorers = {{
    "left-of": __pwLeftOf,
    "right-of": __pwRightOf,
    "above": __pwAbove,
    "below": __pwBelow,
    "near": __pwNear
  }};
  const __pwClauseScore = (element, clause) => {{
    const box = element.getBoundingClientRect();
    const scorer = __pwScorers[clause.kind];
    let best;
    for (const inner of __pw.css({root}, clause.selector)) {{
      if (inner === element) continue;
      const score = scorer(box, inner.getBoundingClientRect(), clause.maxDistance);
      if (score === undefined) continue;
      if (best === undefined || score < best) best = score;
    }}
    return best;
  }};
  const __pwMatches = (element, simple) => {{
    if (!element.matches(simple.css)) return false;
    for (const clause of simple.clauses) {{
      const score = __pwClauseScore(element, clause);
      if (score === undefined) return false;
      __pwScores.set(element, score);
    }}
    return true;
  }};
  const __pwLast = __pwPlan[__pwPlan.length - 1];
  let __pwElements = __pw.css({root}, __pwLast.css).filter(element => __pwMatches(element, __pwLast));
  __pwElements = __pwElements.filter(element => {{
    let current = element;
    for (let index = __pwPlan.length - 2; index >= 0; --index) {{
      current = current.previousElementSibling;
      if (!current || !__pwMatches(current, __pwPlan[index])) return false;
    }}
    return true;
  }});
  if (__pwScores.size) {{
    __pwElements.sort((a, b) => {{
      const aScore = __pwScores.get(a);
      const bScore = __pwScores.get(b);
      if (aScore === bScore) return 0;
      if (aScore === undefined) return 1;
      if (bScore === undefined) return -1;
      return aScore - bScore;
    }});
  }}
  return __pwElements;
}})()"#
        );
        if program.len() > MAX_LAYOUT_SELECTOR_PROGRAM_BYTES {
            return Err(HostError::ResourceExhausted(format!(
                "layout selector program exceeds {MAX_LAYOUT_SELECTOR_PROGRAM_BYTES} UTF-8 bytes"
            )));
        }
        Ok(program)
    }
}

pub fn parse_layout_selector(
    source: &str,
) -> Result<Option<LayoutSelectorProgram>, LayoutSelectorIssue> {
    let trimmed = source.trim();
    for kind in kinds() {
        if trimmed.starts_with(kind.as_str()) && trimmed[kind.as_str().len()..].starts_with('=') {
            return Err(LayoutSelectorIssue::CannotBeFirst(kind));
        }
    }
    if !contains_layout_marker(trimmed) {
        return Ok(None);
    }

    let pieces = split_adjacent_siblings(trimmed)?;
    let mut simples = Vec::with_capacity(pieces.len());
    let mut parts = pieces.len();
    for piece in pieces {
        let simple = parse_simple(piece)?;
        parts = parts.saturating_add(simple.clauses.len());
        if parts > MAX_LAYOUT_SELECTOR_PARTS {
            return Err(LayoutSelectorIssue::Malformed);
        }
        simples.push(simple);
    }
    Ok(Some(LayoutSelectorProgram {
        simples: Arc::from(simples),
        selector_bytes: source.len(),
        parts,
    }))
}

const fn kinds() -> [LayoutSelectorKind; 5] {
    [
        LayoutSelectorKind::LeftOf,
        LayoutSelectorKind::RightOf,
        LayoutSelectorKind::Above,
        LayoutSelectorKind::Below,
        LayoutSelectorKind::Near,
    ]
}

fn contains_layout_marker(source: &str) -> bool {
    kinds()
        .into_iter()
        .any(|kind| source.contains(&format!(":{}(", kind.as_str())))
}

fn split_adjacent_siblings(source: &str) -> Result<Vec<&str>, LayoutSelectorIssue> {
    let mut pieces = Vec::new();
    let mut start = 0;
    let mut parentheses = 0usize;
    let mut brackets = 0usize;
    let mut quote = None;
    let mut escaped = false;
    for (index, character) in source.char_indices() {
        if let Some(expected) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == expected {
                quote = None;
            }
            continue;
        }
        match character {
            '\'' | '"' => quote = Some(character),
            '(' => parentheses = parentheses.saturating_add(1),
            ')' => {
                parentheses = parentheses
                    .checked_sub(1)
                    .ok_or(LayoutSelectorIssue::Malformed)?;
            }
            '[' => brackets = brackets.saturating_add(1),
            ']' => {
                brackets = brackets
                    .checked_sub(1)
                    .ok_or(LayoutSelectorIssue::Malformed)?;
            }
            '+' if parentheses == 0 && brackets == 0 => {
                let piece = source[start..index].trim();
                if piece.is_empty() {
                    return Err(LayoutSelectorIssue::Malformed);
                }
                pieces.push(piece);
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    if quote.is_some() || parentheses != 0 || brackets != 0 {
        return Err(LayoutSelectorIssue::Malformed);
    }
    let last = source[start..].trim();
    if last.is_empty() {
        return Err(LayoutSelectorIssue::Malformed);
    }
    pieces.push(last);
    Ok(pieces)
}

fn parse_simple(source: &str) -> Result<LayoutSimple, LayoutSelectorIssue> {
    let mut css = String::with_capacity(source.len());
    let mut clauses = Vec::new();
    let mut cursor = 0;
    while let Some((start, kind, open)) = find_marker(source, cursor) {
        css.push_str(&source[cursor..start]);
        let close = matching_parenthesis(source, open)?;
        clauses.push(parse_clause(kind, &source[open + 1..close])?);
        cursor = close + 1;
    }
    css.push_str(&source[cursor..]);
    let css = css.trim();
    Ok(LayoutSimple {
        css: if css.is_empty() { "*" } else { css }.to_owned(),
        clauses,
    })
}

fn find_marker(source: &str, start: usize) -> Option<(usize, LayoutSelectorKind, usize)> {
    let suffix = &source[start..];
    kinds()
        .into_iter()
        .filter_map(|kind| {
            let needle = format!(":{}(", kind.as_str());
            suffix.find(&needle).map(|offset| {
                let marker = start + offset;
                (marker, kind, marker + needle.len() - 1)
            })
        })
        .min_by_key(|(marker, _, _)| *marker)
}

fn matching_parenthesis(source: &str, open: usize) -> Result<usize, LayoutSelectorIssue> {
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    for (relative, character) in source[open..].char_indices() {
        let index = open + relative;
        if let Some(expected) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == expected {
                quote = None;
            }
            continue;
        }
        match character {
            '\'' | '"' => quote = Some(character),
            '(' => depth = depth.saturating_add(1),
            ')' => {
                depth = depth.checked_sub(1).ok_or(LayoutSelectorIssue::Malformed)?;
                if depth == 0 {
                    return Ok(index);
                }
            }
            _ => {}
        }
    }
    Err(LayoutSelectorIssue::Malformed)
}

fn parse_clause(
    kind: LayoutSelectorKind,
    arguments: &str,
) -> Result<LayoutClause, LayoutSelectorIssue> {
    let mut comma = None;
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    for (index, character) in arguments.char_indices() {
        if let Some(expected) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == expected {
                quote = None;
            }
            continue;
        }
        match character {
            '\'' | '"' => quote = Some(character),
            '(' | '[' => depth = depth.saturating_add(1),
            ')' | ']' => depth = depth.checked_sub(1).ok_or(LayoutSelectorIssue::Malformed)?,
            ',' if depth == 0 => comma = Some(index),
            _ => {}
        }
    }
    if quote.is_some() || depth != 0 {
        return Err(LayoutSelectorIssue::Malformed);
    }

    let (selector, max_distance) = if let Some(comma) = comma {
        let candidate = arguments[comma + 1..].trim();
        match candidate.parse::<f64>() {
            Ok(distance) if distance.is_finite() => (arguments[..comma].trim(), Some(distance)),
            _ => (arguments.trim(), None),
        }
    } else {
        let selector = arguments.trim();
        if selector.parse::<f64>().is_ok() {
            return Err(LayoutSelectorIssue::InvalidArguments(kind));
        }
        (selector, None)
    };
    if selector.is_empty() {
        return Err(LayoutSelectorIssue::InvalidArguments(kind));
    }
    Ok(LayoutClause {
        kind,
        selector: selector.to_owned(),
        max_distance,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_composed_layout_selector_with_distances() {
        // Arrange
        let source = "div:right-of(#id3, 50) + div:above(#id8)";

        // Act
        let program = parse_layout_selector(source).unwrap().unwrap();
        let compiled = program.compile("document").unwrap();

        // Assert
        assert_eq!(program.part_count(), 4);
        assert!(compiled.contains("\"right-of\""));
        assert!(compiled.contains("\"maxDistance\":50.0"));
        assert!(compiled.contains("previousElementSibling"));
    }

    #[test]
    fn rejects_layout_engine_as_the_first_selector_part() {
        // Arrange
        let source = "left-of=\"div\"";

        // Act
        let issue = parse_layout_selector(source).unwrap_err();

        // Assert
        assert_eq!(issue.message(), "\"left-of\" selector cannot be first");
    }

    #[test]
    fn rejects_numeric_near_argument_without_a_selector() {
        // Arrange
        let source = ":near(50)";

        // Act
        let issue = parse_layout_selector(source).unwrap_err();

        // Assert
        assert_eq!(
            issue.message(),
            "\"near\" engine expects a selector list and optional maximum distance in pixels"
        );
    }
}
