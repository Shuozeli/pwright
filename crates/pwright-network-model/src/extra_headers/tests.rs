use super::*;
use pwright_core::ResourceLimits;

fn profile(owners: usize, headers: usize, bytes: usize) -> HeaderProfile {
    HeaderProfile::new(ResourceScope::extension(128, 0), owners, headers, bytes).unwrap()
}

fn resources(profile: HeaderProfile, doubled: bool) -> ProductionResourceLedger {
    let mut requirements = profile.requirements().unwrap();
    if doubled {
        requirements = requirements.combine_concurrent(requirements);
    }
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let bound = requirements.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    ProductionResourceLedger::new(limits, requirements).unwrap()
}

fn header<'a>(name: &'a str, value: &'a str) -> HeaderRef<'a> {
    HeaderRef { name, value }
}

#[test]
fn page_snapshot_preserves_duplicate_spelling_and_order() {
    // Arrange
    let profile = profile(2, 4, 128);
    let ledger = resources(profile, false);
    let pool = HeaderPool::new(profile, &ledger).unwrap();
    let input = [header("X-A", "first"), header("x-a", "last")];
    // Act
    let page = pool.page(&input).unwrap();
    // Assert
    assert_eq!(page.iter().collect::<Vec<_>>(), input);
    assert_eq!(pool.active_owners(), 1);
}

#[test]
fn merge_uses_last_spelling_and_value_without_moving_first_position() {
    // Arrange
    let profile = profile(3, 4, 128);
    let ledger = resources(profile, false);
    let pool = HeaderPool::new(profile, &ledger).unwrap();
    let context = pool
        .context(&[header("X-A", "context"), header("X-B", "kept")])
        .unwrap();
    let page = pool
        .page(&[
            header("x-a", "first"),
            header("X-C", "new"),
            header("X-a", "last"),
        ])
        .unwrap();
    // Act
    let effective = pool.merge(Some(&context), Some(&page)).unwrap();
    // Assert
    assert_eq!(
        effective.iter().collect::<Vec<_>>(),
        [
            header("X-a", "last"),
            header("X-B", "kept"),
            header("X-C", "new")
        ]
    );
}

#[test]
fn clearing_page_reveals_context_and_clearing_context_keeps_page() {
    for clear_page in [false, true] {
        // Arrange
        let profile = profile(3, 2, 128);
        let ledger = resources(profile, false);
        let pool = HeaderPool::new(profile, &ledger).unwrap();
        let context = pool.context(&[header("X-A", "context")]).unwrap();
        let page = pool.page(&[header("X-A", "page")]).unwrap();
        // Act
        let effective = pool
            .merge(
                clear_page.then_some(&context),
                (!clear_page).then_some(&page),
            )
            .unwrap();
        // Assert
        assert_eq!(
            effective.iter().collect::<Vec<_>>(),
            [header("X-A", if clear_page { "context" } else { "page" })]
        );
    }
}

#[test]
fn navigation_uses_first_page_referer_but_network_merge_uses_last() {
    // Arrange
    let profile = profile(2, 3, 128);
    let ledger = resources(profile, false);
    let pool = HeaderPool::new(profile, &ledger).unwrap();
    let page = pool
        .page(&[header("Referer", "first"), header("referer", "last")])
        .unwrap();
    // Act
    let navigation = page.navigation_referrer(Some("first")).unwrap();
    let effective = pool.merge(None, Some(&page)).unwrap();
    // Assert
    assert_eq!(navigation, Some("first"));
    assert_eq!(
        effective.iter().collect::<Vec<_>>(),
        [header("referer", "last")]
    );
}

#[test]
fn navigation_conflict_includes_empty_page_referer() {
    for value in ["first", ""] {
        // Arrange
        let profile = profile(1, 2, 128);
        let ledger = resources(profile, false);
        let pool = HeaderPool::new(profile, &ledger).unwrap();
        let page = pool
            .page(&[header("Referer", value), header("referer", "last")])
            .unwrap();
        // Act
        let result = page.navigation_referrer(Some("last"));
        // Assert
        assert!(matches!(result, Err(HeaderError::ReferrerConflict)));
    }
}

