"""Mutation controls for the independent three-case URL-match v5 verifier."""

from __future__ import annotations

import hashlib
import json
import xml.etree.ElementTree as ET
import zipfile
from pathlib import Path

import pytest
import verify_playwright_python_navigation_url_match_strict_paired_v5 as verify


def _line(row: dict[str, object]) -> bytes:
    return (json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n").encode()


def _ledger_fixture() -> tuple[
    list[bytes], list[bytes], list[dict[str, object]], dict[str, float]
]:
    inventory = [
        {
            "id": "tests/async/test_navigation.py::synthetic_error[chromium]",
            "sourceClassification": "public-api-source",
        },
        *(
            {"id": test_id, "sourceClassification": "public-api-source"}
            for test_id in verify.TARGET_IDS
        ),
        {
            "id": "tests/async/test_other.py::after[chromium]",
            "sourceClassification": "public-api-source",
        },
    ]
    base = [
        _line(
            {
                **inventory[0],
                "outcome": "error",
                "attempts": 1,
                "error": "synthetic report error",
            }
        ),
        *(
            _line(
                {
                    **inventory[index],
                    "outcome": "failed",
                    "attempts": 1,
                    "error": "old unsupported",
                }
            )
            for index in (1, 2, 3)
        ),
        _line({**inventory[4], "outcome": "not-run", "attempts": 0}),
    ]
    durations = dict(zip(verify.TARGET_IDS, (0.25, 0.5, 0.75)))
    successor = list(base)
    for index, test_id in zip((1, 2, 3), verify.TARGET_IDS):
        successor[index] = _line(
            {
                **inventory[index],
                "outcome": "passed",
                "attempts": 1,
                "durationSeconds": durations[test_id],
                "resultFile": "/evidence/url-match.xml",
            }
        )
    return base, successor, inventory, durations


def _check(
    base: list[bytes],
    successor: list[bytes],
    inventory: list[dict[str, object]],
    durations: dict[str, float],
    mode: str = "candidate",
) -> object:
    return verify.check_reconciliation(
        base,
        successor,
        inventory,
        (1, 2, 3),
        verify.TARGET_IDS,
        durations,
        "/evidence/url-match.xml",
        mode,
    )


def test_candidate_changes_exact_three_targets() -> None:
    # Arrange
    base, successor, inventory, durations = _ledger_fixture()

    # Act
    counts = _check(base, successor, inventory, durations)

    # Assert
    assert counts == {"error": 1, "passed": 3, "not-run": 1}
    assert successor[0] == base[0]


@pytest.mark.parametrize(
    "mutation",
    (
        "target-still-failed",
        "target-extra-error",
        "duration",
        "attempts",
        "result-file",
        "synthetic-error-byte",
        "off-target-byte",
        "target-byte",
        "base-not-failed",
        "base-attempts",
        "base-error-missing",
        "source-classification",
        "target-id",
        "missing-duration",
        "duplicate-target-index",
    ),
)
def test_candidate_mutation_is_rejected(mutation: str) -> None:
    # Arrange
    base, successor, inventory, durations = _ledger_fixture()
    target = json.loads(successor[2])
    if mutation == "target-still-failed":
        target["outcome"] = "failed"
        target["error"] = "old unsupported"
    elif mutation == "target-extra-error":
        target["error"] = "fabricated"
    elif mutation == "duration":
        target["durationSeconds"] = 0.6
    elif mutation == "attempts":
        target["attempts"] = 2
    elif mutation == "result-file":
        target["resultFile"] = "/wrong.xml"
    elif mutation == "synthetic-error-byte":
        successor[0] = b" " + successor[0]
    elif mutation == "off-target-byte":
        successor[4] = b" " + successor[4]
    elif mutation == "target-byte":
        successor[2] = b" " + successor[2]
    elif mutation in {"base-not-failed", "base-attempts", "base-error-missing"}:
        before = json.loads(base[2])
        if mutation == "base-not-failed":
            before["outcome"] = "passed"
        elif mutation == "base-attempts":
            before["attempts"] = 2
        else:
            before.pop("error")
        base[2] = _line(before)
    elif mutation == "source-classification":
        inventory[2]["sourceClassification"] = "private-api-source"
        before = json.loads(base[2])
        before["sourceClassification"] = "private-api-source"
        base[2] = _line(before)
        target["sourceClassification"] = "private-api-source"
    elif mutation == "target-id":
        target["id"] = "tests/async/test_navigation.py::wrong[chromium]"
    elif mutation == "missing-duration":
        durations.pop(verify.TARGET_IDS[1])
    if mutation not in {"synthetic-error-byte", "off-target-byte", "target-byte"}:
        successor[2] = _line(target)

    # Act / Assert
    with pytest.raises(verify.UrlMatchValidationError):
        if mutation == "duplicate-target-index":
            verify.check_reconciliation(
                base,
                successor,
                inventory,
                (1, 1, 3),
                verify.TARGET_IDS,
                durations,
                "/evidence/url-match.xml",
                "candidate",
            )
        else:
            _check(base, successor, inventory, durations)


@pytest.mark.parametrize("index", (0, 1, 2, 3, 4))
def test_oracle_any_raw_row_mutation_is_rejected(index: int) -> None:
    # Arrange
    _, _, inventory, durations = _ledger_fixture()
    base = [_line({**row, "outcome": "passed", "attempts": 1}) for row in inventory]
    successor = list(base)
    successor[index] = b" " + successor[index]

    # Act / Assert
    with pytest.raises(verify.UrlMatchValidationError):
        _check(base, successor, inventory, durations, "oracle")


def test_oracle_exact_raw_rows_are_accepted() -> None:
    # Arrange
    _, _, inventory, durations = _ledger_fixture()
    base = [_line({**row, "outcome": "passed", "attempts": 1}) for row in inventory]

    # Act
    counts = _check(base, list(base), inventory, durations, "oracle")

    # Assert
    assert counts == {"passed": 5}


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


def test_three_case_bounded_scope_is_accepted() -> None:
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
    with pytest.raises(verify.UrlMatchValidationError):
        verify.check_receipt_scope(receipt)


def _report_metadata() -> dict[str, object]:
    return {
        "schema": 5,
        "mode": "candidate",
        "commit": verify.COMMIT,
        "manifest": str(verify.MANIFEST),
        "manifestSha256": verify.MANIFEST_SHA256,
        "upstream": str(verify.UPSTREAM),
        "inventory": None,
        "inventorySha256": None,
        "scope": "selected",
        "endpoint": "http://127.0.0.1:9222",
        "fixtureBind": "100.78.97.63",
        "fixtureHost": "macbookpro-yuacx.tail8f3b66.ts.net",
        "result": str(verify.RESULT),
        "resultSha256": verify.RESULT_SHA256,
        "caseReports": None,
        "caseReportsSha256": None,
        "caseTimeout": 45.0,
        "isolatedCaseIdsSha256": None,
        "isolatedCaseCount": 0,
        "selections": list(verify.TARGET_IDS),
        "browsers": ["chromium"],
        "launchMode": "cdp-adapter",
        "isolateCases": False,
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
        "playwrightPackageSha256": verify.PLAYWRIGHT_PACKAGE_SHA256,
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
        ("inventory", str(verify.INVENTORY)),
        ("inventorySha256", verify.INVENTORY_SHA256),
        ("caseReports", str(Path(f"{verify.RESULT}.cases"))),
        ("caseReportsSha256", "0" * 64),
        ("isolatedCaseIdsSha256", "0" * 64),
        ("isolatedCaseCount", 3),
        ("selections", ["tests/async/test_navigation.py"]),
        ("browsers", ["firefox"]),
        ("isolateCases", True),
        ("resumeIsolated", True),
        ("exitCode", 1),
        ("pwrightPackage", "/mutable/venv/pwright"),
        ("pwrightArtifactSha256", "0" * 64),
        ("pwrightPackageSha256", "0" * 64),
        ("playwrightPackageSha256", "0" * 64),
    ),
)
def test_focused_report_metadata_mutation_is_rejected(
    field: str, value: object
) -> None:
    # Arrange
    payload = {**_report_metadata(), field: value}

    # Act / Assert
    with pytest.raises(verify.UrlMatchValidationError):
        verify.check_report_metadata(payload)


