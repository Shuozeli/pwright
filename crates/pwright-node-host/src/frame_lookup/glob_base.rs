// Copyright (c) Microsoft Corporation.
// SPDX-License-Identifier: Apache-2.0
// Adapted from Playwright 1.58.2 urlMatch; modified by pwright contributors
// for bounded Rust buffers and a borrowed URL-resolution port. See crate NOTICE.
//! URL-glob preprocessing/restoration, separate from the URL parser's owner.

use super::{GlobPlan, RESOLVED_GLOB_BYTES};
use crate::HostError;
use crate::frame_tree::{FrameConversion, FrameTreeFactory};

pub const URL_WORK_BYTES: usize = 256 * 1024;

/// Borrowed output from a separately admitted WHATWG URL resolver. No parser
/// object, URL allocation or foreign runtime is retained by the pattern plan.
pub struct GlobUrlResolution<'a> {
    pub serialized: &'a str,
    pub origin: &'a str,
}

pub struct PreparedGlob {
    source: String,
    relative: String,
    skip_resolution: bool,
    _storage: FrameConversion,
}

fn exhausted(message: &'static str) -> HostError {
    HostError::ResourceExhausted(message.into())
}

impl PreparedGlob {
    pub fn new(resources: &FrameTreeFactory, glob: &str) -> Result<Self, HostError> {
        if glob.len() > RESOLVED_GLOB_BYTES {
            return Err(exhausted("Frame URL glob input exceeds byte limit"));
        }
        // At most five decimal index digits: a key is at most ten bytes.
        // Nonempty source pieces consume at least one byte; separators and
        // verbatim scheme/dot pieces consume no more bytes than their source.
        let skip_resolution = glob.starts_with('*')
            || ["about:", "data:", "chrome:", "edge:", "file:"]
                .iter()
                .any(|scheme| glob.starts_with(scheme));
        let relative_capacity = if skip_resolution { 0 } else { 10 * glob.len() };
        let bytes = glob.len() + relative_capacity + std::mem::size_of::<Self>()
            - std::mem::size_of::<FrameConversion>();
        let storage = resources.reserve_conversion(bytes)?;
        let mut source = String::with_capacity(glob.len());
        if glob.starts_with('*') {
            source.push_str(glob);
        } else {
            let mut rest = glob;
            while let Some(index) = rest.find("\\\\?") {
                source.push_str(&rest[..index]);
                source.push('?');
                rest = &rest[index + 3..];
            }
            source.push_str(rest);
        }
        let mut relative = String::with_capacity(relative_capacity);
        if !skip_resolution {
            pieces(&source, |piece| {
                relative.push_str(match piece {
                    Piece::Literal(value) => value,
                    Piece::Token(key, _) => key,
                });
                Ok(())
            })?;
        }
        debug_assert!(relative.len() <= relative_capacity);
        Ok(Self {
            source,
            relative,
            skip_resolution,
            _storage: storage,
        })
    }

    /// None means the parser must not be called (leading star or special URL).
    pub fn relative_path(&self) -> Option<&str> {
        (!self.skip_resolution).then_some(&self.relative)
    }

    /// None is the pinned parser-failure fallback to the prepared relative path.
    /// The caller owns/admit-checks parser inputs, outputs and temporary storage.
    pub fn compile(
        self,
        resources: &FrameTreeFactory,
        resolution: Option<GlobUrlResolution<'_>>,
    ) -> Result<GlobPlan, HostError> {
        if self.skip_resolution {
            return GlobPlan::compile_resolved(resources, &self.source);
        }
        let (serialized, origin) = resolution.map_or((self.relative.as_str(), None), |value| {
            (value.serialized, Some(value.origin))
        });
        if serialized.len() > URL_WORK_BYTES
            || origin.is_some_and(|value| value.len() > URL_WORK_BYTES)
        {
            return Err(exhausted(
                "Frame URL resolver output exceeds workspace limit",
            ));
        }
        let capacity = serialized
            .len()
            .checked_add(self.source.len())
            .map(|n| n.max(RESOLVED_GLOB_BYTES))
            .filter(|n| *n <= URL_WORK_BYTES)
            .ok_or_else(|| exhausted("Frame URL restoration workspace exceeds limit"))?;
        // Two fixed-capacity alternating buffers. Unicode lowercasing is
        // temporary: ASCII preserves width; a non-ASCII scalar consumes >=2
        // bytes and expands to <=3 scalars of <=4 bytes, so result <=6*n.
        // Rust 1.98's single String doubles capacity: old/new allocation overlap
        // is <=18*n plus minimum-capacity slack. Reserve 24*n+32 and its body.
        // Re-audit std allocation strategy on toolchain upgrades; see the design.
        let bytes = 2 * capacity + 24 * self.source.len() + 32 + 3 * std::mem::size_of::<String>();
        let _storage = resources.reserve_conversion(bytes)?;
        let mut current = String::with_capacity(capacity);
        current.push_str(serialized);
        let mut next = String::with_capacity(capacity);
        pieces(&self.source, |piece| {
            let Piece::Token(key, original) = piece else {
                return Ok(());
            };
            let lower;
            let replacement = if origin.is_some_and(|value| value.contains(key)) {
                lower = original.to_lowercase();
                lower.as_str()
            } else {
                original
            };
            if replace_first(&current, key, replacement, &mut next, capacity)? {
                std::mem::swap(&mut current, &mut next);
            }
            Ok(())
        })?;
        GlobPlan::compile_resolved(resources, &current)
    }
}

