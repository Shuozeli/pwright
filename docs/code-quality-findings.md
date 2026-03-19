# Code Quality Findings

Audit performed 2026-03-19 against commit d8fbf98. All items resolved.

## 1. Duplication

### gRPC error conversion boilerplate -- DONE
- **Fix applied:** `cdp_to_status()` helper maps CdpError variants to correct gRPC codes. 40+ closures replaced.

### Node ID validation repeated in execute_action match arms -- DONE
- **Fix applied:** `require_node_id!()` macro deduplicates 10 match arms.

### Touch tap implemented three times -- DONE
- **Fix applied:** `Page::tap()` delegates to `Touchscreen::tap()`. Server handler uses `TouchEventType` enums.

### Keyboard down/up branching -- DONE
- **Fix applied:** `dispatch_key()` helper. `down()`/`up()` delegate to it.

### Executor error emission near-duplicate -- DONE
- **Fix applied:** Single `sink.emit()` + conditional `return`.

## 2. Stringly-Typed APIs

### CDP event type strings (mouse/keyboard/touch) -- DONE
- **Fix applied:** Defined `MouseEventType`, `KeyEventType`, `TouchEventType` enums in `pwright-cdp`. Updated `CdpClient` trait, all implementations (CdpSession, MockCdpClient, FakeCdpClient), and all callers across pwright-bridge, pwright-cli, pwright-server, integration tests.

### Mouse button as Option<String> -- DONE
- **Fix applied:** Defined `MouseButton` enum (`Left`, `Right`, `Middle`) in `pwright-cdp`. Updated `ClickOptions.button` to `Option<MouseButton>`, `Mouse::down()`/`up()` signatures, CLI button parsing.

### ExecutionResult.status as String -- DONE
- **Fix applied:** Defined `ExecutionStatus` enum (`Ok`, `Error`) with `Serialize`. Updated `ExecutionResult`, `StepResult`, `ScriptSummary`, and all test assertions across pwright-script, pwright-cli, and integration tests.

## 3. Silent Failures

### Reload ignores poll_ready_state result -- DONE
- **Fix applied:** Logs `tracing::warn!` on failure.

### JsonlSink drops serialization and write errors -- DONE
- **Fix applied:** `emit()` and `write_summary()` now log `tracing::warn!` on serialization or write failure.

### State file permission failure silenced -- DONE
- **Fix applied:** Logs `tracing::debug!` on permission set failure.

## 4. Missing Abstractions

### Proto conversion boilerplate (no From impls)
- **Status:** Deferred -- medium effort, 4 impl blocks needed. Worth doing when adding new proto fields.

### set_cookies uses json!() instead of typed struct -- DONE
- **Fix applied:** Changed `network_set_cookies` signature from `Vec<Value>` to `&[Cookie]`. Server now constructs typed `Cookie` structs. Bridge `set_cookies` accepts `&[Cookie]`.

## 5. Unsafe Patterns

### #[allow(unused_imports)] removed -- DONE

### root_node_id uses unwrap_or(1) fallback -- DONE
- **Fix applied:** Changed return type to `CdpResult<i64>`, returns `CdpError::Other` on parse failure. Updated 3 call sites.

### Numeric cast: f64 to u64 for timeout -- DONE
- **Fix applied:** Added `.max(0.0)` before cast to prevent negative wrapping.

## 6. Dead Code / Stubs

### Placeholder test file
- **Status:** Deferred -- intentional placeholder with clear documentation.

### MockCdpClient missing setters
- **Status:** Deferred -- add when tests need these responses.

## 7. Noise

### Duplicate doc comment -- DONE
