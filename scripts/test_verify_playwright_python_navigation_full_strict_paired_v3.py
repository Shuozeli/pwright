"""Mutation controls for the independent full-80 navigation v3 verifier."""

from __future__ import annotations

import hashlib
import json
import xml.etree.ElementTree as ET
import zipfile
from pathlib import Path

import pytest
import verify_playwright_python_navigation_full_strict_paired_v3 as verify


def _line(row: dict[str, object]) -> bytes:
    return (json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n").encode()


def _ledger_fixture() -> tuple[
    list[bytes],
    list[bytes],
    list[dict[str, object]],
    dict[str, dict[str, object]],
]:
    inventory = [
        {
            "id": "tests/async/test_other.py::before[chromium]",
            "sourceClassification": "public-api-source",
        },
        {
            "id": "tests/async/test_navigation.py::pass[chromium]",
            "sourceClassification": "public-api-source",
        },
        {"id": verify.ERROR_ID, "sourceClassification": "public-api-source"},
        {
            "id": "tests/async/test_other.py::after[chromium]",
            "sourceClassification": "public-api-source",
        },
    ]
    base = [
        _line({**inventory[0], "outcome": "passed", "attempts": 1}),
        _line(
            {**inventory[1], "outcome": "failed", "attempts": 1, "error": "old failure"}
        ),
        _line({**inventory[2], "outcome": "passed", "attempts": 1}),
        _line({**inventory[3], "outcome": "not-run", "attempts": 0}),
    ]
    expected = {
        str(inventory[1]["id"]): {"outcome": "passed", "durationSeconds": 0.25},
        str(inventory[2]["id"]): {
            "outcome": "error",
            "durationSeconds": 0.5,
            "error": "isolated report invalid",
        },
    }
    successor = list(base)
    for index in (1, 2):
        successor[index] = _line(
            {
                **inventory[index],
                **expected[str(inventory[index]["id"])],
                "attempts": 1,
                "resultFile": "/evidence/full80.xml",
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
        base, successor, inventory, [1, 2], expected, "/evidence/full80.xml", mode
    )


def test_candidate_exact_reconciliation_preserves_synthetic_error() -> None:
    # Arrange
    base, successor, inventory, expected = _ledger_fixture()

    # Act
    counts = _check(base, successor, inventory, expected)

    # Assert
    assert counts == {"passed": 2, "error": 1, "not-run": 1}
    assert json.loads(successor[2])["outcome"] == "error"


@pytest.mark.parametrize(
    "mutation",
    (
        "reclassify-error",
        "error-message",
        "extra-passed-error",
        "duration",
        "attempts",
        "result-file",
        "off-target-byte",
        "target-byte",
        "base-attempts",
        "source-classification",
        "missing-target",
        "duplicate-target-index",
    ),
)
def test_candidate_mutation_is_rejected(mutation: str) -> None:
    # Arrange
    base, successor, inventory, expected = _ledger_fixture()
    target = json.loads(successor[2])
    if mutation == "reclassify-error":
        target["outcome"] = "passed"
        target.pop("error")
    elif mutation == "error-message":
        target["error"] = "manufactured error"
    elif mutation == "extra-passed-error":
        passed = json.loads(successor[1])
        passed["error"] = "extra"
        successor[1] = _line(passed)
    elif mutation == "duration":
        target["durationSeconds"] = 0.6
    elif mutation == "attempts":
        target["attempts"] = 2
    elif mutation == "result-file":
        target["resultFile"] = "/wrong.xml"
    elif mutation == "off-target-byte":
        successor[3] = b" " + successor[3]
    elif mutation == "target-byte":
        successor[2] = b" " + successor[2]
    elif mutation == "base-attempts":
        before = json.loads(base[2])
        before["attempts"] = 2
        base[2] = _line(before)
    elif mutation == "source-classification":
        inventory[2]["sourceClassification"] = "private-api-source"
        before = json.loads(base[2])
        before["sourceClassification"] = "private-api-source"
        base[2] = _line(before)
        target["sourceClassification"] = "private-api-source"
    elif mutation == "missing-target":
        expected.pop(verify.ERROR_ID)
    if mutation not in {"off-target-byte", "target-byte", "duplicate-target-index"}:
        successor[2] = _line(target)

    # Act / Assert
    with pytest.raises(verify.FullNavigationValidationError):
        if mutation == "duplicate-target-index":
            verify.check_reconciliation(
                base,
                successor,
                inventory,
                [1, 1],
                expected,
                "/evidence/full80.xml",
                "candidate",
            )
        else:
            _check(base, successor, inventory, expected)


@pytest.mark.parametrize("index", (0, 1, 2, 3))
def test_oracle_any_raw_row_mutation_is_rejected(index: int) -> None:
    # Arrange
    _, _, inventory, expected = _ledger_fixture()
    base = [_line({**row, "outcome": "passed", "attempts": 1}) for row in inventory]
    successor = list(base)
    successor[index] = b" " + successor[index]

    # Act / Assert
    with pytest.raises(verify.FullNavigationValidationError):
        _check(base, successor, inventory, expected, "oracle")


def _scope() -> dict[str, object]:
    return {
        "schema": 2,
        "kind": verify.KIND,
        "scope": verify.SCOPE,
        "publicApiParityEvidence": False,
        "fullCorpusExecution": False,
        "completeFileExecution": True,
        "fixtureAdapted": True,
        "launchMode": "cdp-adapter",
        "chromeVersionObserved": "155.0.8059.85",
    }


def test_full_file_bounded_scope_is_accepted() -> None:
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
        ("scope", "all-python"),
        ("publicApiParityEvidence", True),
        ("fullCorpusExecution", True),
        ("completeFileExecution", False),
        ("fixtureAdapted", False),
        ("launchMode", "native"),
        ("chromeVersionObserved", "unknown"),
    ),
)
def test_false_scope_claim_is_rejected(field: str, value: object) -> None:
    # Arrange
    receipt = {**_scope(), field: value}

    # Act / Assert
    with pytest.raises(verify.FullNavigationValidationError):
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
        "exitCode": 1,
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