enum Piece<'a> {
    Literal(&'a str),
    Token(&'a str, &'a str),
}

fn pieces(
    source: &str,
    mut emit: impl FnMut(Piece<'_>) -> Result<(), HostError>,
) -> Result<(), HostError> {
    for (index, token) in source.split('/').enumerate() {
        if index != 0 {
            emit(Piece::Literal("/"))?;
        }
        if matches!(token, "" | "." | "..") {
            emit(Piece::Literal(token))?;
        } else if index == 0 && token.ends_with(':') {
            if token.contains(['*', '{']) {
                emit(Piece::Token("http:", token))?;
            } else {
                emit(Piece::Literal(token))?;
            }
        } else if let Some(question) = token.find('?') {
            if question != 0 {
                let key = Key::new(index, false);
                emit(Piece::Token(key.as_str(), &token[..question]))?;
            }
            let key = Key::new(index, true);
            emit(Piece::Token(key.as_str(), &token[question..]))?;
        } else {
            let key = Key::new(index, false);
            emit(Piece::Token(key.as_str(), token))?;
        }
    }
    Ok(())
}

struct Key {
    bytes: [u8; 10],
    len: usize,
}
impl Key {
    fn new(mut index: usize, query: bool) -> Self {
        let mut digits = [0; 5];
        let mut start = digits.len();
        loop {
            start -= 1;
            digits[start] = b'0' + u8::try_from(index % 10).expect("decimal digit");
            index /= 10;
            if index == 0 {
                break;
            }
        }
        let mut value = Self {
            bytes: [0; 10],
            len: 0,
        };
        if query {
            value.push(b'?');
        }
        value.push(b'$');
        value.push(b'_');
        for digit in &digits[start..] {
            value.push(*digit);
        }
        value.push(b'_');
        value.push(b'$');
        value
    }
    fn push(&mut self, byte: u8) {
        self.bytes[self.len] = byte;
        self.len += 1;
    }
    fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len]).expect("ASCII token key")
    }
}

fn append(output: &mut String, value: &str, capacity: usize) -> Result<(), HostError> {
    if value.len() > capacity - output.len() {
        return Err(exhausted(
            "Frame URL replacement expansion exceeds workspace limit",
        ));
    }
    output.push_str(value);
    Ok(())
}

fn replace_first(
    input: &str,
    token: &str,
    replacement: &str,
    output: &mut String,
    capacity: usize,
) -> Result<bool, HostError> {
    let Some(index) = input.find(token) else {
        return Ok(false);
    };
    let prefix = &input[..index];
    let suffix = &input[index + token.len()..];
    output.clear();
    append(output, prefix, capacity)?;
    let mut rest = replacement;
    while let Some(dollar) = rest.find('$') {
        append(output, &rest[..dollar], capacity)?;
        let expanded = match rest.as_bytes().get(dollar + 1) {
            Some(b'$') => Some("$"),
            Some(b'&') => Some(token),
            Some(b'`') => Some(prefix),
            Some(b'\'') => Some(suffix),
            _ => None,
        };
        if let Some(value) = expanded {
            append(output, value, capacity)?;
            rest = &rest[dollar + 2..];
        } else {
            append(output, "$", capacity)?;
            rest = &rest[dollar + 1..];
        }
    }
    append(output, rest, capacity)?;
    append(output, suffix, capacity)?;
    Ok(true)
}

#[cfg(test)]
mod tests;