#[test]
fn absent_navigation_option_preserves_missing_empty_and_nonempty_headers() {
    for value in [None, Some(""), Some("first")] {
        // Arrange
        let profile = profile(1, 1, 128);
        let ledger = resources(profile, false);
        let pool = HeaderPool::new(profile, &ledger).unwrap();
        let input: Vec<_> = value
            .map(|value| header("REFERER", value))
            .into_iter()
            .collect();
        let page = pool.page(&input).unwrap();
        // Act
        let result = page.navigation_referrer(None).unwrap();
        // Assert
        assert_eq!(result, value);
    }
}

#[test]
fn no_page_referer_allows_explicit_navigation_option() {
    // Arrange
    let profile = profile(1, 1, 128);
    let ledger = resources(profile, false);
    let pool = HeaderPool::new(profile, &ledger).unwrap();
    let page = pool.page(&[]).unwrap();
    // Act
    let result = page.navigation_referrer(Some("option")).unwrap();
    // Assert
    assert_eq!(result, Some("option"));
}

#[test]
fn desired_invalid_syntax_is_retained_but_effective_wire_validation_rejects_without_allocation() {
    for (name, value, expected) in [
        ("X Ä", "value", "HTTP header name is not ASCII"),
        ("", "value", "Invalid header name"),
        ("X A", "value", "Invalid header name"),
        ("X-A", "a\nb", "Invalid header value"),
        ("X-A", "a\rb", "Invalid header value"),
        ("X-A", "\u{7f}", "Invalid header value"),
    ] {
        // Arrange
        let profile = profile(2, 2, 128);
        let ledger = resources(profile, false);
        let pool = HeaderPool::new(profile, &ledger).unwrap();
        let page = pool.page(&[header(name, value)]).unwrap();
        let effective = pool.merge(None, Some(&page)).unwrap();
        let before = ledger
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current;
        // Act
        let result = effective.validate_http_syntax();
        // Assert
        assert_eq!(result.unwrap_err().to_string(), expected);
        assert_eq!(pool.active_owners(), 2);
        assert_eq!(page.iter().collect::<Vec<_>>(), [header(name, value)]);
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::BackendStateBytes)
                .current,
            before
        );
    }
}

#[test]
fn final_page_override_and_duplicate_replacement_can_mask_invalid_desired_values() {
    // Arrange
    let profile = profile(3, 2, 128);
    let ledger = resources(profile, false);
    let pool = HeaderPool::new(profile, &ledger).unwrap();
    let context = pool.context(&[header("X-A", "bad\ncontext")]).unwrap();
    let page = pool
        .page(&[header("X-A", "bad\npage"), header("x-a", "valid")])
        .unwrap();
    // Act
    let effective = pool.merge(Some(&context), Some(&page)).unwrap();
    let result = effective.validate_http_syntax();
    // Assert
    assert!(result.is_ok());
    assert_eq!(
        effective.iter().collect::<Vec<_>>(),
        [header("x-a", "valid")]
    );
    assert_eq!(context.iter().next().unwrap().value, "bad\ncontext");
    assert_eq!(page.iter().next().unwrap().value, "bad\npage");
}

#[test]
fn exact_count_and_byte_bounds_accept_tab_and_utf8_values() {
    // Arrange
    let profile = profile(1, 2, 6);
    let ledger = resources(profile, false);
    let pool = HeaderPool::new(profile, &ledger).unwrap();
    // Act
    let page = pool.page(&[header("A", "\té"), header("B", "x")]).unwrap();
    // Assert
    assert_eq!(page.iter().len(), 2);
}

#[test]
fn byte_and_count_overflow_reject_without_owners() {
    for (input, expected) in [
        (
            vec![header("A", "1234")],
            "extra header bytes exceed profile",
        ),
        (
            vec![header("A", ""), header("B", "")],
            "extra header count exceeds profile",
        ),
    ] {
        // Arrange
        let profile = profile(1, 1, 4);
        let ledger = resources(profile, false);
        let pool = HeaderPool::new(profile, &ledger).unwrap();
        // Act
        let result = pool.page(&input);
        // Assert
        assert_eq!(result.unwrap_err().to_string(), expected);
        assert_eq!(pool.active_owners(), 0);
    }
}

