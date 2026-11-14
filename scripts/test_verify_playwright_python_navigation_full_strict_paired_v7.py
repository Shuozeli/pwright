"""Mutation controls for the independent ABI131 navigation v7 verifier."""

from __future__ import annotations

import hashlib
import json
import xml.etree.ElementTree as ET
import zipfile
from pathlib import Path

import pytest
import verify_playwright_python_navigation_full_strict_paired_v7 as verify


def _line(row: dict[str, object]) -> bytes:
    return (json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n").encode()


def _rows(
    monkeypatch: pytest.MonkeyPatch,
) -> tuple[
    list[bytes], list[bytes], list[dict[str, object]], dict[str, dict[str, object]]
]:
    # Arrange
    inventory: list[dict[str, object]] = [
        {
            "id": "tests/async/test_other.py::before[chromium]",
            "sourceClassification": "public-api-source",
        },
        {
            "id": "tests/async/test_navigation.py::first[chromium]",
            "sourceClassification": "public-api-source",
        },
        {
            "id": "tests/async/test_navigation.py::second[chromium]",
            "sourceClassification": "public-api-source",
        },
        {
            "id": "tests/async/test_other.py::after[chromium]",
            "sourceClassification": "public-api-source",
        },
    ]
    monkeypatch.setattr(verify, "RECORDS", 4)
    monkeypatch.setattr(verify, "TARGET_INDEXES", (1, 2))
    monkeypatch.setattr(
        verify,
        "EXPECTED_SUCCESSOR_COUNTS",
        {
            "candidate": {"passed": 2, "failed": 1, "not-run": 1},
            "oracle": {"passed": 3, "not-run": 1},
        },
    )
    cases = {
        str(inventory[1]["id"]): {"outcome": "passed", "durationSeconds": 0.25},
        str(inventory[2]["id"]): {
            "outcome": "failed",
            "durationSeconds": 0.5,
            "error": "real official assertion failed",
        },
    }
    base = [
        _line({**inventory[0], "outcome": "passed", "attempts": 1}),
        _line(
            {
                **inventory[1],
                "outcome": "passed",
                "attempts": 1,
                "durationSeconds": 0.2,
                "resultFile": "/evidence/old.xml",
            }
        ),
        _line(
            {
                **inventory[2],
                "outcome": "error",
                "attempts": 1,
                "durationSeconds": 0.4,
                "resultFile": "/evidence/old.xml",
                "error": "old synthetic report error",
            }
        ),
        _line({**inventory[3], "outcome": "not-run", "attempts": 0}),
    ]
    successor = list(base)
    for index in (1, 2):
        case = cases[str(inventory[index]["id"])]
        successor[index] = _line(
            {
                **inventory[index],
                **case,
                "attempts": 1,
                "resultFile": str(verify.RESULT),
            }
        )
    return base, successor, inventory, cases


def test_candidate_exactly_two_target_rows_are_replaced(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    base, successor, inventory, cases = _rows(monkeypatch)

    # Act
    counts = verify.check_reconciliation(base, successor, inventory, cases, "candidate")

    # Assert
    assert counts == {"passed": 2, "failed": 1, "not-run": 1}
    assert successor[0] == base[0] and successor[3] == base[3]
    assert successor[1] != base[1] and successor[2] != base[2]


@pytest.mark.parametrize(
    "mutation",
    (
        "off-target-byte",
        "target-byte",
        "target-status",
        "target-error",
        "target-duration",
        "target-attempts",
        "target-result-path",
        "missing-replacement",
        "target-id",
        "target-source-metadata",
        "missing-report-target",
        "extra-report-target",
        "base-attempts",
    ),
)
def test_candidate_mutation_is_rejected(
    monkeypatch: pytest.MonkeyPatch, mutation: str
) -> None:
    # Arrange
    base, successor, inventory, cases = _rows(monkeypatch)
    target = json.loads(successor[2])
    if mutation == "off-target-byte":
        successor[3] = b" " + successor[3]
    elif mutation == "target-byte":
        successor[2] = b" " + successor[2]
    elif mutation == "target-status":
        target["outcome"] = "passed"
        target.pop("error")
        successor[2] = _line(target)
    elif mutation == "target-error":
        target["error"] = "invented failure"
        successor[2] = _line(target)
    elif mutation == "target-duration":
        target["durationSeconds"] = 0.6
        successor[2] = _line(target)
    elif mutation == "target-attempts":
        target["attempts"] = 2
        successor[2] = _line(target)
    elif mutation == "target-result-path":
        target["resultFile"] = "/evidence/wrong.xml"
        successor[2] = _line(target)
    elif mutation == "missing-replacement":
        successor[2] = base[2]
    elif mutation == "target-id":
        target["id"] = "tests/async/test_other.py::forged[chromium]"
        successor[2] = _line(target)
    elif mutation == "target-source-metadata":
        target["sourceClassification"] = "private-api-source"
        successor[2] = _line(target)
    elif mutation == "missing-report-target":
        cases.pop(str(inventory[2]["id"]))
    elif mutation == "extra-report-target":
        cases["tests/async/test_navigation.py::forged[chromium]"] = {
            "outcome": "passed",
            "durationSeconds": 0.1,
        }
    elif mutation == "base-attempts":
        before = json.loads(base[2])
        before["attempts"] = 0
        base[2] = _line(before)

    # Act / Assert
    with pytest.raises(verify.FullNavigationV7ValidationError):
        verify.check_reconciliation(base, successor, inventory, cases, "candidate")


@pytest.mark.parametrize("index", (0, 1, 2, 3))
def test_oracle_any_row_mutation_is_rejected(
    monkeypatch: pytest.MonkeyPatch, index: int
) -> None:
    # Arrange
    _, _, inventory, cases = _rows(monkeypatch)
    oracle = [
        _line({**source, "outcome": "passed", "attempts": 1})
        if row_index < 3
        else _line({**source, "outcome": "not-run", "attempts": 0})
        for row_index, source in enumerate(inventory)
    ]
    successor = list(oracle)
    successor[index] = b" " + successor[index]

    # Act / Assert
    with pytest.raises(verify.FullNavigationV7ValidationError):
        verify.check_reconciliation(oracle, successor, inventory, cases, "oracle")


def _receipt_scope() -> dict[str, object]:
    return {
        "schema": 2,
        "kind": verify.KIND,
        "scope": verify.SCOPE,
        "publicApiParityEvidence": False,
        "fullCorpusExecution": False,
        "completeFileExecution": True,
        "fixtureAdapted": True,
        "fullFileRegression": True,
        "focusedGreenDoesNotImplyFullGreen": True,
        "launchMode": "cdp-adapter",
        "chromeVersionObserved": "155.0.8059.85",
    }


def test_receipt_scope_accepts_red_regression() -> None:
    # Arrange
    receipt = _receipt_scope()

    # Act
    result = verify.check_receipt_scope(receipt)

    # Assert
    assert result is None


@pytest.mark.parametrize(
    ("field", "value"),
    (
        ("publicApiParityEvidence", True),
        ("fullCorpusExecution", True),
        ("completeFileExecution", False),
        ("fullFileRegression", False),
        ("focusedGreenDoesNotImplyFullGreen", False),
    ),
)
def test_receipt_scope_rejects_false_claim(field: str, value: object) -> None:
    # Arrange
    receipt = {**_receipt_scope(), field: value}

    # Act / Assert
    with pytest.raises(verify.FullNavigationV7ValidationError):
        verify.check_receipt_scope(receipt)


def _junit() -> tuple[bytes, list[str]]:
    ids = [
        f"tests/async/test_navigation.py::test_case_{index}[chromium]"
        for index in range(80)
    ]
    suite = ET.Element("testsuite")
    for index, test_id in enumerate(ids):
        case = ET.SubElement(
            suite,
            "testcase",
            classname="tests.async.test_navigation",
            name=test_id.split("::", 1)[1],
            time="0.25",
        )
        if index == 70:
            ET.SubElement(case, "failure", message="real assertion failed")
    return ET.tostring(suite), ids


def test_full80_junit_preserves_red_case(monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    raw, ids = _junit()
    monkeypatch.setattr(verify, "EXPECTED_REPORT_COUNTS", {"passed": 79, "failed": 1})

    # Act
    cases = verify._junit_cases(raw, ids)

    # Assert
    assert len(cases) == 80
    assert cases[ids[70]] == {
        "outcome": "failed",
        "durationSeconds": 0.25,
        "error": "real assertion failed",
    }


@pytest.mark.parametrize(
    "mutation",
    (
        "missing",
        "duplicate",
        "reordered",
        "skip",
        "nan",
        "missing-diagnostic",
        "wrong-counts",
    ),
)
def test_full80_junit_mutation_is_rejected(
    monkeypatch: pytest.MonkeyPatch, mutation: str
) -> None:
    # Arrange
    raw, ids = _junit()
    monkeypatch.setattr(verify, "EXPECTED_REPORT_COUNTS", {"passed": 79, "failed": 1})
    suite = ET.fromstring(raw)
    cases = suite.findall("testcase")
    if mutation == "missing":
        suite.remove(cases[0])
    elif mutation == "duplicate":
        cases[0].set("name", cases[1].get("name", ""))
    elif mutation == "reordered":
        suite.remove(cases[0])
        suite.append(cases[0])
    elif mutation == "skip":
        ET.SubElement(cases[0], "skipped")
    elif mutation == "nan":
        cases[0].set("time", "nan")
    elif mutation == "missing-diagnostic":
        cases[70].find("failure").set("message", "")  # type: ignore[union-attr]
    elif mutation == "wrong-counts":
        ET.SubElement(cases[0], "failure", message="new failure")

    # Act / Assert
    with pytest.raises(verify.FullNavigationV7ValidationError):
        verify._junit_cases(ET.tostring(suite), ids)


def test_unsealed_report_pin_cannot_be_used(tmp_path: Path) -> None:
    # Arrange
    report = tmp_path / "report.xml"
    report.write_bytes(b"<testsuite/>")

    # Act / Assert
    with pytest.raises(verify.FullNavigationV7ValidationError):
        verify.sealed(report, "0" * 64, "report")


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
        "isolatedCaseCount": 80,
        "selections": ["tests/async/test_navigation.py"],
        "browsers": ["chromium"],
        "launchMode": "cdp-adapter",
        "isolateCases": True,
        "resumeIsolated": False,
        "collectOnly": False,
        "bootstrapOnly": False,
        "manifestCases": False,
        "secureHttpFixture": False,
        "pythonExecutable": str(verify.PYTHON_EXECUTABLE),
        "pwrightPackage": str(verify.CANDIDATE_PACKAGE),
        "pwrightArtifact": str(verify.CANDIDATE_ARTIFACT),
        "pwrightArtifactSha256": verify.PWRIGHT_ARTIFACT_SHA256,
        "pwrightPackageSha256": verify.PWRIGHT_PACKAGE_SHA256,
        "playwrightPackageSha256": verify.PLAYWRIGHT_PACKAGE_SHA256,
        "exitCode": 1,
    }


def test_exact_report_metadata_is_accepted() -> None:
    # Arrange
    metadata = _report_metadata()

    # Act
    result = verify.check_report_metadata(metadata)

    # Assert
    assert result is None


@pytest.mark.parametrize(
    ("field", "value"),
    (
        ("mode", "oracle"),
        ("commit", "wrong"),
        ("manifestSha256", "0" * 64),
        ("inventorySha256", "0" * 64),
        ("fixtureBind", "0.0.0.0"),
        ("fixtureHost", "wrong"),
        ("resultSha256", "1" * 64),
        ("caseReportsSha256", "1" * 64),
        ("isolatedCaseCount", 79),
        ("selections", ["tests/async/test_other.py"]),
        ("browsers", ["firefox"]),
        ("isolateCases", False),
        ("pwrightPackageSha256", "1" * 64),
        ("pwrightArtifactSha256", "1" * 64),
        ("exitCode", 2),
    ),
)
def test_report_metadata_mutation_is_rejected(field: str, value: object) -> None:
    # Arrange
    metadata = {**_report_metadata(), field: value}

    # Act / Assert
    with pytest.raises(verify.FullNavigationV7ValidationError):
        verify.check_report_metadata(metadata)


def test_source_pins_bind_wheel_and_report_inputs() -> None:
    # Arrange
    provenance = {
        "harnessInputs": [{"path": "/harness", "sha256": "1" * 64}],
        "toolingSources": [{"path": "/tool", "sha256": "2" * 64}],
        "pwrightRuntimeLibraries": [{"path": "/lib", "sha256": "3" * 64}],
    }

    # Act
    pins = verify.source_pins_expected(provenance)

    # Assert
    assert pins["installedNativeAbi"] == 131
    assert pins["frozenWheelSha256"] == verify.WHEEL_SHA256
    assert pins["pwrightArtifactSha256"] == verify.PWRIGHT_ARTIFACT_SHA256
    assert pins["harnessInputs"] == provenance["harnessInputs"]


@pytest.mark.parametrize(
    "missing", ("harnessInputs", "toolingSources", "pwrightRuntimeLibraries")
)
def test_source_pins_reject_missing_runtime_evidence(missing: str) -> None:
    # Arrange
    provenance = {
        "harnessInputs": [{"path": "/harness", "sha256": "1" * 64}],
        "toolingSources": [{"path": "/tool", "sha256": "2" * 64}],
        "pwrightRuntimeLibraries": [{"path": "/lib", "sha256": "3" * 64}],
    }
    provenance[missing] = []

    # Act / Assert
    with pytest.raises(verify.FullNavigationV7ValidationError):
        verify.source_pins_expected(provenance)


def _wheel_fixture(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> dict[str, object]:
    package = tmp_path / "installed" / "pwright"
    package.mkdir(parents=True)
    members = {
        "_native.abi3.so": b"frozen ABI131 native artifact",
        "_abi.py": b"NATIVE_ABI_VERSION = 131\n",
        "async_api.py": b"async def goto(): pass\n",
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
    monkeypatch.setattr(verify, "CANDIDATE_PACKAGE", package)
    monkeypatch.setattr(verify, "PWRIGHT_PACKAGE_SHA256", digest.hexdigest())
    monkeypatch.setattr(
        verify,
        "PWRIGHT_ARTIFACT_SHA256",
        hashlib.sha256(members["_native.abi3.so"]).hexdigest(),
    )
    return {
        "pwrightPackageSha256": digest.hexdigest(),
        "pwrightArtifactSha256": hashlib.sha256(members["_native.abi3.so"]).hexdigest(),
    }


def test_wheel_matches_executed_python_package(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    provenance = _wheel_fixture(tmp_path, monkeypatch)

    # Act
    result = verify._verify_wheel(provenance)

    # Assert
    assert result is None


@pytest.mark.parametrize(
    "mutation", ("installed-source", "native-pin", "wheel-byte", "report-package")
)
def test_wheel_mismatch_is_rejected(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, mutation: str
) -> None:
    # Arrange
    provenance = _wheel_fixture(tmp_path, monkeypatch)
    if mutation == "installed-source":
        (verify.CANDIDATE_PACKAGE / "async_api.py").write_bytes(b"modified\n")
    elif mutation == "native-pin":
        monkeypatch.setattr(verify, "PWRIGHT_ARTIFACT_SHA256", "0" * 64)
    elif mutation == "wheel-byte":
        with zipfile.ZipFile(verify.WHEEL, "a") as archive:
            archive.writestr("tampered.txt", b"changed")
    elif mutation == "report-package":
        provenance["pwrightPackageSha256"] = "0" * 64

    # Act / Assert
    with pytest.raises(verify.FullNavigationV7ValidationError):
        verify._verify_wheel(provenance)
