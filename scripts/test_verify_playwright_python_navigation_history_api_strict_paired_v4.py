"""Mutation controls for the independent one-case history-API v4 verifier."""

from __future__ import annotations

import hashlib
import json
import xml.etree.ElementTree as ET
import zipfile
from pathlib import Path

import pytest
import verify_playwright_python_navigation_history_api_strict_paired_v4 as verify


def _line(row: dict[str, object]) -> bytes:
    return (json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n").encode()


def _ledger_fixture() -> tuple[list[bytes], list[bytes], list[dict[str, object]]]:
    inventory = [
        {
            "id": "tests/async/test_other.py::before[chromium]",
            "sourceClassification": "public-api-source",
        },
        {"id": verify.TARGET_ID, "sourceClassification": "public-api-source"},
        {
            "id": "tests/async/test_navigation.py::synthetic_error[chromium]",
            "sourceClassification": "public-api-source",
        },
    ]
    base = [
        _line({**inventory[0], "outcome": "not-run", "attempts": 0}),
        _line(
            {**inventory[1], "outcome": "failed", "attempts": 1, "error": "old failure"}
        ),
        _line(
            {
                **inventory[2],
                "outcome": "error",
                "attempts": 1,
                "error": "synthetic report error",
            }
        ),
    ]
    successor = list(base)
    successor[1] = _line(
        {
            **inventory[1],
            "outcome": "passed",
            "attempts": 1,
            "durationSeconds": 1.0,
            "resultFile": "/evidence/history-api.xml",
        }
    )
    return base, successor, inventory


def _check(
    base: list[bytes],
    successor: list[bytes],
    inventory: list[dict[str, object]],
    mode: str = "candidate",
) -> object:
    return verify.check_reconciliation(
        base,
        successor,
        inventory,
        1,
        verify.TARGET_ID,
        1.0,
        "/evidence/history-api.xml",
        mode,
    )


def test_candidate_changes_only_history_api_target() -> None:
    # Arrange
    base, successor, inventory = _ledger_fixture()

    # Act
    counts = _check(base, successor, inventory)

    # Assert
    assert counts == {"not-run": 1, "passed": 1, "error": 1}
    assert successor[2] == base[2]


@pytest.mark.parametrize(
    "mutation",
    (
        "target-still-failed",
        "target-extra-error",
        "duration",
        "attempts",
        "result-file",
        "off-target-byte",
        "synthetic-error-byte",
        "target-byte",
        "base-not-failed",
        "base-attempts",
        "base-error-missing",
        "source-classification",
        "target-id",
    ),
)
def test_candidate_mutation_is_rejected(mutation: str) -> None:
    # Arrange
    base, successor, inventory = _ledger_fixture()
    target = json.loads(successor[1])
    if mutation == "target-still-failed":
        target["outcome"] = "failed"
        target["error"] = "old failure"
    elif mutation == "target-extra-error":
        target["error"] = "fabricated"
    elif mutation == "duration":
        target["durationSeconds"] = 1.5
    elif mutation == "attempts":
        target["attempts"] = 2
    elif mutation == "result-file":
        target["resultFile"] = "/wrong.xml"
    elif mutation == "off-target-byte":
        successor[0] = b" " + successor[0]
    elif mutation == "synthetic-error-byte":
        successor[2] = b" " + successor[2]
    elif mutation == "target-byte":
        successor[1] = b" " + successor[1]
    elif mutation in {"base-not-failed", "base-attempts", "base-error-missing"}:
        before = json.loads(base[1])
        if mutation == "base-not-failed":
            before["outcome"] = "passed"
        elif mutation == "base-attempts":
            before["attempts"] = 2
        else:
            before.pop("error")
        base[1] = _line(before)
    elif mutation == "source-classification":
        inventory[1]["sourceClassification"] = "private-api-source"
        before = json.loads(base[1])
        before["sourceClassification"] = "private-api-source"
        base[1] = _line(before)
        target["sourceClassification"] = "private-api-source"
    elif mutation == "target-id":
        target["id"] = "tests/async/test_navigation.py::wrong[chromium]"
    if mutation not in {"off-target-byte", "synthetic-error-byte", "target-byte"}:
        successor[1] = _line(target)

    # Act / Assert
    with pytest.raises(verify.HistoryApiValidationError):
        _check(base, successor, inventory)


@pytest.mark.parametrize("index", (0, 1, 2))
def test_oracle_any_raw_row_mutation_is_rejected(index: int) -> None:
    # Arrange
    _, _, inventory = _ledger_fixture()
    base = [_line({**row, "outcome": "passed", "attempts": 1}) for row in inventory]
    successor = list(base)
    successor[index] = b" " + successor[index]

    # Act / Assert
    with pytest.raises(verify.HistoryApiValidationError):
        _check(base, successor, inventory, "oracle")


def test_oracle_exact_raw_rows_are_accepted() -> None:
    # Arrange
    _, _, inventory = _ledger_fixture()
    base = [_line({**row, "outcome": "passed", "attempts": 1}) for row in inventory]

    # Act
    counts = _check(base, list(base), inventory, "oracle")

    # Assert
    assert counts == {"passed": 3}


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


def test_one_case_bounded_scope_is_accepted() -> None:
    # Arrange
    receipt = _scope()

    # Act
    result = verify.check_receipt_scope(receipt)

    # Assert
    assert result is None


@pytest.mark.parametrize(
    ("field", "value"),
    (
        ("schema", True),
        ("schema", 1),
        ("kind", "wrong"),
        ("scope", "full-file-80"),
        ("publicApiParityEvidence", True),
        ("fullCorpusExecution", True),
        ("completeFileExecution", True),
        ("fixtureAdapted", False),
        ("launchMode", "native"),
        ("chromeVersionObserved", "unknown"),
    ),
)
def test_false_scope_claim_is_rejected(field: str, value: object) -> None:
    # Arrange
    receipt = {**_scope(), field: value}

    # Act / Assert
    with pytest.raises(verify.HistoryApiValidationError):
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
        "isolatedCaseCount": 1,
        "selections": [verify.TARGET_ID],
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
        "toolingSources": [{"path": "/evidence/tool.py", "sha256": "3" * 64}],
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
        ("caseReportsSha256", "0" * 64),
        ("isolatedCaseIdsSha256", "0" * 64),
        ("isolatedCaseCount", 80),
        ("selections", ["tests/async/test_navigation.py"]),
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
    with pytest.raises(verify.HistoryApiValidationError):
        verify.check_report_metadata(payload)


def _source_pins(provenance: dict[str, object]) -> dict[str, object]:
    return {
        "schema": 1,
        "kind": "playwright-python-navigation-history-api-candidate-v4",
        "upstreamCommit": verify.COMMIT,
        "upstreamNavigationSha256": verify.NAVIGATION_SHA256,
        "officialManifestSha256": verify.MANIFEST_SHA256,
        "profiledInventorySha256": verify.INVENTORY_SHA256,
        "targetTestId": verify.TARGET_ID,
        "targetZeroBasedIndex": verify.TARGET_INDEX,
        "frozenWheel": str(verify.WHEEL),
        "frozenWheelSha256": verify.WHEEL_SHA256,
        "wheelName": verify.WHEEL.name,
        "installedNativeAbi": 129,
        "pwrightArtifactSha256": verify.PWRIGHT_ARTIFACT_SHA256,
        "pwrightPackageSha256": verify.PWRIGHT_PACKAGE_SHA256,
        "playwrightPackageSha256": provenance["playwrightPackageSha256"],
        "harnessInputs": provenance["harnessInputs"],
        "toolingSources": provenance["toolingSources"],
        "pwrightRuntimeLibraries": provenance["pwrightRuntimeLibraries"],
    }


def test_source_pins_bind_one_target_and_v6_wheel() -> None:
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
        ("targetTestId", "wrong"),
        ("targetZeroBasedIndex", 2703),
        ("upstreamNavigationSha256", "0" * 64),
        ("frozenWheel", "/wrong/wheel.whl"),
        ("frozenWheelSha256", "0" * 64),
        ("installedNativeAbi", 128),
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
    with pytest.raises(verify.HistoryApiValidationError):
        verify.check_source_pins(pins, provenance)


def test_extra_source_pin_claim_is_rejected() -> None:
    # Arrange
    provenance = _report_metadata()
    pins = {**_source_pins(provenance), "completeFileExecution": True}

    # Act / Assert
    with pytest.raises(verify.HistoryApiValidationError):
        verify.check_source_pins(pins, provenance)


def _junit_fixture() -> bytes:
    suite = ET.Element("testsuite")
    ET.SubElement(
        suite,
        "testcase",
        classname="tests.async.test_navigation",
        name=verify.TARGET_ID.split("::", 1)[1],
        time="1.0",
    )
    return ET.tostring(suite)


def test_exact_one_case_junit_is_accepted() -> None:
    # Arrange
    raw = _junit_fixture()

    # Act
    duration = verify._junit_duration(raw, verify.TARGET_ID)

    # Assert
    assert duration == 1.0


@pytest.mark.parametrize(
    "mutation",
    (
        "missing",
        "duplicate",
        "outside",
        "wrong-id",
        "failure",
        "error",
        "skipped",
        "nan-duration",
        "negative-duration",
    ),
)
def test_one_case_junit_mutation_is_rejected(mutation: str) -> None:
    # Arrange
    suite = ET.fromstring(_junit_fixture())
    case = suite.find("testcase")
    assert case is not None
    if mutation == "missing":
        suite.remove(case)
    elif mutation == "duplicate":
        suite.append(ET.fromstring(ET.tostring(case)))
    elif mutation == "outside":
        case.set("classname", "tests.async.test_other")
    elif mutation == "wrong-id":
        case.set("name", "wrong[chromium]")
    elif mutation in {"failure", "error", "skipped"}:
        ET.SubElement(case, mutation)
    elif mutation == "nan-duration":
        case.set("time", "nan")
    elif mutation == "negative-duration":
        case.set("time", "-0.1")

    # Act / Assert
    with pytest.raises(verify.HistoryApiValidationError):
        verify._junit_duration(ET.tostring(suite), verify.TARGET_ID)


def test_sealed_source_pin_bytes_are_accepted(tmp_path: Path) -> None:
    # Arrange
    raw = b'{"kind":"fixed-fixture"}\n'
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
    raw = b'{"kind":"fixed-fixture"}\n'
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
    with pytest.raises(verify.HistoryApiValidationError):
        verify.sealed(path, expected, "source pins")


def _wheel_fixture(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> dict[str, object]:
    package = tmp_path / "installed" / "pwright"
    package.mkdir(parents=True)
    members = {
        "_native.abi3.so": b"frozen native artifact",
        "_abi.py": b"NATIVE_ABI_VERSION = 129\n",
        "async_api.py": b"async def go_forward(): pass\n",
    }
    wheel = tmp_path / "candidate.whl"
    with zipfile.ZipFile(wheel, "w") as archive:
        for name, raw in members.items():
            archive.writestr(f"pwright/{name}", raw)
            (package / name).write_bytes(raw)
    digest = hashlib.sha256()
    for name, raw in sorted(members.items()):
        encoded = name.encode()
        digest.update(len(encoded).to_bytes(8, "big"))
        digest.update(encoded)
        digest.update(len(raw).to_bytes(8, "big"))
        digest.update(raw)
    monkeypatch.setattr(verify, "WHEEL", wheel)
    monkeypatch.setattr(
        verify, "WHEEL_SHA256", hashlib.sha256(wheel.read_bytes()).hexdigest()
    )
    monkeypatch.setattr(verify, "PWRIGHT_PACKAGE_SHA256", digest.hexdigest())
    monkeypatch.setattr(
        verify,
        "PWRIGHT_ARTIFACT_SHA256",
        hashlib.sha256(members["_native.abi3.so"]).hexdigest(),
    )
    return {
        "pwrightPackage": str(package),
        "pwrightArtifactSha256": hashlib.sha256(members["_native.abi3.so"]).hexdigest(),
    }


def test_wheel_and_executed_package_bytes_are_linked(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    provenance = _wheel_fixture(tmp_path, monkeypatch)

    # Act
    result = verify._verify_wheel(provenance)

    # Assert
    assert result is None


@pytest.mark.parametrize(
    "mutation",
    ("installed-facade", "reported-native", "wheel-byte", "package-pin", "abi"),
)
def test_wheel_or_package_mutation_is_rejected(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, mutation: str
) -> None:
    # Arrange
    provenance = _wheel_fixture(tmp_path, monkeypatch)
    if mutation == "installed-facade":
        (Path(str(provenance["pwrightPackage"])) / "async_api.py").write_bytes(
            b"changed\n"
        )
    elif mutation == "reported-native":
        provenance["pwrightArtifactSha256"] = "0" * 64
    elif mutation == "wheel-byte":
        with zipfile.ZipFile(verify.WHEEL, "a") as archive:
            archive.writestr("tampered.txt", b"changed")
    elif mutation == "package-pin":
        monkeypatch.setattr(verify, "PWRIGHT_PACKAGE_SHA256", "0" * 64)
    elif mutation == "abi":
        with zipfile.ZipFile(verify.WHEEL) as archive:
            members = {name: archive.read(name) for name in archive.namelist()}
        members["pwright/_abi.py"] = b"NATIVE_ABI_VERSION = 128\n"
        with zipfile.ZipFile(verify.WHEEL, "w") as archive:
            for name, raw in members.items():
                archive.writestr(name, raw)
        (Path(str(provenance["pwrightPackage"])) / "_abi.py").write_bytes(
            members["pwright/_abi.py"]
        )
        digest = hashlib.sha256()
        for name, raw in sorted(members.items()):
            relative = name.removeprefix("pwright/").encode()
            digest.update(len(relative).to_bytes(8, "big"))
            digest.update(relative)
            digest.update(len(raw).to_bytes(8, "big"))
            digest.update(raw)
        monkeypatch.setattr(verify, "PWRIGHT_PACKAGE_SHA256", digest.hexdigest())
        monkeypatch.setattr(
            verify,
            "WHEEL_SHA256",
            hashlib.sha256(verify.WHEEL.read_bytes()).hexdigest(),
        )

    # Act / Assert
    with pytest.raises(verify.HistoryApiValidationError):
        verify._verify_wheel(provenance)