#[test]
fn merge_checks_final_bytes_after_later_replacement_shrinks_union() {
    // Arrange
    let profile = profile(3, 2, 10);
    let ledger = resources(profile, false);
    let pool = HeaderPool::new(profile, &ledger).unwrap();
    let context = pool.context(&[header("A", "123456789")]).unwrap();
    let page = pool.page(&[header("B", "x"), header("A", "")]).unwrap();
    // Act
    let effective = pool.merge(Some(&context), Some(&page)).unwrap();
    // Assert
    assert_eq!(
        effective.iter().collect::<Vec<_>>(),
        [header("A", ""), header("B", "x")]
    );
}

#[test]
fn failed_merge_releases_temporary_admission() {
    for (headers, bytes) in [(1, 128), (2, 3)] {
        // Arrange
        let profile = profile(3, headers, bytes);
        let ledger = resources(profile, false);
        let pool = HeaderPool::new(profile, &ledger).unwrap();
        let context = pool.context(&[header("A", "x")]).unwrap();
        let page = pool.page(&[header("B", "y")]).unwrap();
        let before = ledger
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current;
        // Act
        let result = pool.merge(Some(&context), Some(&page));
        // Assert
        assert!(matches!(
            result,
            Err(HeaderError::CountLimit | HeaderError::ByteLimit)
        ));
        assert_eq!(pool.active_owners(), 2);
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::BackendStateBytes)
                .current,
            before
        );
    }
}

#[test]
fn local_owner_limit_cannot_borrow_shared_plan_spare_capacity() {
    // Arrange
    let profile = profile(1, 1, 16);
    let ledger = resources(profile, true);
    let pool = HeaderPool::new(profile, &ledger).unwrap();
    let _page = pool.page(&[]).unwrap();
    // Act
    let result = pool.page(&[]);
    // Assert
    assert!(matches!(result, Err(HeaderError::OwnerLimit)));
    assert_eq!(pool.active_owners(), 1);
}

#[test]
fn failed_shared_ledger_reservation_rolls_back_local_owner_count() {
    // Arrange
    let profile = profile(1, 1, 16);
    let ledger = resources(profile, false);
    let other = ledger.claim(profile.requirements().unwrap()).unwrap();
    let pool = HeaderPool::new(profile, &ledger).unwrap();
    let _occupied = other
        .reserve(profile.scope, ResourceClass::LiveObject, 1)
        .unwrap();
    // Act
    let result = pool.page(&[]);
    // Assert
    assert!(matches!(result, Err(HeaderError::Budget(_))));
    assert_eq!(pool.active_owners(), 0);
}

#[test]
fn cloned_snapshot_outlives_pool_and_releases_all_charges_on_final_drop() {
    // Arrange
    let profile = profile(1, 1, 16);
    let ledger = resources(profile, false);
    let pool = HeaderPool::new(profile, &ledger).unwrap();
    let page = pool.page(&[header("A", "secret")]).unwrap();
    let retained = page.clone();
    drop(page);
    drop(pool);
    assert_eq!(retained.iter().collect::<Vec<_>>(), [header("A", "secret")]);
    assert_eq!(
        ledger
            .metrics()
            .unwrap()
            .class(ResourceClass::LiveObject)
            .current,
        1
    );
    // Act
    drop(retained);
    // Assert
    for class in ResourceClass::ALL {
        assert_eq!(ledger.metrics().unwrap().class(class).current, 0);
    }
}

#[test]
fn debug_output_does_not_expose_header_secrets() {
    // Arrange
    let profile = profile(1, 1, 64);
    let ledger = resources(profile, false);
    let pool = HeaderPool::new(profile, &ledger).unwrap();
    let input = header("Authorization", "sensitive-token");
    let page = pool.page(&[input]).unwrap();
    // Act
    let debug = format!("{page:?} {input:?}");
    // Assert
    assert!(!debug.contains("sensitive-token"));
    assert!(!debug.contains("Authorization"));
}

#[test]
fn overflowing_and_zero_profiles_are_rejected() {
    for (owners, headers, bytes) in [
        (0, 1, 1),
        (1, 0, 1),
        (1, 1, 0),
        (usize::MAX, 1, 1),
        (1, usize::MAX, 1),
        (1, 1, usize::MAX),
    ] {
        // Arrange
        let scope = ResourceScope::extension(128, 0);
        // Act
        let result = HeaderProfile::new(scope, owners, headers, bytes);
        // Assert
        assert!(matches!(result, Err(HeaderError::InvalidLimits)));
    }
}
