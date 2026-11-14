//! Bounded raw HTTP header values, independent of browser and transport owners.
//!
//! CDP's extra-info headers can contain newline-separated repeated values.
//! Provisional headers cannot be treated as raw headers when extra-info is
//! expected. Correlation of those events with requests is a separate leaf.

use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};

use crate::HttpHeader;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RawHeaderLimits {
    pub entries: usize,
    pub input_bytes: usize,
    pub retained_bytes: usize,
    pub ffi_copy_bytes: usize,
}

impl RawHeaderLimits {
    pub fn validate(self) -> Result<Self, RawHeaderError> {
        for (field, limit) in [
            ("header entries", self.entries),
            ("input header bytes", self.input_bytes),
            ("retained header bytes", self.retained_bytes),
            ("FFI header copy bytes", self.ffi_copy_bytes),
        ] {
            if limit == 0 {
                return Err(RawHeaderError::ZeroLimit { field });
            }
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RawHeaderSource {
    /// Values from `Network.requestWillBeSentExtraInfo` or
    /// `Network.responseReceivedExtraInfo`; newline separates wire values.
    CdpExtraInfo,
    /// Only used when CDP explicitly reports that extra-info is unavailable.
    Provisional,
}

/// Python's public RawHeaders reader de-duplicates equal values per name;
/// Playwright Core's Node reader preserves them. Both retain the original
/// array, so callers must select their public-language semantics explicitly.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RawHeaderReader {
    PythonUnique,
    NodePreserve,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RawHeaderError {
    ZeroLimit {
        field: &'static str,
    },
    InvalidName,
    SizeOverflow {
        field: &'static str,
    },
    Capacity {
        field: &'static str,
        actual: usize,
        limit: usize,
    },
}

impl Display for RawHeaderError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroLimit { field } => write!(formatter, "{field} limit must be positive"),
            Self::InvalidName => formatter.write_str("raw header name is invalid"),
            Self::SizeOverflow { field } => write!(formatter, "{field} size overflow"),
            Self::Capacity {
                field,
                actual,
                limit,
            } => write!(formatter, "{field} exceeded: {actual} > {limit}"),
        }
    }
}

impl std::error::Error for RawHeaderError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawHeaders {
    headers: Vec<HttpHeader>,
    limits: RawHeaderLimits,
}