def _source_pins(provenance: dict[str, object]) -> dict[str, object]:
    return {
        "schema": 1,
        "kind": "playwright-python-navigation-url-match-candidate-v5",
        "upstreamCommit": verify.COMMIT,
        "upstreamNavigationSha256": verify.NAVIGATION_SHA256,
        "officialManifestSha256": verify.MANIFEST_SHA256,
        "profiledInventorySha256": verify.INVENTORY_SHA256,
        "targetTestIds": list(verify.TARGET_IDS),
        "targetZeroBasedIndexes": list(verify.TARGET_INDEXES),
        "frozenWheel": str(verify.WHEEL),
        "frozenWheelSha256": verify.WHEEL_SHA256,
        "wheelName": verify.WHEEL.name,
        "abi130Identity": str(verify.IDENTITY),
        "abi130IdentitySha256": verify.IDENTITY_SHA256,
        "installedNativeAbi": 130,
        "pwrightArtifactSha256": verify.PWRIGHT_ARTIFACT_SHA256,
        "pwrightPackageSha256": verify.PWRIGHT_PACKAGE_SHA256,
        "playwrightPackageSha256": provenance["playwrightPackageSha256"],
        "harnessInputs": provenance["harnessInputs"],
        "toolingSources": provenance["toolingSources"],
        "pwrightRuntimeLibraries": provenance["pwrightRuntimeLibraries"],
    }


