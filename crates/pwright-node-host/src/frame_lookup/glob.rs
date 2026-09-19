// Copyright (c) Microsoft Corporation.
// SPDX-License-Identifier: Apache-2.0
// Adapted from Playwright 1.58.2 urlMatch; modified by pwright contributors
// for Rust iteration and explicit bounded resource admission. See ../../NOTICE
// (crate root NOTICE) and LICENSE-APACHE-2.0 for attribution and license text.
//! Playwright 1.58.2 glob-to-RegExp planning, after baseURL resolution.

use crate::HostError;
use crate::frame_tree::{FrameConversion, FrameTreeFactory};

pub const RESOLVED_GLOB_BYTES: usize = 16 * 1024;

/// A compiled pattern whose native allocation stays charged through conversion.
/// This does not resolve a baseURL or execute a RegExp in a different engine.
pub struct GlobPlan {
    pattern: String,
    _storage: FrameConversion,
}

impl GlobPlan {
    pub fn compile_resolved(resources: &FrameTreeFactory, glob: &str) -> Result<Self, HostError> {
        const {
            assert!(
                std::mem::size_of::<GlobPlan>()
                    == std::mem::size_of::<String>() + std::mem::size_of::<FrameConversion>()
            )
        };
        if glob.len() > RESOLVED_GLOB_BYTES {
            return Err(HostError::ResourceExhausted(
                "resolved Frame URL glob exceeds byte limit".into(),
            ));
        }
        // The largest token is single '*' -> '([^/]*)' (seven bytes).
        // Every other token emits at most seven bytes per consumed UTF-8 byte;
        // anchors cost two. No token vector, input copy or growth is needed.
        let capacity = glob
            .len()
            .checked_mul(7)
            .and_then(|n| n.checked_add(2))
            .ok_or_else(|| {
                HostError::ResourceExhausted("Frame URL glob capacity overflow".into())
            })?;
        let bytes = capacity
            .checked_add(std::mem::size_of::<String>())
            .ok_or_else(|| {
                HostError::ResourceExhausted("Frame URL glob storage overflow".into())
            })?;
        let storage = resources.reserve_conversion(bytes)?;
        let mut pattern = String::with_capacity(capacity);
        pattern.push('^');
        let mut chars = glob.chars().peekable();
        let mut previous = None;
        let mut in_group = false;
        while let Some(c) = chars.next() {
            if c == '\\'
                && let Some(escaped) = chars.next()
            {
                literal(&mut pattern, escaped);
                previous = Some(escaped);
                continue;
            }
            if c == '*' {
                let before = previous;
                let mut count = 1;
                while chars.peek() == Some(&'*') {
                    chars.next();
                    count += 1;
                }
                previous = Some('*');
                if count == 1 {
                    pattern.push_str("([^/]*)");
                } else if chars.peek() == Some(&'/') {
                    pattern.push_str(if before == Some('/') {
                        "((.+/)|)"
                    } else {
                        "(.*/)"
                    });
                    chars.next();
                    previous = Some('/');
                } else {
                    pattern.push_str("(.*)");
                }
                continue;
            }
            match c {
                // Keep the pinned version's boolean group behavior, including
                // malformed/nested groups; the eventual JS RegExp parses it.
                '{' => {
                    in_group = true;
                    pattern.push('(');
                }
                '}' => {
                    in_group = false;
                    pattern.push(')');
                }
                ',' if in_group => pattern.push('|'),
                ',' => pattern.push_str("\\,"),
                other => literal(&mut pattern, other),
            }
            previous = Some(c);
        }
        pattern.push('$');
        debug_assert!(pattern.len() <= capacity);
        Ok(Self {
            pattern,
            _storage: storage,
        })
    }

    pub fn pattern(&self) -> &str {
        &self.pattern
    }
}

fn literal(output: &mut String, c: char) {
    if matches!(
        c,
        '$' | '^' | '+' | '.' | '*' | '(' | ')' | '|' | '\\' | '?' | '{' | '}' | '[' | ']'
    ) {
        output.push('\\');
    }
    output.push(c);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame_tree::FrameTreeLimits;

    fn resources(bytes: u64, revisions: usize) -> FrameTreeFactory {
        FrameTreeFactory::new(FrameTreeLimits {
            states: 1,
            frames_per_page: 1,
            revisions,
            bytes,
        })
        .unwrap()
    }

    #[test]
    fn pinned_glob_tokens_include_legacy_malformed_group_behavior() {
        // Arrange
        let resources = resources(128 * 1024, 2);
        let cases = [
            ("", "^$"),
            ("*", "^([^/]*)$"),
            ("**", "^(.*)$"),
            ("**/a", "^(.*/)a$"),
            ("a/**/b", "^a/((.+/)|)b$"),
            ("a/*/b", "^a/([^/]*)/b$"),
            ("{a,b}", "^(a|b)$"),
            ("{{a,b},c}", "^((a|b)\\,c)$"),
            ("a}", "^a)$"),
            ("{a", "^(a$"),
            ("a?q=1", "^a\\?q=1$"),
            ("\\*", "^\\*$"),
            ("\\,", "^,$"),
            ("\\", "^\\\\$"),
            ("你好/🦀", "^你好/🦀$"),
        ];
        // Act / Assert
        for (glob, expected) in cases {
            let plan = GlobPlan::compile_resolved(&resources, glob).unwrap();
            assert_eq!(plan.pattern(), expected, "{glob:?}");
        }
    }

    #[test]
    fn maximum_input_never_grows_beyond_its_precharged_capacity() {
        // Arrange
        let resources = resources(128 * 1024, 1);
        let glob = "*a".repeat(RESOLVED_GLOB_BYTES / 2);
        // Act
        let plan = GlobPlan::compile_resolved(&resources, &glob).unwrap();
        // Assert
        assert_eq!(plan.pattern.capacity(), 7 * RESOLVED_GLOB_BYTES + 2);
        assert!(resources.metrics().unwrap().bytes >= plan.pattern.capacity() as u64);
    }

    #[test]
    fn oversized_input_rejects_without_admission_or_truncation() {
        // Arrange
        let resources = resources(128 * 1024, 1);
        let before = resources.metrics().unwrap();
        // Act
        let result = GlobPlan::compile_resolved(&resources, &"a".repeat(RESOLVED_GLOB_BYTES + 1));
        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        assert_eq!(resources.metrics().unwrap(), before);
    }

    #[test]
    fn failed_byte_admission_releases_the_read_slot() {
        // Arrange
        let resources = resources(1, 1);
        let before = resources.metrics().unwrap();
        // Act
        let result = GlobPlan::compile_resolved(&resources, "*");
        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        assert_eq!(resources.metrics().unwrap(), before);
    }

    #[test]
    fn plan_retains_its_slot_until_drop_and_then_releases_all_bytes() {
        // Arrange
        let resources = resources(1024, 1);
        let plan = GlobPlan::compile_resolved(&resources, "*").unwrap();
        let before = resources.metrics().unwrap();
        // Act
        let result = GlobPlan::compile_resolved(&resources, "other");
        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        assert_eq!(resources.metrics().unwrap(), before);
        drop(plan);
        assert_eq!(resources.metrics().unwrap().bytes, 0);
        assert_eq!(resources.metrics().unwrap().revisions, 0);
    }
}