impl RawHeaders {
    /// Build one snapshot from borrowed CDP header entries. The caller must
    /// supply bounded input; this method checks bytes before copying strings.
    pub fn from_entries<'a>(
        entries: impl IntoIterator<Item = (&'a str, &'a str)>,
        source: RawHeaderSource,
        limits: RawHeaderLimits,
    ) -> Result<Self, RawHeaderError> {
        let limits = limits.validate()?;
        let mut headers = Vec::new();
        let mut input_bytes = 0_usize;
        let mut retained_bytes = 0_usize;
        for (name, value) in entries {
            if name.is_empty() || !name.bytes().all(|byte| (0x21..=0x7e).contains(&byte)) {
                return Err(RawHeaderError::InvalidName);
            }
            let next_input = input_bytes
                .checked_add(name.len())
                .and_then(|sum| sum.checked_add(value.len()))
                .ok_or(RawHeaderError::SizeOverflow {
                    field: "input header bytes",
                })?;
            check("input header bytes", next_input, limits.input_bytes)?;
            input_bytes = next_input;

            let mut append = |part: &str| {
                let next_count =
                    headers
                        .len()
                        .checked_add(1)
                        .ok_or(RawHeaderError::SizeOverflow {
                            field: "header entries",
                        })?;
                check("header entries", next_count, limits.entries)?;
                let next_bytes = retained_bytes
                    .checked_add(name.len())
                    .and_then(|sum| sum.checked_add(part.len()))
                    .ok_or(RawHeaderError::SizeOverflow {
                        field: "retained header bytes",
                    })?;
                check("retained header bytes", next_bytes, limits.retained_bytes)?;
                retained_bytes = next_bytes;
                headers.push(HttpHeader::new(name, part));
                Ok::<(), RawHeaderError>(())
            };
            match source {
                RawHeaderSource::CdpExtraInfo => {
                    for part in value.split('\n') {
                        append(part.trim())?;
                    }
                }
                RawHeaderSource::Provisional => append(value)?,
            }
        }
        headers.shrink_to_fit();
        Ok(Self { headers, limits })
    }

    /// The original spelling and repeated values are retained in event order.
    /// Cross-name order is only as reliable as the supplying CDP object.
    pub fn headers_array(&self) -> &[HttpHeader] {
        &self.headers
    }

    pub fn header_values(&self, name: &str, reader: RawHeaderReader) -> Vec<&str> {
        let mut values = Vec::new();
        for header in &self.headers {
            if header.name.eq_ignore_ascii_case(name)
                && (reader == RawHeaderReader::NodePreserve
                    || !values.contains(&header.value.as_str()))
            {
                values.push(header.value.as_str());
            }
        }
        values
    }

    pub fn header_value(
        &self,
        name: &str,
        reader: RawHeaderReader,
    ) -> Result<Option<String>, RawHeaderError> {
        let values = self.header_values(name, reader);
        if values.is_empty() {
            return Ok(None);
        }
        let separator = if name.eq_ignore_ascii_case("set-cookie") {
            "\n"
        } else {
            ", "
        };
        let bytes = values
            .iter()
            .try_fold(0_usize, |sum, value| sum.checked_add(value.len()))
            .and_then(|sum| sum.checked_add(separator.len().saturating_mul(values.len() - 1)))
            .ok_or(RawHeaderError::SizeOverflow {
                field: "FFI header copy bytes",
            })?;
        check("FFI header copy bytes", bytes, self.limits.ffi_copy_bytes)?;
        Ok(Some(values.join(separator)))
    }

    /// Return a lowercase-name map with values joined according to the
    /// selected public-language reader. This is a bounded FFI copy.
    pub fn all_headers(
        &self,
        reader: RawHeaderReader,
    ) -> Result<BTreeMap<String, String>, RawHeaderError> {
        let mut result = BTreeMap::new();
        let mut copied = 0_usize;
        for header in &self.headers {
            let name = header.name.to_ascii_lowercase();
            if result.contains_key(&name) {
                continue;
            }
            let Some(value) = self.header_value(&name, reader)? else {
                continue;
            };
            let next = copied
                .checked_add(name.len())
                .and_then(|sum| sum.checked_add(value.len()))
                .ok_or(RawHeaderError::SizeOverflow {
                    field: "FFI header copy bytes",
                })?;
            check("FFI header copy bytes", next, self.limits.ffi_copy_bytes)?;
            copied = next;
            result.insert(name, value);
        }
        Ok(result)
    }

    pub fn headers_array_owned(&self) -> Result<Vec<HttpHeader>, RawHeaderError> {
        let bytes = self.headers.iter().try_fold(0_usize, |sum, header| {
            sum.checked_add(header.name.len())
                .and_then(|sum| sum.checked_add(header.value.len()))
        });
        let bytes = bytes.ok_or(RawHeaderError::SizeOverflow {
            field: "FFI header copy bytes",
        })?;
        check("FFI header copy bytes", bytes, self.limits.ffi_copy_bytes)?;
        Ok(self.headers.clone())
    }
}

