//! Generated common wire values shared by independently installable codecs.
//!
//! This crate contains no transport, executor, browser backend, YAML parser,
//! or build script. Capability codecs opt into it only when their source schema
//! imports `pwright.common`.

#[doc(hidden)]
#[rustfmt::skip]
mod generated_schema;

pub use generated_schema::*;

#[cfg(test)]
mod tests {
    use super::{CATALOG, PWRIGHT_COMMON_DURATION_SCHEMA};

    #[test]
    fn generated_common_library_catalog_is_valid_and_contains_duration_constraints() {
        // Arrange
        let nanoseconds = PWRIGHT_COMMON_DURATION_SCHEMA.fields()[1];

        // Act
        let validation = CATALOG.validate();

        // Assert
        assert_eq!(validation, Ok(()));
        let range = nanoseconds
            .numeric_range()
            .expect("nanoseconds should have a numeric range");
        assert_eq!(range.minimum(), None);
        assert_eq!(range.maximum(), Some(999_999_999));
    }
}
