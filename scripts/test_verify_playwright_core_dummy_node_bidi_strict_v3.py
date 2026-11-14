"""Pure mutation checks for browserless BiDi official evidence validation."""

from __future__ import annotations

import copy
import json
import os
import runpy
import signal
from pathlib import Path

import pytest
import run_playwright_core_dummy_node_bidi_strict_v3 as capture
import verify_playwright_core_dummy_node_bidi_strict_v3 as verifier

UPSTREAM = Path("/pinned/playwright-v1.58.0")
CASE = verifier.CASE_MAP["bidi-chromium-library"]
FIREFOX = verifier.CASE_MAP["moz-firefox-library"]
RUN_ID = "a" * 32
OUTPUT = verifier.TASK_RESULTS / f"synthetic-{RUN_ID}.output"


def report(case: dict[str, object], *, passing: bool) -> dict[str, object]:
    error = {
        "location": {
            "file": str(UPSTREAM / "tests/library/browsertype-launch.spec.ts"),
            "line": 72,
            "column": 32,
        },
        "message": 'Expected substring: "Browser logs:"',
    }
    test = {
        "projectId": case["project"],
        "expectedStatus": "passed",
        "status": "expected" if passing else "unexpected",
        "annotations": [],
        "results": [
            {
                "status": "passed" if passing else "failed",
                "retry": 0,
                "workerIndex": 0,
                "errors": [] if passing else [error],
            }
        ],
    }
    spec = {
        "id": case["specId"],
        "file": "library/browsertype-launch.spec.ts",
        "line": 64,
        "column": 3,
        "title": verifier.TITLE,
        "ok": passing,
        "tests": [test],
    }
    return {
        "suites": [{"specs": [spec], "suites": []}],
        "stats": {
            "expected": int(passing),
            "unexpected": int(not passing),
            "skipped": 0,
            "flaky": 0,
        },
        "config": {
            "workers": 1,
            "projects": [
                {"id": case["project"], "retries": 0, "outputDir": str(OUTPUT)}
            ],
        },
        "errors": [],
    }


def strict(case: dict[str, object], mode: str, *, passing: bool) -> dict[str, object]:
    return {
        "upstream": str(UPSTREAM),
        "runId": RUN_ID,
        "officialCliPid": 100,
        "harnessPid": 50,
        "node": "/opt/homebrew/bin/node",
        "testStatus": "expected" if passing else "unexpected",
    }


def manifest() -> dict[str, str]:
    return {"candidateArtifactSha256": "b" * 64}


def launch_events(case: dict[str, object], mode: str) -> list[dict[str, object]]:
    event = {
        "mode": mode,
        "project": case["project"],
        "engine": case["engine"],
        "runId": RUN_ID,
        "publicMethod": f"{case['engine']}.launch",
        "executablePath": str(UPSTREAM / verifier.ASSET),
        "assetSha256": verifier.ASSET_SHA256,
        "manifestSha256": verifier.MANIFEST_SHA256,
        "workerPid": 200,
        "workerParentPid": 100,
        "candidateProfile": verifier.PROFILE if mode == "candidate" else None,
        "candidateArtifactSha256": "b" * 64 if mode == "candidate" else None,
        "candidatePublicLaunchInherited": True if mode == "candidate" else None,
    }
    return [{"event": name, **event} for name in ("launch-invoked", "launch-failed")]


def process_event(case: dict[str, object], mode: str) -> dict[str, object]:
    chain = [
        {"pid": 310, "ppid": 200, "uid": os.getuid(), "executable": "/usr/bin/python3"},
        {
            "pid": 200,
            "ppid": 100,
            "uid": os.getuid(),
            "executable": "/opt/homebrew/bin/node",
        },
        {
            "pid": 100,
            "ppid": 50,
            "uid": os.getuid(),
            "executable": "/opt/homebrew/bin/node",
        },
        {"pid": 50, "ppid": 1, "uid": os.getuid(), "executable": "/usr/bin/python3"},
    ]
    event = {
        "event": "dummy-node-exited",
        "mode": mode,
        "runId": RUN_ID,
        "asset": str(UPSTREAM / verifier.ASSET),
        "exitCode": 1,
        "guardPid": 310,
        "guardParentPid": 200,
        "childPid": 400,
        "childReaped": True,
        "childSnapshot": {
            "pid": 400,
            "ppid": 310,
            "uid": os.getuid(),
            "executable": "/opt/homebrew/bin/node",
        },
        "ancestry": chain,
        "harnessPid": 50,
    }
    stat = {"device": 1, "inode": 2, "uid": os.getuid()}
    if case["engine"] == "chromium":
        profile = (
            "/tmp/playwright_chromiumdev_profile-test"
            if mode == "oracle"
            else "/tmp/pwright-node-bidi-startup-test"
        )
        return {
            **event,
            "candidateProfile": verifier.PROFILE if mode == "candidate" else None,
            "browserProfilePath": profile,
            "browserProfileStat": stat,
            "browserArguments": [f"--user-data-dir={profile}"],
        }
    profile = (
        "/tmp/playwright_firefoxdev_profile-test"
        if mode == "oracle"
        else "/tmp/pwright-node-bidi-startup-test"
    )
    return {
        **event,
        "project": case["project"],
        "engine": "firefox",
        "browserProfile": {"path": profile, **stat},
        "browserArguments": ["--profile", profile],
    }