fn check(field: &'static str, actual: usize, limit: usize) -> Result<(), RawHeaderError> {
    if actual > limit {
        return Err(RawHeaderError::Capacity {
            field,
            actual,
            limit,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> RawHeaderLimits {
        RawHeaderLimits {
            entries: 8,
            input_bytes: 128,
            retained_bytes: 128,
            ffi_copy_bytes: 128,
        }
    }

    #[test]
    fn extra_info_preserves_array_duplicates_and_distinguishes_public_readers() {
        // Arrange
        let entries = [
            ("Header-A", "value-a\nvalue-a-1\nvalue-a"),
            ("set-cookie", "a=b\nc=d"),
        ];

        // Act
        let headers = RawHeaders::from_entries(entries, RawHeaderSource::CdpExtraInfo, limits())
            .expect("bounded headers");

        // Assert
        assert_eq!(headers.headers_array().len(), 5);
        assert_eq!(
            headers.header_values("HEADER-A", RawHeaderReader::PythonUnique),
            vec!["value-a", "value-a-1"]
        );
        assert_eq!(
            headers.header_value("header-a", RawHeaderReader::PythonUnique),
            Ok(Some("value-a, value-a-1".into()))
        );
        assert_eq!(
            headers.header_value("header-a", RawHeaderReader::NodePreserve),
            Ok(Some("value-a, value-a-1, value-a".into()))
        );
        assert_eq!(
            headers.header_value("Set-Cookie", RawHeaderReader::PythonUnique),
            Ok(Some("a=b\nc=d".into()))
        );
    }

    #[test]
    fn provisional_value_is_not_split_as_extra_info() {
        // Arrange
        let entries = [("x-test", "one\ntwo")];

        // Act
        let headers = RawHeaders::from_entries(entries, RawHeaderSource::Provisional, limits())
            .expect("bounded provisional headers");

        // Assert
        assert_eq!(
            headers.headers_array(),
            &[HttpHeader::new("x-test", "one\ntwo")]
        );
    }

    #[test]
    fn expanded_entry_limit_rejects_before_copying_an_unbounded_snapshot() {
        // Arrange
        let profile = RawHeaderLimits {
            entries: 2,
            ..limits()
        };

        // Act
        let result = RawHeaders::from_entries(
            [("x-test", "one\ntwo\nthree")],
            RawHeaderSource::CdpExtraInfo,
            profile,
        );

        // Assert
        assert_eq!(
            result,
            Err(RawHeaderError::Capacity {
                field: "header entries",
                actual: 3,
                limit: 2,
            })
        );
    }

    #[test]
    fn ffi_copy_limit_rejects_oversized_header_view() {
        // Arrange
        let profile = RawHeaderLimits {
            ffi_copy_bytes: 4,
            ..limits()
        };
        let headers =
            RawHeaders::from_entries([("x-test", "value")], RawHeaderSource::Provisional, profile)
                .expect("retained input fits");

        // Act
        let result = headers.headers_array_owned();

        // Assert
        assert_eq!(
            result,
            Err(RawHeaderError::Capacity {
                field: "FFI header copy bytes",
                actual: 11,
                limit: 4,
            })
        );
    }

    #[test]
    fn all_headers_lowercases_names_and_preserves_public_join_rules() {
        // Arrange
        let headers = RawHeaders::from_entries(
            [("Header-A", " one \n two "), ("SET-cookie", "a=b\nc=d")],
            RawHeaderSource::CdpExtraInfo,
            limits(),
        )
        .expect("bounded extra-info headers");

        // Act
        let result = headers
            .all_headers(RawHeaderReader::PythonUnique)
            .expect("bounded FFI map");

        // Assert
        assert_eq!(result.get("header-a"), Some(&"one, two".to_owned()));
        assert_eq!(result.get("set-cookie"), Some(&"a=b\nc=d".to_owned()));
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn retained_budget_counts_repeated_name_after_newline_expansion() {
        // Arrange
        let profile = RawHeaderLimits {
            retained_bytes: 11,
            ..limits()
        };

        // Act
        let result =
            RawHeaders::from_entries([("x-test", "a\nb")], RawHeaderSource::CdpExtraInfo, profile);

        // Assert
        assert_eq!(
            result,
            Err(RawHeaderError::Capacity {
                field: "retained header bytes",
                actual: 14,
                limit: 11,
            })
        );
    }

    #[test]
    fn input_budget_rejects_before_copying_long_value() {
        // Arrange
        let profile = RawHeaderLimits {
            input_bytes: 7,
            ..limits()
        };

        // Act
        let result =
            RawHeaders::from_entries([("x-test", "ab")], RawHeaderSource::CdpExtraInfo, profile);

        // Assert
        assert_eq!(
            result,
            Err(RawHeaderError::Capacity {
                field: "input header bytes",
                actual: 8,
                limit: 7,
            })
        );
    }

    #[test]
    fn aggregate_ffi_copy_budget_is_checked_across_all_names() {
        // Arrange
        let profile = RawHeaderLimits {
            ffi_copy_bytes: 7,
            ..limits()
        };
        let headers = RawHeaders::from_entries(
            [("x-a", "a"), ("x-b", "b")],
            RawHeaderSource::Provisional,
            profile,
        )
        .expect("retained input fits");

        // Act
        let result = headers.all_headers(RawHeaderReader::PythonUnique);

        // Assert
        assert_eq!(
            result,
            Err(RawHeaderError::Capacity {
                field: "FFI header copy bytes",
                actual: 8,
                limit: 7,
            })
        );
    }

    #[test]
    fn node_reader_retains_duplicate_set_cookie_values_in_joined_map() {
        // Arrange
        let headers = RawHeaders::from_entries(
            [("Set-Cookie", "a=b\na=b")],
            RawHeaderSource::CdpExtraInfo,
            limits(),
        )
        .expect("bounded extra-info headers");

        // Act
        let node = headers
            .all_headers(RawHeaderReader::NodePreserve)
            .expect("bounded Node view");
        let python = headers
            .all_headers(RawHeaderReader::PythonUnique)
            .expect("bounded Python view");

        // Assert
        assert_eq!(node.get("set-cookie"), Some(&"a=b\na=b".to_owned()));
        assert_eq!(python.get("set-cookie"), Some(&"a=b".to_owned()));
        assert_eq!(headers.headers_array().len(), 2);
    }
}
