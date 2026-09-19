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
fn tokenization_preserves_scheme_dots_and_query_boundaries() {
    // Arrange
    let resources = resources(1024 * 1024, 4);
    // Act
    let plan = PreparedGlob::new(&resources, "http*://Example/{a,b}/../item?q=1").unwrap();
    // Assert
    assert_eq!(
        plan.relative_path(),
        Some("http://$_2_$/$_3_$/../$_5_$?$_5_$")
    );
}

#[test]
fn direct_globs_skip_the_parser_but_only_special_schemes_clean_escaped_queries() {
    // Arrange
    let resources = resources(1024 * 1024, 4);
    let cases = [
        ("**/\\\\?", "**/\\\\?"),
        ("about:a\\\\?", "about:a?"),
        ("data:x", "data:x"),
    ];
    // Act / Assert
    for (input, source) in cases {
        let prepared = PreparedGlob::new(&resources, input).unwrap();
        assert!(prepared.relative_path().is_none());
        let actual = prepared.compile(&resources, None).unwrap();
        let expected = GlobPlan::compile_resolved(&resources, source).unwrap();
        assert_eq!(actual.pattern(), expected.pattern());
    }
}

#[test]
fn parsed_origin_controls_case_normalization_without_lowercasing_paths() {
    // Arrange
    let resources = resources(1024 * 1024, 4);
    let prepared = PreparedGlob::new(&resources, "HTTPS://EXAMPLE.TEST/Path").unwrap();
    // Act
    let actual = prepared
        .compile(
            &resources,
            Some(GlobUrlResolution {
                serialized: "https://$_2_$/$_3_$",
                origin: "https://$_2_$",
            }),
        )
        .unwrap();
    // Assert
    let expected = GlobPlan::compile_resolved(&resources, "https://example.test/Path").unwrap();
    assert_eq!(actual.pattern(), expected.pattern());
}

#[test]
fn parser_failure_restores_the_original_relative_glob() {
    // Arrange
    let resources = resources(1024 * 1024, 4);
    let prepared = PreparedGlob::new(&resources, "folder/*?q=1").unwrap();
    // Act
    let actual = prepared.compile(&resources, None).unwrap();
    // Assert
    let expected = GlobPlan::compile_resolved(&resources, "folder/*?q=1").unwrap();
    assert_eq!(actual.pattern(), expected.pattern());
}

#[test]
fn string_replacement_preserves_legacy_dollar_substitutions() {
    // Arrange
    let mut output = String::with_capacity(128);
    // Act
    replace_first(
        "prefix TOKEN suffix",
        "TOKEN",
        "$$|$&|$`|$'|$1",
        &mut output,
        128,
    )
    .unwrap();
    // Assert
    assert_eq!(output, "prefix $|TOKEN|prefix | suffix|$1 suffix");
}

#[test]
fn replacement_expansion_rejects_before_buffer_growth() {
    // Arrange
    let mut output = String::with_capacity(20);
    // Act
    let result = replace_first("12345678X", "X", "$`$`$`", &mut output, 20);
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(output.capacity(), 20);
}

#[test]
fn preparation_rejection_releases_all_admission() {
    // Arrange
    let resources = resources(1, 4);
    let before = resources.metrics().unwrap();
    // Act
    let result = PreparedGlob::new(&resources, "https://example.test/*");
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(resources.metrics().unwrap(), before);
}

#[test]
fn restoration_slot_failure_releases_the_consumed_preparation() {
    // Arrange
    let resources = resources(1024 * 1024, 1);
    let prepared = PreparedGlob::new(&resources, "folder/*").unwrap();
    // Act
    let result = prepared.compile(&resources, None);
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(resources.metrics().unwrap().bytes, 0);
    assert_eq!(resources.metrics().unwrap().revisions, 0);
}