def test_guard_accepts_only_live_owned_two_dash_profile(tmp_path: Path) -> None:
    # Arrange
    profile = tmp_path / "playwright_firefoxdev_profile-test"
    profile.mkdir()
    guard = runpy.run_path(
        str(verifier.ROOT / "tests/official/dummy-core-bidi-firefox-guard/node")
    )

    # Act
    data = guard["profile_data_from_args"](
        ["--remote-debugging-port=0", "--profile", str(profile)]
    )

    # Assert
    assert data["path"] == str(profile)
    assert data["uid"] == os.getuid()
    assert data["device"] == profile.stat().st_dev
    assert data["inode"] == profile.stat().st_ino


@pytest.mark.parametrize(
    "options",
    [
        ["-profile", "{path}"],
        ["--profile", "{path}", "--profile", "{path}"],
        ["--profile={path}"],
        ["--profile", "{path}", "-profile", "{path}"],
    ],
)
def test_guard_rejects_unpinned_profile_spellings(
    tmp_path: Path, options: list[str]
) -> None:
    # Arrange
    profile = tmp_path / "playwright_firefoxdev_profile-test"
    profile.mkdir()
    arguments = [option.replace("{path}", str(profile)) for option in options]
    guard = runpy.run_path(
        str(verifier.ROOT / "tests/official/dummy-core-bidi-firefox-guard/node")
    )

    # Act / Assert
    with pytest.raises(ValueError, match="profile"):
        guard["profile_data_from_args"](arguments)


def test_guard_rejects_non_live_profile(tmp_path: Path) -> None:
    # Arrange
    missing = tmp_path / "missing"
    guard = runpy.run_path(
        str(verifier.ROOT / "tests/official/dummy-core-bidi-firefox-guard/node")
    )

    # Act / Assert
    with pytest.raises(ValueError, match="live owned"):
        guard["profile_data_from_args"](["--profile", str(missing)])


def test_guard_rejects_linked_profile(tmp_path: Path) -> None:
    # Arrange
    profile = tmp_path / "playwright_firefoxdev_profile-test"
    profile.mkdir()
    linked = tmp_path / "linked-profile"
    linked.symlink_to(profile, target_is_directory=True)
    guard = runpy.run_path(
        str(verifier.ROOT / "tests/official/dummy-core-bidi-firefox-guard/node")
    )

    # Act / Assert
    with pytest.raises(ValueError, match="live owned"):
        guard["profile_data_from_args"](["--profile", str(linked)])


def profile_event(
    case: dict[str, object], mode: str, process: dict[str, object]
) -> dict[str, object]:
    if case["engine"] == "chromium":
        path = process["browserProfilePath"]
        stat = process["browserProfileStat"]
    else:
        data = process["browserProfile"]
        path = data["path"]
        stat = data
    return {
        "event": "profile-created",
        "mode": mode,
        "project": case["project"],
        "engine": case["engine"],
        "runId": RUN_ID,
        "workerPid": 200,
        "path": path,
        "device": stat["device"],
        "inode": stat["inode"],
        "uid": stat["uid"],
    }


@pytest.mark.parametrize(
    "mode,passing", [("candidate", False), ("candidate", True), ("oracle", True)]
)
def test_accepts_truthful_official_status(mode: str, passing: bool) -> None:
    verifier.check_report(
        report(CASE, passing=passing),
        CASE,
        mode,
        UPSTREAM,
        "expected" if passing else "unexpected",
        OUTPUT,
    )


