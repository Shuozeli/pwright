"""Mutation controls for the independent five-case Frame navigation verifier."""

from __future__ import annotations

import hashlib
import json
import xml.etree.ElementTree as ET
import zipfile
from pathlib import Path

import pytest
import verify_playwright_python_navigation_frame_strict_paired_v2 as verify


def _line(row: dict[str, object]) -> bytes:
    return (json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n").encode()


def _fixture() -> tuple[
    list[bytes], list[bytes], list[dict[str, object]], dict[str, dict[str, object]]
]:
    inventory = [
        {
            "id": "tests/async/test_other.py::test_before[chromium]",
            "sourceClassification": "public-api-source",
        },
        {
            "id": "tests/async/test_navigation.py::test_frame[chromium]",
            "sourceClassification": "public-api-source",
        },
        {
            "id": "tests/async/test_other.py::test_after[chromium]",
            "sourceClassification": "public-api-source",
        },
    ]
    base = [
        _line({**inventory[0], "outcome": "passed", "attempts": 1}),
        _line(
            {**inventory[1], "outcome": "failed", "attempts": 1, "error": "old failure"}
        ),
        _line({**inventory[2], "outcome": "not-run", "attempts": 0}),
    ]
    expected = {str(inventory[1]["id"]): {"outcome": "passed", "durationSeconds": 0.25}}
    successor = list(base)
    successor[1] = _line(
        {
            **inventory[1],
            **expected[str(inventory[1]["id"])],
            "attempts": 1,
            "resultFile": "/evidence/focused.xml",
        }
    )
    return base, successor, inventory, expected


def _check(
    base: list[bytes],
    successor: list[bytes],
    inventory: list[dict[str, object]],
    expected: dict[str, dict[str, object]],
    mode: str = "candidate",
) -> object:
    return verify.check_reconciliation(
        base, successor, inventory, [1], expected, "/evidence/focused.xml", mode
    )


def test_candidate_exact_reconciliation_is_accepted() -> None:
    # Arrange
    base, successor, inventory, expected = _fixture()

    # Act
    counts = _check(base, successor, inventory, expected)

    # Assert
    assert counts == {"passed": 2, "not-run": 1}


def test_oracle_all_raw_rows_are_accepted() -> None:
    # Arrange
    _, _, inventory, expected = _fixture()
    base = [_line({**row, "outcome": "passed", "attempts": 1}) for row in inventory]

    # Act
    counts = _check(base, list(base), inventory, expected, "oracle")

    # Assert
    assert counts == {"passed": 3}


@pytest.mark.parametrize(
    "mutation",
    (
        "outcome",
        "duration",
        "attempts",
        "result-file",
        "extra-error",
        "missing-id",
        "duplicate-id",
        "off-target-byte",
        "target-byte",
        "base-not-failed",
        "base-attempts",
        "source-classification",
    ),
)
def test_candidate_mutation_is_rejected(mutation: str) -> None:
    # Arrange
    base, successor, inventory, expected = _fixture()
    target = json.loads(successor[1])
    if mutation == "outcome":
        target["outcome"] = "failed"
    elif mutation == "duration":
        target["durationSeconds"] = 0.5
    elif mutation == "attempts":
        target["attempts"] = 2
    elif mutation == "result-file":
        target["resultFile"] = "/other/result.xml"
    elif mutation == "extra-error":
        target["error"] = "fabricated failure"
    elif mutation == "missing-id":
        del target["id"]
    elif mutation == "duplicate-id":
        target["id"] = inventory[0]["id"]
    elif mutation == "off-target-byte":
        successor[0] = b" " + successor[0]
    elif mutation == "target-byte":
        successor[1] = b" " + successor[1]
    elif mutation == "base-not-failed":
        before = json.loads(base[1])
        before["outcome"] = "passed"
        base[1] = _line(before)
    elif mutation == "base-attempts":
        before = json.loads(base[1])
        before["attempts"] = 2
        base[1] = _line(before)
    elif mutation == "source-classification":
        inventory[1]["sourceClassification"] = "private-api-source"
        before = json.loads(base[1])
        before["sourceClassification"] = "private-api-source"
        base[1] = _line(before)
        target["sourceClassification"] = "private-api-source"
    if mutation not in {"off-target-byte", "target-byte"}:
        successor[1] = _line(target)

    # Act / Assert
    with pytest.raises(verify.FrameNavigationValidationError):
        _check(base, successor, inventory, expected)


@pytest.mark.parametrize("index", (0, 1, 2))
def test_oracle_any_raw_row_change_is_rejected(index: int) -> None:
    # Arrange
    _, _, inventory, expected = _fixture()
    base = [_line({**row, "outcome": "passed", "attempts": 1}) for row in inventory]
    successor = list(base)
    successor[index] = b" " + successor[index]

    # Act / Assert
    with pytest.raises(verify.FrameNavigationValidationError):
        _check(base, successor, inventory, expected, "oracle")


def test_missing_focused_case_is_rejected() -> None:
    # Arrange
    base, successor, inventory, _ = _fixture()

    # Act / Assert
    with pytest.raises(verify.FrameNavigationValidationError):
        _check(base, successor, inventory, {})


def test_duplicate_target_index_is_rejected() -> None:
    # Arrange
    base, successor, inventory, expected = _fixture()

    # Act / Assert
    with pytest.raises(verify.FrameNavigationValidationError):
        verify.check_reconciliation(
            base,
            successor,
            inventory,
            [1, 1],
            expected,
            "/evidence/focused.xml",
            "candidate",
        )


def _scope() -> dict[str, object]:
    return {
        "schema": 2,
        "kind": verify.KIND,
        "scope": verify.SCOPE,
        "publicApiParityEvidence": False,
        "fullCorpusExecution": False,
        "completeFileExecution": False,
        "fixtureAdapted": True,
        "launchMode": "cdp-adapter",
        "chromeVersionObserved": "155.0.8059.85",
    }


def test_exact_bounded_scope_is_accepted() -> None:
    # Arrange
    receipt = _scope()

    # Act
    result = verify.check_receipt_scope(receipt)

    # Assert
    assert result is None


@pytest.mark.parametrize(
    ("field", "value"),
    (
        ("schema", 1),
        ("kind", "other"),
        ("scope", "all-python"),
        ("publicApiParityEvidence", True),
        ("fullCorpusExecution", True),
        ("completeFileExecution", True),
        ("fixtureAdapted", False),
        ("launchMode", "native"),
        ("chromeVersionObserved", "other"),
    ),
)
def test_false_scope_claim_is_rejected(field: str, value: object) -> None:
    # Arrange
    receipt = {**_scope(), field: value}

    # Act / Assert
    with pytest.raises(verify.FrameNavigationValidationError):
        verify.check_receipt_scope(receipt)


def _report_metadata() -> dict[str, object]:
    return {
        "schema": 5,
        "mode": "candidate",
        "commit": verify.COMMIT,
        "manifest": str(verify.MANIFEST),
        "manifestSha256": verify.MANIFEST_SHA256,
        "upstream": str(verify.UPSTREAM),
        "inventory": str(verify.INVENTORY),
        "inventorySha256": verify.INVENTORY_SHA256,
        "scope": "selected",
        "endpoint": "http://127.0.0.1:9222",
        "fixtureBind": "100.78.97.63",
        "fixtureHost": "macbookpro-yuacx.tail8f3b66.ts.net",
        "result": str(verify.RESULT),
        "resultSha256": verify.RESULT_SHA256,
        "caseReports": str(Path(f"{verify.RESULT}.cases")),
        "caseReportsSha256": verify.CASE_REPORTS_SHA256,
        "caseTimeout": 45.0,
        "isolatedCaseIdsSha256": verify.ISOLATED_IDS_SHA256,
        "isolatedCaseCount": 5,
        "selections": list(verify.REPORT_SELECTIONS),
        "browsers": ["chromium"],
        "launchMode": "cdp-adapter",
        "isolateCases": True,
        "resumeIsolated": False,
        "collectOnly": False,
        "bootstrapOnly": False,
        "manifestCases": False,
        "secureHttpFixture": False,
        "exitCode": 0,
        "pythonExecutable": str(verify.PYTHON_EXECUTABLE),
        "pwrightPackage": str(verify.CANDIDATE_PACKAGE),
        "pwrightArtifact": str(verify.CANDIDATE_ARTIFACT),
        "pwrightArtifactSha256": verify.PWRIGHT_ARTIFACT_SHA256,
        "pwrightPackageSha256": verify.PWRIGHT_PACKAGE_SHA256,
        "playwrightPackageSha256": "1" * 64,
        "harnessInputs": [{"path": "/evidence/harness", "sha256": "2" * 64}],
        "toolingSources": [{"path": "scripts/tool.py", "sha256": "3" * 64}],
        "pwrightRuntimeLibraries": [
            {"path": "/evidence/lib.dylib", "sha256": "4" * 64}
        ],
    }


def test_focused_report_metadata_is_accepted() -> None:
    # Arrange
    payload = _report_metadata()

    # Act
    result = verify.check_report_metadata(payload)

    # Assert
    assert result is None


@pytest.mark.parametrize(
    ("field", "value"),
    (
        ("mode", "oracle"),
        ("endpoint", "http://0.0.0.0:9222"),
        ("fixtureHost", "wrong-host"),
        ("resultSha256", "0" * 64),
        ("caseReports", "/wrong/cases"),
        ("caseReportsSha256", "0" * 64),
        ("isolatedCaseIdsSha256", "0" * 64),
        ("isolatedCaseCount", 80),
        ("selections", list(verify.TARGET_IDS)),
        ("browsers", ["firefox"]),
        ("isolateCases", False),
        ("resumeIsolated", True),
        ("exitCode", 1),
        ("pwrightPackage", "/mutable/venv/pwright"),
        ("pwrightArtifactSha256", "0" * 64),
        ("pwrightPackageSha256", "0" * 64),
    ),
)
def test_focused_report_metadata_mutation_is_rejected(
    field: str, value: object
) -> None:
    # Arrange
    payload = {**_report_metadata(), field: value}

    # Act / Assert
    with pytest.raises(verify.FrameNavigationValidationError):
        verify.check_report_metadata(payload)


def _source_pins(provenance: dict[str, object]) -> dict[str, object]:
    return {
        "schema": 1,
        "kind": "playwright-python-navigation-frame-five-candidate",
        "upstreamCommit": verify.COMMIT,
        "upstreamNavigationSha256": verify.NAVIGATION_SHA256,
        "officialManifestSha256": verify.MANIFEST_SHA256,
        "profiledInventorySha256": verify.INVENTORY_SHA256,
        "frozenWheel": str(verify.WHEEL),
        "frozenWheelSha256": verify.WHEEL_SHA256,
        "wheelName": verify.WHEEL.name,
        "installedNativeAbi": 128,
        "pwrightArtifactSha256": verify.PWRIGHT_ARTIFACT_SHA256,
        "pwrightPackageSha256": verify.PWRIGHT_PACKAGE_SHA256,
        "playwrightPackageSha256": provenance["playwrightPackageSha256"],
        "harnessInputs": provenance["harnessInputs"],
        "toolingSources": provenance["toolingSources"],
        "pwrightRuntimeLibraries": provenance["pwrightRuntimeLibraries"],
    }


def test_exact_source_pins_are_accepted() -> None:
    # Arrange
    provenance = _report_metadata()
    pins = _source_pins(provenance)

    # Act
    result = verify.check_source_pins(pins, provenance)

    # Assert
    assert result is None


@pytest.mark.parametrize(
    ("field", "value"),
    (
        ("schema", True),
        ("schema", 2),
        ("upstreamNavigationSha256", "0" * 64),
        ("frozenWheel", "/wrong/wheel.whl"),
        ("frozenWheelSha256", "0" * 64),
        ("wheelName", "wrong.whl"),
        ("installedNativeAbi", 127),
        ("pwrightPackageSha256", "0" * 64),
        ("pwrightArtifactSha256", "0" * 64),
        ("playwrightPackageSha256", "0" * 64),
        ("harnessInputs", []),
        ("toolingSources", []),
        ("pwrightRuntimeLibraries", []),
    ),
)
def test_source_pin_mutation_is_rejected(field: str, value: object) -> None:
    # Arrange
    provenance = _report_metadata()
    pins = {**_source_pins(provenance), field: value}

    # Act / Assert
    with pytest.raises(verify.FrameNavigationValidationError):
        verify.check_source_pins(pins, provenance)


def test_extra_source_pin_claim_is_rejected() -> None:
    # Arrange
    provenance = _report_metadata()
    pins = {**_source_pins(provenance), "fullCorpusExecution": True}

    # Act / Assert
    with pytest.raises(verify.FrameNavigationValidationError):
        verify.check_source_pins(pins, provenance)


def _wheel_fixture(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> dict[str, object]:
    package = tmp_path / "installed" / "pwright"
    package.mkdir(parents=True)
    native = b"frozen native artifact"
    facade = b"async def goto(): pass\n"
    (package / "_native.abi3.so").write_bytes(native)
    (package / "async_api.py").write_bytes(facade)
    wheel = tmp_path / "candidate.whl"
    with zipfile.ZipFile(wheel, "w") as archive:
        archive.writestr("pwright/_native.abi3.so", native)
        archive.writestr("pwright/async_api.py", facade)
    monkeypatch.setattr(verify, "WHEEL", wheel)
    monkeypatch.setattr(
        verify, "WHEEL_SHA256", hashlib.sha256(wheel.read_bytes()).hexdigest()
    )
    monkeypatch.setattr(
        verify, "PWRIGHT_ARTIFACT_SHA256", hashlib.sha256(native).hexdigest()
    )
    return {
        "pwrightPackage": str(package),
        "pwrightArtifactSha256": hashlib.sha256(native).hexdigest(),
    }


def test_wheel_and_reported_package_bytes_are_linked(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    provenance = _wheel_fixture(tmp_path, monkeypatch)

    # Act
    result = verify._verify_wheel(provenance)

    # Assert
    assert result is None


@pytest.mark.parametrize(
    "mutation", ("installed-facade", "reported-native", "wheel-byte")
)
def test_wheel_or_report_byte_mutation_is_rejected(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, mutation: str
) -> None:
    # Arrange
    provenance = _wheel_fixture(tmp_path, monkeypatch)
    if mutation == "installed-facade":
        (Path(str(provenance["pwrightPackage"])) / "async_api.py").write_bytes(
            b"async def goto(): return None\n"
        )
    elif mutation == "reported-native":
        provenance["pwrightArtifactSha256"] = "0" * 64
    elif mutation == "wheel-byte":
        with zipfile.ZipFile(verify.WHEEL, "a") as archive:
            archive.writestr("tampered.txt", b"changed")

    # Act / Assert
    with pytest.raises(verify.FrameNavigationValidationError):
        verify._verify_wheel(provenance)


def _focused_xml() -> bytes:
    suite = ET.Element("testsuite")
    for test_id in verify.TARGET_IDS:
        ET.SubElement(
            suite,
            "testcase",
            classname="tests.async.test_navigation",
            name=test_id.split("::", 1)[1],
            time="0.25",
        )
    return ET.tostring(suite)


def test_exact_focused_xml_is_accepted() -> None:
    # Arrange
    raw = _focused_xml()

    # Act
    cases = verify._junit_cases(raw)

    # Assert
    assert tuple(cases) == verify.TARGET_IDS
    assert all(case["outcome"] == "passed" for case in cases.values())


@pytest.mark.parametrize(
    "mutation",
    (
        "missing",
        "duplicate",
        "outside",
        "failure",
        "error",
        "skipped",
        "nan-duration",
        "order",
    ),
)
def test_focused_xml_mutation_is_rejected(mutation: str) -> None:
    # Arrange
    suite = ET.fromstring(_focused_xml())
    cases = suite.findall("testcase")
    if mutation == "missing":
        suite.remove(cases[0])
    elif mutation == "duplicate":
        cases[0].set("name", cases[1].get("name", ""))
    elif mutation == "outside":
        cases[0].set("classname", "tests.async.test_other")
    elif mutation in {"failure", "error", "skipped"}:
        ET.SubElement(cases[0], mutation)
    elif mutation == "nan-duration":
        cases[0].set("time", "nan")
    elif mutation == "order":
        suite.remove(cases[0])
        suite.append(cases[0])

    # Act / Assert
    with pytest.raises(verify.FrameNavigationValidationError):
        verify._junit_cases(ET.tostring(suite))


def test_sealed_source_pin_is_accepted(tmp_path: Path) -> None:
    # Arrange
    raw = b'{"wheelSha256":"fixed-fixture"}\n'
    path = tmp_path / "source-pins.json"
    path.write_bytes(raw)
    expected = hashlib.sha256(raw).hexdigest()

    # Act
    observed = verify.sealed(path, expected, "source pins")

    # Assert
    assert observed == raw


@pytest.mark.parametrize("mutation", ("bytes", "symlink", "invalid-pin"))
def test_sealed_source_pin_mutation_is_rejected(tmp_path: Path, mutation: str) -> None:
    # Arrange
    raw = b'{"wheelSha256":"fixed-fixture"}\n'
    path = tmp_path / "source-pins.json"
    expected = hashlib.sha256(raw).hexdigest()
    if mutation == "bytes":
        path.write_bytes(raw + b" ")
    elif mutation == "symlink":
        source = tmp_path / "actual.json"
        source.write_bytes(raw)
        path.symlink_to(source)
    elif mutation == "invalid-pin":
        path.write_bytes(raw)
        expected = "unsealed"

    # Act / Assert
    with pytest.raises(verify.FrameNavigationValidationError):
        verify.sealed(path, expected, "source pins")
