#!/usr/bin/env python3
"""Independently verify one source-bound dummy-Node Core launch or a pair."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path
from typing import Any, cast

import run_playwright_core_dummy_node_launch as lane

SCOPE = "core-dummy-node-launch-strict-v1"
HEX32 = re.compile(r"^[0-9a-f]{32}$")
HEX64 = re.compile(r"^[0-9a-f]{64}$")
STRICT_FIELDS = {
    "schema",
    "scope",
    "mode",
    "runId",
    "officialCliPid",
    "suite",
    "specId",
    "selector",
    "upstreamCommit",
    "upstreamSources",
    "upstreamArtifacts",
    "nodeSha256",
    "manifest",
    "manifestSha256",
    "candidateProfile",
    "candidateSourceSha256",
    "candidateArtifactSha256",
    "localSources",
    "result",
    "resultSha256",
    "provenanceSha256",
    "launchReceiptSha256",
    "processReceiptSha256",
    "logSha256",
    "testStatus",
    "exitCode",
    "processOwnershipProof",
    "fixtureAdapted",
    "unadaptedOfficialPass",
    "upstreamAssertionsModified",
    "fixtureAdaptationEligible",
    "publicApiParityEvidence",
    "diagnosticOnly",
    "ledgerEligible",
}
PROVENANCE_FIELDS = {
    "schema",
    "evidenceScope",
    "mode",
    "executionPolicy",
    "manifest",
    "manifestSha256",
    "commit",
    "suite",
    "selector",
    "specId",
    "upstream",
    "candidate",
    "candidateProfile",
    "candidateSourceSha256",
    "candidateArtifactSha256",
    "localSources",
    "sandboxProfileSha256",
    "sandboxAllowedExecutables",
    "sandboxNetworkOutbound",
    "command",
    "exitCode",
    "testStatus",
    "result",
    "resultSha256",
    "receipt",
    "receiptSha256",
    "processReceipt",
    "processReceiptSha256",
    "runId",
    "officialCliPid",
    "upstreamSources",
    "upstreamArtifacts",
    "nodeSha256",
    "log",
    "logSha256",
    "fixtureAdapted",
    "unadaptedOfficialPass",
}
PROOF_FIELDS = {
    "publicMethod",
    "guardPid",
    "launcherPid",
    "childPid",
    "cliPid",
    "harnessPid",
    "browserProfilePath",
    "browserProfileRemoved",
    "childReaped",
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def object_at(path: Path, label: str) -> dict[str, Any]:
    require(path.is_file() and not path.is_symlink(), f"{label} is missing or linked")
    value = json.loads(path.read_text())
    require(isinstance(value, dict), f"{label} is not an object")
    return cast(dict[str, Any], value)


def file_hash(path: Path, expected: object, label: str) -> None:
    require(
        isinstance(expected, str)
        and bool(HEX64.fullmatch(expected))
        and path.is_file()
        and not path.is_symlink()
        and lane.digest(path) == expected,
        f"{label} source or artifact SHA-256 differs",
    )


def verify_strict(strict_path: Path) -> dict[str, Any]:
    strict_path = strict_path.parent.resolve() / strict_path.name
    strict = object_at(strict_path, "strict sidecar")
    require(set(strict) == STRICT_FIELDS, "strict sidecar fields differ")
    require(
        strict["schema"] == 2 and strict["scope"] == SCOPE, "strict identity differs"
    )
    mode = strict["mode"]
    require(mode in ("candidate", "oracle"), "strict mode differs")
    run_id = strict["runId"]
    require(
        isinstance(run_id, str) and bool(HEX32.fullmatch(run_id)),
        "strict run ID differs",
    )
    require(
        strict["suite"] == lane.SUITE
        and strict["specId"] == lane.SPEC_ID
        and strict["selector"] == lane.SELECTOR
        and strict["upstreamCommit"] == lane.COMMIT
        and strict["upstreamSources"] == lane.UPSTREAM_SOURCES,
        "strict official case or source selection differs",
    )
    manifest = lane.STRICT_MANIFEST
    require(strict["manifest"] == str(manifest), "strict manifest path differs")
    file_hash(manifest, strict["manifestSha256"], "strict manifest")
    result = Path(strict["result"])
    require(
        result.is_absolute()
        and strict_path == result.with_suffix(".strict.json")
        and not result.is_symlink(),
        "strict result path differs",
    )
    provenance_path = result.with_suffix(".provenance.json")
    launch_receipt = result.with_suffix(".launch-receipt.jsonl")
    process_receipt = result.with_suffix(".process-receipt.jsonl")
    log_path = result.with_suffix(".log")
    for path, field, label in (
        (result, "resultSha256", "result"),
        (provenance_path, "provenanceSha256", "provenance"),
        (launch_receipt, "launchReceiptSha256", "launch receipt"),
        (process_receipt, "processReceiptSha256", "process receipt"),
        (log_path, "logSha256", "log"),
    ):
        file_hash(path, strict[field], label)
    provenance = object_at(provenance_path, "provenance")
    require(set(provenance) == PROVENANCE_FIELDS, "strict provenance fields differ")
    require(
        provenance["schema"] == 2
        and provenance["evidenceScope"] == "fixture-adapted-owned-public-launch-strict"
        and provenance["executionPolicy"] == lane.STRICT_POLICY
        and provenance["mode"] == mode
        and provenance["runId"] == run_id
        and provenance["officialCliPid"] == strict["officialCliPid"]
        and provenance["manifest"] == str(manifest)
        and provenance["manifestSha256"] == strict["manifestSha256"]
        and provenance["commit"] == lane.COMMIT
        and provenance["suite"] == lane.SUITE
        and provenance["selector"] == lane.SELECTOR
        and provenance["specId"] == lane.SPEC_ID
        and provenance["result"] == str(result)
        and provenance["resultSha256"] == strict["resultSha256"]
        and provenance["receipt"] == str(launch_receipt)
        and provenance["receiptSha256"] == strict["launchReceiptSha256"]
        and provenance["processReceipt"] == str(process_receipt)
        and provenance["processReceiptSha256"] == strict["processReceiptSha256"]
        and provenance["log"] == str(log_path)
        and provenance["logSha256"] == strict["logSha256"]
        and provenance["upstreamSources"] == lane.UPSTREAM_SOURCES
        and provenance["upstreamArtifacts"] == strict["upstreamArtifacts"]
        and provenance["nodeSha256"] == strict["nodeSha256"]
        and provenance["fixtureAdapted"] is True
        and provenance["unadaptedOfficialPass"] is False,
        "strict provenance does not bind the selected execution",
    )
    upstream = Path(provenance["upstream"])
    require(
        upstream.is_absolute() and upstream.is_dir() and not upstream.is_symlink(),
        "upstream path differs",
    )
    lane.pinned_checkout(upstream)
    lane.pinned_manifest(upstream, manifest)
    require(
        strict["upstreamArtifacts"]
        == {
            relative: lane.directory_digest(upstream / relative)
            for relative in lane.UPSTREAM_ARTIFACT_DIRS
        },
        "upstream runtime artifact SHA-256 differs",
    )
    local = strict["localSources"]
    require(
        isinstance(local, dict)
        and set(local) == set(lane.STRICT_SOURCES)
        and provenance["localSources"] == local,
        "strict local tooling source set differs",
    )
    for relative in lane.STRICT_SOURCES:
        file_hash(lane.ROOT / relative, local[relative], relative)
    require(
        strict["candidateProfile"]
        == (lane.TRACE_PROFILE if mode == "candidate" else None)
        and strict["candidateSourceSha256"]
        == (lane.STRICT_CANDIDATE_SOURCE_SHA256 if mode == "candidate" else None)
        and strict["candidateArtifactSha256"]
        == (lane.STRICT_CANDIDATE_ARTIFACT_SHA256 if mode == "candidate" else None)
        and provenance["candidateProfile"] == strict["candidateProfile"]
        and provenance["candidateSourceSha256"] == strict["candidateSourceSha256"]
        and provenance["candidateArtifactSha256"] == strict["candidateArtifactSha256"],
        "strict candidate source/profile/artifact identity differs",
    )
    if mode == "candidate":
        candidate = Path(provenance["candidate"])
        require(
            candidate == lane.ROOT / "packages/pwright"
            and candidate.is_dir()
            and not candidate.is_symlink()
            and lane.source_digest(candidate) == strict["candidateSourceSha256"],
            "strict candidate package source differs",
        )
        file_hash(
            candidate / "native" / lane.TRACE_PROFILE / "pwright.node",
            strict["candidateArtifactSha256"],
            "strict candidate native artifact",
        )
    else:
        require(provenance["candidate"] is None, "oracle contains a candidate package")
    allowed = provenance["sandboxAllowedExecutables"]
    require(
        isinstance(allowed, list) and len(allowed) == 8,
        "strict sandbox allowlist differs",
    )
    node = Path(allowed[0])
    require(
        node.is_absolute() and node.resolve() == node,
        "strict Node executable path differs",
    )
    file_hash(node, strict["nodeSha256"], "strict Node executable")
    require(
        allowed
        == [
            str(node),
            str(upstream / lane.ASSET),
            str(lane.ROOT / lane.PROCESS_GUARD),
            "/usr/bin/env",
            "/usr/bin/python3",
            "/Library/Developer/CommandLineTools/usr/bin/python3",
            "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/bin/python3.9",
            "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/Resources/Python.app/Contents/MacOS/Python",
        ],
        "strict process allowlist contains an unexpected executable",
    )
    sandbox = lane.sandbox_profile([Path(value) for value in allowed])
    require(
        provenance["sandboxProfileSha256"]
        == hashlib.sha256(sandbox.encode()).hexdigest()
        and provenance["sandboxNetworkOutbound"] == "denied"
        and provenance["command"]
        == [
            "/usr/bin/sandbox-exec",
            "-p",
            sandbox,
            *lane.command(upstream, node),
        ],
        "strict sandbox policy or official command differs",
    )
    lane.strict_report_case(result)
    require(
        strict["testStatus"] == provenance["testStatus"] == "expected"
        and strict["exitCode"] == provenance["exitCode"] == 0,
        "strict test did not pass exactly once",
    )
    process = lane.strict_process_receipt(
        process_receipt,
        run_id=run_id,
        mode=mode,
        executable=upstream / lane.ASSET,
        node=node,
    )
    lane.strict_launch_receipt(
        launch_receipt,
        mode=mode,
        run_id=run_id,
        manifest_sha256=strict["manifestSha256"],
        executable=upstream / lane.ASSET,
        process_event=process,
        artifact_sha256=strict["candidateArtifactSha256"],
    )
    require(
        isinstance(strict["officialCliPid"], int)
        and strict["officialCliPid"] > 1
        and process["harnessPid"] == process["ancestry"][-1]["pid"]
        and process["harnessPid"] != strict["officialCliPid"]
        and any(
            entry.get("pid") == strict["officialCliPid"]
            and entry.get("executable") == str(node)
            for entry in process["ancestry"]
        ),
        "strict process chain excludes the selected official CLI process",
    )
    proof = strict["processOwnershipProof"]
    require(
        isinstance(proof, dict) and set(proof) == PROOF_FIELDS,
        "strict process proof fields differ",
    )
    require(
        proof
        == {
            "publicMethod": "chromium.launch",
            "guardPid": process["guardPid"],
            "launcherPid": process["guardParentPid"],
            "childPid": process["childPid"],
            "cliPid": strict["officialCliPid"],
            "harnessPid": process["harnessPid"],
            "browserProfilePath": process["browserProfilePath"],
            "browserProfileRemoved": True,
            "childReaped": True,
        },
        "strict public launch process/profile proof differs",
    )
    require(
        strict["fixtureAdapted"] is True
        and strict["unadaptedOfficialPass"] is False
        and strict["upstreamAssertionsModified"] is False
        and strict["fixtureAdaptationEligible"] is True
        and strict["publicApiParityEvidence"] is True
        and strict["diagnosticOnly"] is False
        and strict["ledgerEligible"] is True,
        "strict ledger eligibility differs from the verified source and ownership proof",
    )
    return strict


def verify_pair(candidate_path: Path, oracle_path: Path) -> dict[str, dict[str, Any]]:
    candidate = verify_strict(candidate_path)
    oracle = verify_strict(oracle_path)
    require(
        candidate["mode"] == "candidate"
        and oracle["mode"] == "oracle"
        and candidate_path != oracle_path
        and candidate["result"] != oracle["result"]
        and candidate["runId"] != oracle["runId"]
        and candidate["manifestSha256"] == oracle["manifestSha256"]
        and candidate["upstreamSources"] == oracle["upstreamSources"]
        and candidate["upstreamArtifacts"] == oracle["upstreamArtifacts"]
        and candidate["nodeSha256"] == oracle["nodeSha256"]
        and candidate["suite"] == oracle["suite"]
        and candidate["specId"] == oracle["specId"]
        and candidate["selector"] == oracle["selector"]
        and candidate["processOwnershipProof"]["childPid"]
        != oracle["processOwnershipProof"]["childPid"],
        "strict Core evidence is not a distinct same-case candidate/oracle pair",
    )
    return {"candidate": candidate, "oracle": oracle}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("candidate_strict", type=Path)
    parser.add_argument("oracle_strict", type=Path, nargs="?")
    args = parser.parse_args()
    try:
        if args.oracle_strict:
            pair = verify_pair(args.candidate_strict, args.oracle_strict)
            print(f"verified strict Core dummy-node pair {pair['candidate']['specId']}")
        else:
            strict = verify_strict(args.candidate_strict)
            print(
                f"verified strict Core dummy-node {strict['mode']} {strict['specId']}"
            )
    except (OSError, ValueError, TypeError, KeyError, json.JSONDecodeError) as error:
        parser.error(f"strict Core dummy-node evidence invalid: {error}")


if __name__ == "__main__":
    main()