def test_rejects_oracle_red() -> None:
    with pytest.raises(ValueError, match="outcome"):
        verifier.check_report(
            report(CASE, passing=False), CASE, "oracle", UPSTREAM, "unexpected", OUTPUT
        )


@pytest.mark.parametrize("mutation", ["spec", "attempt", "assertion", "project"])
def test_rejects_official_report_mutation(mutation: str) -> None:
    value = report(CASE, passing=False)
    spec = value["suites"][0]["specs"][0]
    if mutation == "spec":
        spec["id"] = "wrong"
    elif mutation == "attempt":
        spec["tests"][0]["results"].append(
            copy.deepcopy(spec["tests"][0]["results"][0])
        )
    elif mutation == "assertion":
        spec["tests"][0]["results"][0]["errors"][0]["location"]["line"] = 71
    else:
        spec["tests"][0]["projectId"] = "wrong"
    with pytest.raises(ValueError):
        verifier.check_report(value, CASE, "candidate", UPSTREAM, "unexpected", OUTPUT)


def test_rejects_launch_asset_and_worker_mutations() -> None:
    for key, value in (
        ("executablePath", "/Applications/Chrome.app"),
        ("workerParentPid", 999),
        ("candidatePublicLaunchInherited", False),
    ):
        events = launch_events(CASE, "candidate")
        events[0][key] = value
        with pytest.raises(ValueError, match="launch"):
            verifier.check_launch_events(
                events,
                CASE,
                "candidate",
                strict(CASE, "candidate", passing=False),
                manifest(),
            )


def test_rejects_official_report_output_dir_escape() -> None:
    value = report(CASE, passing=False)
    value["config"]["projects"][0]["outputDir"] = str(UPSTREAM / "test-results")
    with pytest.raises(ValueError, match="project"):
        verifier.check_report(value, CASE, "candidate", UPSTREAM, "unexpected", OUTPUT)


def output_strict() -> dict[str, object]:
    return {
        **strict(CASE, "oracle", passing=True),
        "outputDir": str(OUTPUT),
        "outputDirWasAbsentBefore": True,
        "upstreamOutputDir": str(UPSTREAM / "test-results"),
        "processGroupId": 100,
        "processGroupIsolated": True,
        "processGroupEmptyAfter": True,
        "fixturePorts": [8907, 8908, 8909, 8910],
        "fixturePortsFreeBefore": True,
        "fixturePortsFreeAfter": True,
    }


@pytest.mark.parametrize(
    "key,value",
    [
        ("outputDir", str(UPSTREAM / "test-results")),
        ("outputDirWasAbsentBefore", False),
        ("upstreamOutputDir", "/wrong/test-results"),
        ("processGroupId", 999),
        ("processGroupIsolated", False),
        ("processGroupEmptyAfter", False),
        ("fixturePortsFreeBefore", False),
        ("fixturePortsFreeAfter", False),
    ],
)
def test_rejects_output_and_resource_attestation_mutation(
    key: str, value: object
) -> None:
    item = output_strict()
    item[key] = value
    with pytest.raises(ValueError, match="isolation"):
        verifier.check_output_contract(
            item, verifier.TASK_RESULTS / "synthetic.json", UPSTREAM
        )


def test_output_contract_rejects_result_outside_task_scope() -> None:
    verifier.check_output_contract(
        output_strict(), verifier.TASK_RESULTS / "synthetic.json", UPSTREAM
    )
    with pytest.raises(ValueError, match="escaped"):
        verifier.check_output_contract(
            output_strict(), UPSTREAM / "test-results/synthetic.json", UPSTREAM
        )


@pytest.mark.parametrize(
    "bind,host",
    [
        ("192.168.1.9", "host.tail8f3b66.ts.net"),
        ("100.95.116.72", "host.local"),
    ],
)
def test_rejects_non_tailnet_binding(bind: str, host: str) -> None:
    with pytest.raises(ValueError, match="Tailscale"):
        verifier.check_tailscale_binding(bind, host)


