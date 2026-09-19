//! Pinned mime@3.0.0 lookup policy, matching Playwright 1.58.2 utilsBundle.
use super::MimeLookup;

mod table;

/// Static optional data; lookup allocates no lowercased filename or hash map.
pub struct PinnedMime;
impl PinnedMime {
    /// Longest exact extension key in the pinned table, for bounded FFI scratch.
    /// This bounds an extension copy, not the caller's whole filename.
    pub const MAX_EXTENSION_BYTES: usize = {
        let mut maximum = 0;
        let mut index = 0;
        while index < table::TYPES.len() {
            let length = table::TYPES[index].0.len();
            if length > maximum {
                maximum = length;
            }
            index += 1;
        }
        maximum
    };

    /// Exact property-key lookup after a language adapter has performed the
    /// pinned coercion/normalization. Do not normalize observable results twice.
    pub fn lookup_extension(&self, extension: &str) -> Option<&'static str> {
        table::TYPES
            .iter()
            .find_map(|(key, mime)| (*key == extension).then_some(*mime))
    }
}
impl MimeLookup for PinnedMime {
    fn lookup(&self, name: &str) -> Option<&str> {
        let filename = name.rsplit(['/', '\\']).next()?;
        let extension = filename.rsplit('.').next()?;
        let has_path = filename.len() < name.len();
        if has_path && extension.len() >= filename.len().saturating_sub(1) {
            return None;
        }
        table::TYPES.iter().find_map(|(ext, mime)| {
            extension
                .chars()
                .flat_map(char::to_lowercase)
                .eq(ext.chars())
                .then_some(*mime)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_lookup_does_not_normalize_language_results_again() {
        // Arrange
        let mime = PinnedMime;
        // Act
        let actual =
            ["txt", "TXT", "one.txt", "dir/txt", "unknown"].map(|key| mime.lookup_extension(key));
        // Assert
        assert_eq!(actual, [Some("text/plain"), None, None, None, None]);
    }

    #[test]
    fn exact_lookup_and_stack_bound_cover_every_pinned_extension() {
        // Arrange
        let mime = PinnedMime;
        // Act
        let maximum = table::TYPES.iter().map(|(key, _)| key.len()).max().unwrap();
        // Assert
        assert_eq!(maximum, PinnedMime::MAX_EXTENSION_BYTES);
        for (key, expected) in table::TYPES {
            assert!(key.len() <= PinnedMime::MAX_EXTENSION_BYTES);
            assert_eq!(mime.lookup_extension(key), Some(*expected));
        }
    }
}
