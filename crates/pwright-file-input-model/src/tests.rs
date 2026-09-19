use super::*;

fn limits() -> FileInputLimits {
    FileInputLimits::new(8, 128, 64, 64, 256).unwrap()
}
fn payload(bytes: u64) -> PayloadMetadata<'static> {
    PayloadMetadata {
        name: "sample.txt",
        mime_type: "text/plain",
        byte_length: bytes,
    }
}
fn file(bytes: u64) -> ResolvedPathMetadata<'static> {
    ResolvedPathMetadata {
        path: "/fixtures/sample.txt",
        kind: PathKind::File { byte_length: bytes },
    }
}
fn directory() -> ResolvedPathMetadata<'static> {
    ResolvedPathMetadata {
        path: "/fixtures/directory",
        kind: PathKind::Directory,
    }
}
fn target(multiple: bool, directory: bool) -> InputTarget {
    InputTarget {
        is_input: true,
        multiple,
        directory,
    }
}

#[test]
fn empty_input_selects_payload_clearing() {
    // Arrange
    let limits = limits();
    // Act
    let shape = limits.classify(&[]);
    // Assert
    assert_eq!(shape, Ok(InputShape::Payloads));
}

#[test]
fn mixed_paths_and_payloads_are_rejected_before_materialization() {
    // Arrange
    let items = [InputItemKind::Payload, InputItemKind::Path];
    // Act
    let result = limits().classify(&items);
    // Assert
    assert_eq!(result, Err(FileInputError::MixedPathsAndPayloads));
}

#[test]
fn homogeneous_paths_select_path_resolution() {
    // Arrange
    let items = [InputItemKind::Path; 2];
    // Act
    let result = limits().classify(&items);
    // Assert
    assert_eq!(result, Ok(InputShape::Paths));
}

#[test]
fn item_capacity_applies_before_shape_inspection() {
    // Arrange
    let items = [InputItemKind::Payload; 9];
    // Act
    let result = limits().classify(&items);
    // Assert
    assert_eq!(result, Err(FileInputError::CountLimit));
}

#[test]
fn exact_fifty_mib_payload_is_rejected_without_allocating_a_buffer() {
    // Arrange
    let items = [payload(UPLOAD_SIZE_LIMIT)];
    // Act
    let result = limits().payloads(&items);
    // Assert
    assert_eq!(result, Err(FileInputError::PayloadSizeLimit));
}

#[test]
fn one_byte_below_fifty_mib_is_admissible_metadata() {
    // Arrange
    let items = [payload(UPLOAD_SIZE_LIMIT - 1)];
    // Act
    let result = limits().payloads(&items).unwrap();
    // Assert
    assert_eq!(result.source_bytes(), UPLOAD_SIZE_LIMIT - 1);
    assert_eq!(result.route(), UploadRoute::Payloads);
    assert_eq!(result.items(), 1);
}

#[test]
fn payload_limit_is_aggregate_not_per_file() {
    // Arrange
    let items = [payload(UPLOAD_SIZE_LIMIT - 1), payload(1)];
    // Act
    let result = limits().payloads(&items);
    // Assert
    assert_eq!(result, Err(FileInputError::PayloadSizeLimit));
}

#[test]
fn enormous_size_cannot_overflow_into_an_accepted_payload() {
    // Arrange
    let items = [payload(1), payload(u64::MAX)];
    // Act
    let result = limits().payloads(&items);
    // Assert
    assert_eq!(result, Err(FileInputError::PayloadSizeLimit));
}

#[test]
fn each_base64_payload_gets_its_own_padding() {
    // Arrange
    let items = [payload(1), payload(1), payload(0)];
    // Act
    let result = limits().payloads(&items).unwrap();
    // Assert
    assert_eq!(result.source_bytes(), 2);
    assert_eq!(result.base64_bytes(), 8);
    assert_eq!(result.metadata_bytes(), 60);
}

#[test]
fn payloads_are_bounded_even_without_source_bytes() {
    // Arrange
    let limits = FileInputLimits::new(8, 128, 64, 64, 39).unwrap();
    let items = [payload(0), payload(0)];
    // Act
    let result = limits.payloads(&items);
    // Assert
    assert_eq!(result, Err(FileInputError::MetadataByteLimit));
}

#[test]
fn multibyte_names_are_measured_as_bytes() {
    // Arrange
    let limits = FileInputLimits::new(8, 128, 5, 64, 256).unwrap();
    let items = [PayloadMetadata {
        name: "文件",
        ..payload(1)
    }];
    // Act
    let result = limits.payloads(&items);
    // Assert
    assert_eq!(result, Err(FileInputError::NameByteLimit));
}

#[test]
fn mime_type_has_an_independent_bound() {
    // Arrange
    let limits = FileInputLimits::new(8, 128, 64, 4, 256).unwrap();
    // Act
    let result = limits.payloads(&[payload(1)]);
    // Assert
    assert_eq!(result, Err(FileInputError::MimeByteLimit));
}

#[test]
fn empty_names_and_mime_strings_remain_representable() {
    // Arrange
    let items = [PayloadMetadata {
        name: "",
        mime_type: "",
        byte_length: 0,
    }];
    // Act
    let result = limits().payloads(&items).unwrap();
    // Assert
    assert_eq!(result.items(), 1);
    assert_eq!(result.metadata_bytes(), 0);
}