def test_full80_report_metadata_is_accepted() -> None:
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
        ("isolatedCaseCount", 5),
        ("selections", ["tests/async/test_frame.py"]),
        ("browsers", ["firefox"]),
        ("isolateCases", False),
        ("resumeIsolated", True),
        ("exitCode", 0),
        ("pwrightPackage", "/mutable/venv/pwright"),
        ("pwrightArtifactSha256", "0" * 64),
        ("pwrightPackageSha256", "0" * 64),
    ),
)
def test_full80_report_metadata_mutation_is_rejected(field: str, value: object) -> None:
    # Arrange
    payload = {**_report_metadata(), field: value}

    # Act / Assert
    with pytest.raises(verify.FullNavigationValidationError):
        verify.check_report_metadata(payload)


def _source_pins(provenance: dict[str, object]) -> dict[str, object]:
    return {
        "schema": 1,
        "kind": "playwright-python-navigation-full-file-candidate-v3",
        "upstreamCommit": verify.COMMIT,
        "upstreamNavigationSha256": verify.NAVIGATION_SHA256,
        "officialManifestSha256": verify.MANIFEST_SHA256,
        "profiledInventorySha256": verify.INVENTORY_SHA256,
        "chromiumExpandedCases": 80,
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


def test_source_pins_bind_full80_and_v5_wheel() -> None:
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
        ("chromiumExpandedCases", 5),
        ("upstreamNavigationSha256", "0" * 64),
        ("frozenWheel", "/wrong/wheel.whl"),
        ("frozenWheelSha256", "0" * 64),
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
    with pytest.raises(verify.FullNavigationValidationError):
        verify.check_source_pins(pins, provenance)


def test_extra_source_pin_claim_is_rejected() -> None:
    # Arrange
    provenance = _report_metadata()
    pins = {**_source_pins(provenance), "publicApiParityEvidence": True}

    # Act / Assert
    with pytest.raises(verify.FullNavigationValidationError):
        verify.check_source_pins(pins, provenance)


def _junit_fixture() -> tuple[bytes, list[str]]:
    ids = [
        verify.ERROR_ID
        if index == 1
        else f"tests/async/test_navigation.py::test_case_{index}[chromium]"
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
        if index == 1:
            ET.SubElement(
                case,
                "error",
                message=f"isolated pytest report was invalid after exit 1: {verify.ERROR_RAW_SHA256}",
            )
        elif 2 <= index <= 8:
            ET.SubElement(case, "failure", message=f"failure {index}")
    return ET.tostring(suite), ids


def test_exact_full80_junit_keeps_seven_failures_and_synthetic_error() -> None:
    # Arrange
    raw, ids = _junit_fixture()

    # Act
    cases = verify._junit_cases(raw, ids)

    # Assert
    assert list(cases) == ids
    assert cases[verify.ERROR_ID]["outcome"] == "error"
    assert sum(case["outcome"] == "failed" for case in cases.values()) == 7
    assert sum(case["outcome"] == "passed" for case in cases.values()) == 72


@pytest.mark.parametrize(
    "mutation",
    (
        "missing",
        "duplicate",
        "outside",
        "order",
        "skip",
        "reclassify-error",
        "extra-error",
        "wrong-error-diagnostic",
        "wrong-error-raw-sha",
        "missing-failure-message",
        "nan-duration",
        "negative-duration",
        "wrong-counts",
    ),
)
def test_full80_junit_mutation_is_rejected(mutation: str) -> None:
    # Arrange
    raw, ids = _junit_fixture()
    suite = ET.fromstring(raw)
    cases = suite.findall("testcase")
    if mutation == "missing":
        suite.remove(cases[0])
    elif mutation == "duplicate":
        cases[0].set("name", cases[1].get("name", ""))
    elif mutation == "outside":
        cases[0].set("classname", "tests.async.test_other")
    elif mutation == "order":
        suite.remove(cases[0])
        suite.append(cases[0])
    elif mutation == "skip":
        ET.SubElement(cases[0], "skipped")
    elif mutation == "reclassify-error":
        cases[1].remove(cases[1].find("error"))  # type: ignore[arg-type]
    elif mutation == "extra-error":
        ET.SubElement(cases[0], "error", message="extra error")
    elif mutation == "wrong-error-diagnostic":
        cases[1].find("error").set("message", "normal pytest error")  # type: ignore[union-attr]
    elif mutation == "wrong-error-raw-sha":
        cases[1].find("error").set(  # type: ignore[union-attr]
            "message", "isolated pytest report was invalid after exit 1: " + "0" * 64
        )
    elif mutation == "missing-failure-message":
        cases[2].find("failure").set("message", "")  # type: ignore[union-attr]
    elif mutation == "nan-duration":
        cases[0].set("time", "nan")
    elif mutation == "negative-duration":
        cases[0].set("time", "-0.1")
    elif mutation == "wrong-counts":
        ET.SubElement(cases[0], "failure", message="extra failure")

    # Act / Assert
    with pytest.raises(verify.FullNavigationValidationError):
        verify._junit_cases(ET.tostring(suite), ids)


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
    with pytest.raises(verify.FullNavigationValidationError):
        verify.sealed(path, expected, "source pins")


def _wheel_fixture(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> dict[str, object]:
    package = tmp_path / "installed" / "pwright"
    package.mkdir(parents=True)
    members = {
        "_native.abi3.so": b"frozen native artifact",
        "_abi.py": b"NATIVE_ABI_VERSION = 128\n",
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
    "mutation", ("installed-facade", "reported-native", "wheel-byte", "package-pin")
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

    # Act / Assert
    with pytest.raises(verify.FullNavigationValidationError):
        verify._verify_wheel(provenance)