def test_source_pins_bind_three_targets_and_abi130() -> None:
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
        ("targetTestIds", list(reversed(verify.TARGET_IDS))),
        ("targetZeroBasedIndexes", [2881, 2914, 2916]),
        ("upstreamNavigationSha256", "0" * 64),
        ("frozenWheel", "/wrong/wheel.whl"),
        ("frozenWheelSha256", "0" * 64),
        ("abi130Identity", "/wrong/identity.json"),
        ("abi130IdentitySha256", "0" * 64),
        ("installedNativeAbi", 129),
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
    with pytest.raises(verify.UrlMatchValidationError):
        verify.check_source_pins(pins, provenance)


def test_extra_source_pin_claim_is_rejected() -> None:
    # Arrange
    provenance = _report_metadata()
    pins = {**_source_pins(provenance), "completeFileExecution": True}

    # Act / Assert
    with pytest.raises(verify.UrlMatchValidationError):
        verify.check_source_pins(pins, provenance)


def _identity() -> dict[str, object]:
    return {
        "schema": 1,
        "kind": "pwright-python-navigation-url-match-abi130-v2-source-bound-identity",
        "wheelPath": str(verify.WHEEL),
        "wheelSha256": verify.WHEEL_SHA256,
        "installedIdentity": {
            "abi": 130,
            "nativeModule": str(verify.CANDIDATE_ARTIFACT),
            "packageVersion": "0.1.0",
            "selectedFeatures": ["frame-navigation", "navigation-url-match"],
        },
        "nativeArtifactSha256": verify.PWRIGHT_ARTIFACT_SHA256,
        "installedPackageSha256": verify.PWRIGHT_PACKAGE_SHA256,
        "officialRun": {
            "resultPath": str(verify.RESULT),
            "resultSha256": verify.RESULT_SHA256,
            "provenancePath": str(verify.PROVENANCE),
            "exitCode": 0,
            "selections": list(verify.TARGET_IDS),
            "upstreamCommit": verify.COMMIT,
            "upstreamTag": "v1.58.0",
            "manifestSha256": verify.MANIFEST_SHA256,
            "tests": 3,
            "failures": 0,
            "errors": 0,
            "skipped": 0,
        },
        "defaultOffProof": {
            "addonDefaultFeatures": [],
            "releaseCheckPassed": True,
            "urlMatchAddedPackagesVsFrameNavigation": [],
        },
    }