#[test]
fn separate_browser_reads_files_instead_of_sending_caller_paths() {
    // Arrange
    let items = [file(4)];
    // Act
    let result = limits().paths(&items, BrowserLocation::Separate).unwrap();
    // Assert
    assert_eq!(result.route(), UploadRoute::ReadFiles);
    assert_eq!(result.source_bytes(), 4);
    assert_eq!(result.base64_bytes(), 8);
}

#[test]
fn collocated_browser_does_not_admit_file_content_buffers() {
    // Arrange
    let items = [file(u64::MAX), file(u64::MAX)];
    // Act
    let result = limits().paths(&items, BrowserLocation::Colocated).unwrap();
    // Assert
    assert_eq!(result.route(), UploadRoute::NativePaths);
    assert_eq!(result.source_bytes(), 0);
    assert_eq!(result.base64_bytes(), 0);
}

#[test]
fn separate_browser_rejects_exact_transfer_size_limit() {
    // Arrange
    let items = [file(UPLOAD_SIZE_LIMIT)];
    // Act
    let result = limits().paths(&items, BrowserLocation::Separate);
    // Assert
    assert_eq!(result, Err(FileInputError::RemoteFileSizeLimit));
}

#[test]
fn directory_stays_a_native_path_for_separate_browser() {
    // Arrange
    let items = [directory()];
    // Act
    let result = limits().paths(&items, BrowserLocation::Separate).unwrap();
    // Assert
    assert_eq!(result.route(), UploadRoute::NativeDirectory);
    assert_eq!(result.source_bytes(), 0);
}

#[test]
fn multiple_directories_are_rejected() {
    // Arrange
    let items = [directory(), directory()];
    // Act
    let result = limits().paths(&items, BrowserLocation::Colocated);
    // Assert
    assert_eq!(result, Err(FileInputError::MultipleDirectories));
}

#[test]
fn directory_mixing_precedes_remote_transfer_size_validation() {
    // Arrange
    let items = [file(u64::MAX), directory()];
    // Act
    let result = limits().paths(&items, BrowserLocation::Separate);
    // Assert
    assert_eq!(result, Err(FileInputError::MixedFilesAndDirectory));
}

#[test]
fn second_directory_precedes_final_files_and_directory_mixing_error() {
    // Arrange
    let items = [file(0), directory(), directory()];
    // Act
    let result = limits().paths(&items, BrowserLocation::Separate);
    // Assert
    assert_eq!(result, Err(FileInputError::MultipleDirectories));
}

#[test]
fn path_bytes_are_bounded_without_filesystem_access() {
    // Arrange
    let limits = FileInputLimits::new(8, 4, 64, 64, 256).unwrap();
    // Act
    let result = limits.paths(&[file(0)], BrowserLocation::Colocated);
    // Assert
    assert_eq!(result, Err(FileInputError::PathByteLimit));
}

#[test]
fn empty_resolved_path_list_has_clear_semantics() {
    // Arrange
    let limits = limits();
    // Act
    let result = limits.paths(&[], BrowserLocation::Colocated).unwrap();
    // Assert
    assert_eq!(result.route(), UploadRoute::Payloads);
    assert_eq!(result.items(), 0);
}

#[test]
fn non_input_rejection_precedes_multiple_file_validation() {
    // Arrange
    let plan = limits().payloads(&[payload(0), payload(0)]).unwrap();
    let target = InputTarget {
        is_input: false,
        multiple: false,
        directory: false,
    };
    // Act
    let result = plan.validate_target(target);
    // Assert
    assert_eq!(result, Err(FileInputError::NotInput));
}

#[test]
fn single_file_input_rejects_multiple_payloads() {
    // Arrange
    let plan = limits().payloads(&[payload(0), payload(0)]).unwrap();
    // Act
    let result = plan.validate_target(target(false, false));
    // Assert
    assert_eq!(result, Err(FileInputError::MultipleFiles));
}

#[test]
fn multiple_input_accepts_multiple_payloads() {
    // Arrange
    let plan = limits().payloads(&[payload(0), payload(0)]).unwrap();
    // Act
    let result = plan.validate_target(target(true, false));
    // Assert
    assert_eq!(result, Ok(()));
}

#[test]
fn directory_requires_webkitdirectory_attribute() {
    // Arrange
    let plan = limits()
        .paths(&[directory()], BrowserLocation::Separate)
        .unwrap();
    // Act
    let result = plan.validate_target(target(true, false));
    // Assert
    assert_eq!(result, Err(FileInputError::DirectoryNotSupported));
}

#[test]
fn directory_target_rejects_regular_file_lists() {
    // Arrange
    let plan = limits()
        .paths(&[file(0)], BrowserLocation::Separate)
        .unwrap();
    // Act
    let result = plan.validate_target(target(false, true));
    // Assert
    assert_eq!(result, Err(FileInputError::DirectoryRequired));
}

#[test]
fn empty_clear_does_not_bypass_directory_target_validation() {
    // Arrange
    let plan = limits().payloads(&[]).unwrap();
    // Act
    let result = plan.validate_target(target(false, true));
    // Assert
    assert_eq!(result, Err(FileInputError::DirectoryRequired));
}

#[test]
fn zero_limits_do_not_create_an_implicit_fallback() {
    // Arrange
    let item_limit = 0;
    // Act
    let result = FileInputLimits::new(item_limit, 1, 1, 1, 1);
    // Assert
    assert!(matches!(result, Err(FileInputError::InvalidLimits)));
}