def test_group_wait_reaps_exited_leader_before_ps(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    class Leader:
        reaped = False

        def poll(self) -> int:
            self.reaped = True
            return -signal.SIGTERM

    leader = Leader()
    monkeypatch.setattr(
        capture, "process_group_members", lambda _pgid: set() if leader.reaped else {42}
    )
    assert capture.wait_group_empty(42, 0.0, leader) is True
    assert leader.reaped is True


def test_timeout_group_stop_reaps_signaled_leader(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    class Leader:
        signaled = False
        reaped = False

        def poll(self) -> int | None:
            if self.signaled:
                self.reaped = True
                return -signal.SIGTERM
            return None

    leader = Leader()
    signals: list[tuple[int, signal.Signals]] = []

    def killpg(pgid: int, sig: signal.Signals) -> None:
        signals.append((pgid, sig))
        leader.signaled = True

    monkeypatch.setattr(capture.os, "killpg", killpg)
    monkeypatch.setattr(
        capture, "process_group_members", lambda _pgid: set() if leader.reaped else {42}
    )
    capture.stop_owned_group(42, leader)
    assert signals == [(42, signal.SIGTERM)]
    assert leader.reaped is True


@pytest.mark.parametrize(
    "mode,passing", [("candidate", False), ("candidate", True), ("oracle", True)]
)
def test_every_lane_requires_real_dummy_exit(mode: str, passing: bool) -> None:
    with pytest.raises(ValueError, match="exit-1"):
        verifier.check_process_events(
            [], [], CASE, mode, strict(CASE, mode, passing=passing), 200
        )


@pytest.mark.parametrize("case", [CASE, FIREFOX])
def test_accepts_oracle_process_receipts(case: dict[str, object]) -> None:
    process = process_event(case, "oracle")
    verifier.check_process_events(
        [profile_event(case, "oracle", process)],
        [process],
        case,
        "oracle",
        strict(case, "oracle", passing=True),
        200,
    )


@pytest.mark.parametrize("mutation", ["one-dash", "duplicate", "project"])
def test_rejects_firefox_guard_receipt_mutation(mutation: str) -> None:
    # Arrange
    process = process_event(FIREFOX, "oracle")
    profile = profile_event(FIREFOX, "oracle", process)
    if mutation == "one-dash":
        process["browserArguments"][0] = "-profile"
    elif mutation == "duplicate":
        process["browserArguments"].extend(process["browserArguments"])
    else:
        process["project"] = "moz-firefox-beta-library"

    # Act / Assert
    with pytest.raises(ValueError, match="Firefox"):
        verifier.check_process_events(
            [profile],
            [process],
            FIREFOX,
            "oracle",
            strict(FIREFOX, "oracle", passing=True),
            200,
        )


def test_rejects_unremoved_oracle_profile() -> None:
    process = process_event(CASE, "oracle")
    with pytest.raises(ValueError, match="profile"):
        verifier.check_process_events(
            [profile_event(CASE, "oracle", process)],
            [process],
            CASE,
            "oracle",
            strict(CASE, "oracle", passing=True),
            200,
            profile_removed=False,
        )


@pytest.mark.parametrize("case", [CASE, FIREFOX])
@pytest.mark.parametrize("passing", [False, True])
def test_accepts_candidate_process_receipt(
    case: dict[str, object], passing: bool
) -> None:
    process = process_event(case, "candidate")
    verifier.check_process_events(
        [],
        [process],
        case,
        "candidate",
        strict(case, "candidate", passing=passing),
        200,
    )


@pytest.mark.parametrize("case", [CASE, FIREFOX])
def test_rejects_unremoved_candidate_profile(case: dict[str, object]) -> None:
    process = process_event(case, "candidate")
    with pytest.raises(ValueError, match="profile"):
        verifier.check_process_events(
            [],
            [process],
            case,
            "candidate",
            strict(case, "candidate", passing=True),
            200,
            profile_removed=False,
        )


@pytest.mark.parametrize("case", [CASE, FIREFOX])
def test_rejects_candidate_profile_with_foreign_prefix(case: dict[str, object]) -> None:
    process = process_event(case, "candidate")
    if case["engine"] == "firefox":
        process["browserProfile"]["path"] = "/tmp/pwright-node-other-feature-test"
        process["browserArguments"][1] = "/tmp/pwright-node-other-feature-test"
    else:
        process["browserProfilePath"] = "/tmp/pwright-node-other-feature-test"
        process["browserArguments"][0] = (
            "--user-data-dir=/tmp/pwright-node-other-feature-test"
        )
    with pytest.raises(ValueError, match="profile"):
        verifier.check_process_events(
            [],
            [process],
            case,
            "candidate",
            strict(case, "candidate", passing=True),
            200,
        )


@pytest.mark.parametrize("case", [CASE, FIREFOX])
@pytest.mark.parametrize("mode", ["candidate", "oracle"])
def test_capture_requires_one_exited_process_and_removed_profile(
    tmp_path: Path, case: dict[str, object], mode: str
) -> None:
    # Arrange
    process = process_event(case, mode)
    profile = profile_event(case, mode, process)
    process_path = tmp_path / "process.jsonl"
    profile_path = tmp_path / "profile.jsonl"
    process_path.write_text(json.dumps(process) + "\n")
    profile_path.write_text((json.dumps(profile) + "\n") if mode == "oracle" else "")
    paths = {"processReceipt": process_path, "profileReceipt": profile_path}

    # Act
    outcome = capture.checked_process_and_profile(
        paths, mode, str(case["engine"]), str(case["project"]), RUN_ID
    )

    # Assert
    assert outcome is None


def test_capture_rejects_missing_candidate_process(tmp_path: Path) -> None:
    # Arrange
    process_path = tmp_path / "process.jsonl"
    profile_path = tmp_path / "profile.jsonl"
    process_path.write_text("")
    profile_path.write_text("")

    # Act / Assert
    with pytest.raises(ValueError, match="one process"):
        capture.checked_process_and_profile(
            {"processReceipt": process_path, "profileReceipt": profile_path},
            "candidate",
            "chromium",
            str(CASE["project"]),
            RUN_ID,
        )


def test_capture_rejects_unremoved_candidate_profile(tmp_path: Path) -> None:
    # Arrange
    profile = tmp_path / "pwright-node-bidi-startup-test"
    profile.mkdir()
    process = process_event(CASE, "candidate")
    process["browserProfilePath"] = str(profile)
    process["browserArguments"] = [f"--user-data-dir={profile}"]
    process_path = tmp_path / "process.jsonl"
    profile_path = tmp_path / "profile.jsonl"
    process_path.write_text(json.dumps(process) + "\n")
    profile_path.write_text("")

    # Act / Assert
    with pytest.raises(ValueError, match="not cleaned"):
        capture.checked_process_and_profile(
            {"processReceipt": process_path, "profileReceipt": profile_path},
            "candidate",
            "chromium",
            str(CASE["project"]),
            RUN_ID,
        )


@pytest.mark.parametrize("mutation", ["exit", "ancestry", "asset", "profile"])
def test_rejects_process_receipt_mutation(mutation: str) -> None:
    process = process_event(CASE, "oracle")
    profile = profile_event(CASE, "oracle", process)
    if mutation == "exit":
        process["exitCode"] = 0
    elif mutation == "ancestry":
        process["ancestry"][1]["ppid"] = 999
    elif mutation == "asset":
        process["asset"] = "/Applications/Chrome.app"
    else:
        profile["inode"] = 999
    with pytest.raises(ValueError):
        verifier.check_process_events(
            [profile],
            [process],
            CASE,
            "oracle",
            strict(CASE, "oracle", passing=True),
            200,
        )


def pair_evidence() -> list[dict[str, object]]:
    items = []
    for index, case in enumerate(verifier.CASE_MAP.values()):
        for mode in ("candidate", "oracle"):
            number = len(items) + 1
            items.append(
                {
                    "project": case["project"],
                    "mode": mode,
                    "runId": f"{number:032x}",
                    "officialCliPid": 100 + number,
                    "result": f"/tmp/{number}.json",
                    "upstreamArtifactsSha256": {"runtime": "a" * 64},
                    "nodeSha256": "b" * 64,
                    "manifestSha256": "c" * 64,
                    "upstreamSources": {"source": "d" * 64},
                }
            )
    return items


def test_pairs_require_12_distinct_runs_and_same_sources() -> None:
    items = pair_evidence()
    verifier.check_pairs(items)
    duplicate = copy.deepcopy(items)
    duplicate[1]["runId"] = duplicate[0]["runId"]
    with pytest.raises(ValueError, match="runId"):
        verifier.check_pairs(duplicate)
    drift = copy.deepcopy(items)
    drift[1]["nodeSha256"] = "e" * 64
    with pytest.raises(ValueError, match="same-source"):
        verifier.check_pairs(drift)
    with pytest.raises(ValueError, match="12 lanes"):
        verifier.check_pairs(items[:-1])