def test_source_bound_identity_is_accepted() -> None:
    # Arrange
    identity = _identity()

    # Act
    result = verify.check_identity(identity)

    # Assert
    assert result is None


@pytest.mark.parametrize(
    ("part", "field", "value"),
    (
        ("root", "wheelSha256", "0" * 64),
        ("root", "nativeArtifactSha256", "0" * 64),
        ("root", "installedPackageSha256", "0" * 64),
        ("installedIdentity", "abi", 129),
        ("installedIdentity", "nativeModule", "/wrong/native.so"),
        ("installedIdentity", "selectedFeatures", ["frame-navigation"]),
        ("officialRun", "resultSha256", "0" * 64),
        ("officialRun", "provenancePath", "/wrong/provenance.json"),
        ("officialRun", "selections", list(reversed(verify.TARGET_IDS))),
        ("officialRun", "failures", 1),
        ("defaultOffProof", "addonDefaultFeatures", ["navigation-url-match"]),
        ("defaultOffProof", "releaseCheckPassed", False),
        ("defaultOffProof", "urlMatchAddedPackagesVsFrameNavigation", ["unexpected"]),
    ),
)
def test_source_bound_identity_mutation_is_rejected(
    part: str, field: str, value: object
) -> None:
    # Arrange
    identity = _identity()
    if part == "root":
        identity[field] = value
    else:
        nested = identity[part]
        assert isinstance(nested, dict)
        nested[field] = value

    # Act / Assert
    with pytest.raises(verify.UrlMatchValidationError):
        verify.check_identity(identity)


def _junit_fixture() -> bytes:
    suite = ET.Element("testsuite")
    for test_id in verify.TARGET_IDS:
        ET.SubElement(
            suite,
            "testcase",
            classname="tests.async.test_navigation",
            name=test_id.split("::", 1)[1],
            time="0.5",
        )
    return ET.tostring(suite)


def test_exact_three_case_junit_is_accepted() -> None:
    # Arrange
    raw = _junit_fixture()

    # Act
    durations = verify._junit_durations(raw, verify.TARGET_IDS)

    # Assert
    assert tuple(durations) == verify.TARGET_IDS
    assert set(durations.values()) == {0.5}


@pytest.mark.parametrize(
    "mutation",
    (
        "missing",
        "duplicate",
        "outside",
        "wrong-id",
        "order",
        "failure",
        "error",
        "skipped",
        "nan-duration",
        "negative-duration",
    ),
)
def test_three_case_junit_mutation_is_rejected(mutation: str) -> None:
    # Arrange
    suite = ET.fromstring(_junit_fixture())
    cases = suite.findall("testcase")
    if mutation == "missing":
        suite.remove(cases[0])
    elif mutation == "duplicate":
        cases[0].set("name", cases[1].get("name", ""))
    elif mutation == "outside":
        cases[0].set("classname", "tests.async.test_other")
    elif mutation == "wrong-id":
        cases[0].set("name", "wrong[chromium]")
    elif mutation == "order":
        suite.remove(cases[0])
        suite.append(cases[0])
    elif mutation in {"failure", "error", "skipped"}:
        ET.SubElement(cases[0], mutation)
    elif mutation == "nan-duration":
        cases[0].set("time", "nan")
    elif mutation == "negative-duration":
        cases[0].set("time", "-0.1")

    # Act / Assert
    with pytest.raises(verify.UrlMatchValidationError):
        verify._junit_durations(ET.tostring(suite), verify.TARGET_IDS)


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
    with pytest.raises(verify.UrlMatchValidationError):
        verify.sealed(path, expected, "source pins")


def _wheel_fixture(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> dict[str, object]:
    package = tmp_path / "installed" / "pwright"
    package.mkdir(parents=True)
    members = {
        "_native.abi3.so": b"frozen native artifact",
        "_abi.py": b"NATIVE_ABI_VERSION = 130\n",
        "async_api.py": b"async def expect_navigation(): pass\n",
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
    with pytest.raises(verify.UrlMatchValidationError):
        verify._verify_wheel(provenance)