#[test]
fn oversized_parser_output_rejects_before_copy_and_releases_preparation() {
    // Arrange
    let resources = resources(1024 * 1024, 4);
    let prepared = PreparedGlob::new(&resources, "a").unwrap();
    let oversized = "x".repeat(URL_WORK_BYTES + 1);
    // Act
    let result = prepared.compile(
        &resources,
        Some(GlobUrlResolution {
            serialized: &oversized,
            origin: "null",
        }),
    );
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(resources.metrics().unwrap().bytes, 0);
}

#[test]
fn final_glob_limit_rejects_large_restored_results_without_retention() {
    // Arrange
    let resources = resources(1024 * 1024, 4);
    let prepared = PreparedGlob::new(&resources, "a").unwrap();
    let oversized = "x".repeat(RESOLVED_GLOB_BYTES + 1);
    // Act
    let result = prepared.compile(
        &resources,
        Some(GlobUrlResolution {
            serialized: &oversized,
            origin: "null",
        }),
    );
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(resources.metrics().unwrap().bytes, 0);
    assert_eq!(resources.metrics().unwrap().revisions, 0);
}

#[test]
fn completed_plan_retains_only_its_compiler_admission_until_drop() {
    // Arrange
    let resources = resources(1024 * 1024, 4);
    let prepared = PreparedGlob::new(&resources, "relative/*").unwrap();
    // Act
    let plan = prepared.compile(&resources, None).unwrap();
    // Assert
    assert_eq!(resources.metrics().unwrap().revisions, 1);
    drop(plan);
    assert_eq!(resources.metrics().unwrap().bytes, 0);
    assert_eq!(resources.metrics().unwrap().revisions, 0);
}

#[test]
fn maximum_component_index_fits_the_allocation_free_token_key() {
    // Arrange
    let resources = resources(2 * 1024 * 1024, 4);
    let source = format!("{}?", "/".repeat(RESOLVED_GLOB_BYTES - 1));
    // Act
    let prepared = PreparedGlob::new(&resources, &source).unwrap();
    // Assert
    assert!(prepared.relative_path().unwrap().ends_with("?$_16383_$"));
    assert!(prepared.relative.len() <= 10 * RESOLVED_GLOB_BYTES);
}

#[test]
fn restoration_byte_failure_releases_preparation_and_read_slots() {
    // Arrange
    let resources = resources(1024, 4);
    let prepared = PreparedGlob::new(&resources, "a").unwrap();
    // Act
    let result = prepared.compile(&resources, None);
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(resources.metrics().unwrap().bytes, 0);
    assert_eq!(resources.metrics().unwrap().revisions, 0);
}

#[test]
fn compiler_slot_failure_releases_both_preparation_and_restoration() {
    // Arrange
    let resources = resources(1024 * 1024, 2);
    let prepared = PreparedGlob::new(&resources, "a").unwrap();
    // Act
    let result = prepared.compile(&resources, None);
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(resources.metrics().unwrap().bytes, 0);
    assert_eq!(resources.metrics().unwrap().revisions, 0);
}

#[test]
fn oversized_input_rejects_before_preparation_admission() {
    // Arrange
    let resources = resources(1024 * 1024, 4);
    let input = "a".repeat(RESOLVED_GLOB_BYTES + 1);
    // Act
    let result = PreparedGlob::new(&resources, &input);
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(resources.metrics().unwrap().bytes, 0);
    assert_eq!(resources.metrics().unwrap().revisions, 0);
}

#[test]
fn expanding_unicode_lowercase_rejects_without_retained_storage() {
    // Arrange
    let resources = resources(2 * 1024 * 1024, 3);
    let input = "\u{130}".repeat(RESOLVED_GLOB_BYTES / 2);
    let prepared = PreparedGlob::new(&resources, &input).unwrap();
    // Act
    let result = prepared.compile(
        &resources,
        Some(GlobUrlResolution {
            serialized: "$_0_$",
            origin: "$_0_$",
        }),
    );
    // Assert: expanding Unicode lowercasing exceeds the admitted workspace.
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(resources.metrics().unwrap().bytes, 0);
    assert_eq!(resources.metrics().unwrap().revisions, 0);
}
