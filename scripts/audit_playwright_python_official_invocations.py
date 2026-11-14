#!/usr/bin/env python3
"""Classify every pinned Playwright Python invocation without changing outcomes."""

from __future__ import annotations

import argparse
import ast
import hashlib
import json
import re
import subprocess
from collections import Counter
from collections.abc import Iterable
from dataclasses import dataclass
from pathlib import Path
from typing import Any, TypedDict, cast

PARAMETERS = re.compile(r"\[.*\]$")
INTERNAL_SELECTOR = re.compile(r"\binternal:[A-Za-z0-9_-]+")


class Manifest(TypedDict):
    schema: int
    repository: str
    tag: str
    commit: str


class ApplicabilityRule(TypedDict):
    id: str
    applicability: str
    classification: str
    reason: str


@dataclass(frozen=True, order=True)
class Dependency:
    symbol: str
    usage: str
    line: int

    def as_json(self) -> dict[str, object]:
        return {"symbol": self.symbol, "usage": self.usage, "line": self.line}


def _stable_json_line(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def _nested_counts(
    counts: dict[str, Counter[str]],
) -> dict[str, dict[str, int]]:
    return {name: dict(sorted(outcomes.items())) for name, outcomes in sorted(counts.items())}


def _file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _git(upstream: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(upstream), *args],
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout.strip()


def _verify_upstream(upstream: Path, manifest: Manifest) -> None:
    if _git(upstream, "rev-parse", "HEAD") != manifest["commit"]:
        raise SystemExit("Playwright Python checkout does not match the pinned commit")
    if _git(upstream, "status", "--porcelain", "--untracked-files=no"):
        raise SystemExit("Playwright Python checkout has tracked source changes")


def _verify_inventory(path: Path, expected_commit: str) -> tuple[int, str]:
    summary_path = path.with_suffix(".summary.json")
    if not summary_path.is_file():
        raise SystemExit(f"inventory summary is missing: {summary_path}")
    summary = cast(dict[str, Any], json.loads(summary_path.read_text()))
    if summary.get("commit") != expected_commit:
        raise SystemExit("inventory summary does not match the pinned commit")
    digest = _file_sha256(path)
    if summary.get("ledgerSha256") != digest:
        raise SystemExit("inventory bytes do not match the recorded SHA-256")
    expected_records = summary.get("expandedTests")
    if not isinstance(expected_records, int) or expected_records < 1:
        raise SystemExit("inventory summary has no positive expanded-test count")
    return expected_records, digest


def _verify_outcomes(path: Path, expected_records: int) -> str:
    summary_path = path.with_suffix(".summary.json")
    if not summary_path.is_file():
        raise SystemExit(f"outcome summary is missing: {summary_path}")
    summary = cast(dict[str, Any], json.loads(summary_path.read_text()))
    if summary.get("records") != expected_records:
        raise SystemExit("outcome ledger count does not match the inventory")
    digest = _file_sha256(path)
    if summary.get("ledgerSha256") != digest:
        raise SystemExit("outcome ledger bytes do not match the recorded SHA-256")
    return digest


def _records(path: Path) -> Iterable[dict[str, object]]:
    with path.open() as stream:
        for line_number, line in enumerate(stream, start=1):
            try:
                yield cast(dict[str, object], json.loads(line))
            except json.JSONDecodeError as error:
                raise SystemExit(f"ledger line {line_number} is not valid JSON: {error}") from error


def _is_private_playwright_module(module: str) -> bool:
    parts = module.split(".")
    return parts[0] == "playwright" and any(part.startswith("_") for part in parts[1:])


def _private_import_bindings(tree: ast.Module) -> dict[str, str]:
    bindings: dict[str, str] = {}
    for node in tree.body:
        if isinstance(node, ast.ImportFrom) and node.module:
            if not _is_private_playwright_module(node.module):
                continue
            for alias in node.names:
                local_name = alias.asname or alias.name
                bindings[local_name] = f"{node.module}.{alias.name}"
        elif isinstance(node, ast.Import):
            for alias in node.names:
                if _is_private_playwright_module(alias.name):
                    local_name = alias.asname or alias.name.split(".", maxsplit=1)[0]
                    bindings[local_name] = alias.name
    return bindings


def _locally_defined_private_attributes(tree: ast.Module) -> set[str]:
    return {
        child.name
        for node in tree.body
        if isinstance(node, ast.ClassDef)
        for child in node.body
        if isinstance(child, (ast.FunctionDef, ast.AsyncFunctionDef)) and child.name.startswith("_")
    }


class _UsageCollector(ast.NodeVisitor):
    def __init__(
        self,
        private_bindings: dict[str, str],
        local_private_attributes: set[str],
    ) -> None:
        self._private_bindings = private_bindings
        self._local_private_attributes = local_private_attributes
        self._mode = "runtime"
        self.dependencies: set[Dependency] = set()
        self.runtime_names: set[str] = set()

    def collect_function(self, node: ast.FunctionDef | ast.AsyncFunctionDef) -> None:
        for decorator in node.decorator_list:
            self.visit(decorator)
        for default in (*node.args.defaults, *node.args.kw_defaults):
            if default is not None:
                self.visit(default)
        annotations = [
            argument.annotation
            for argument in (
                *node.args.posonlyargs,
                *node.args.args,
                *node.args.kwonlyargs,
            )
            if argument.annotation is not None
        ]
        if node.args.vararg and node.args.vararg.annotation is not None:
            annotations.append(node.args.vararg.annotation)
        if node.args.kwarg and node.args.kwarg.annotation is not None:
            annotations.append(node.args.kwarg.annotation)
        if node.returns is not None:
            annotations.append(node.returns)
        for annotation in annotations:
            self._visit_annotation(annotation)
        for statement in node.body:
            self.visit(statement)

    def _visit_annotation(self, node: ast.AST) -> None:
        previous = self._mode
        self._mode = "type-annotation"
        self.visit(node)
        self._mode = previous

    def visit_FunctionDef(self, node: ast.FunctionDef) -> None:
        self.collect_function(node)

    def visit_AsyncFunctionDef(self, node: ast.AsyncFunctionDef) -> None:
        self.collect_function(node)

    def visit_Lambda(self, node: ast.Lambda) -> None:
        for default in (*node.args.defaults, *node.args.kw_defaults):
            if default is not None:
                self.visit(default)
        self.visit(node.body)

    def visit_AnnAssign(self, node: ast.AnnAssign) -> None:
        self.visit(node.target)
        self._visit_annotation(node.annotation)
        if node.value is not None:
            self.visit(node.value)

    def visit_Name(self, node: ast.Name) -> None:
        if not isinstance(node.ctx, ast.Load):
            return
        if self._mode == "runtime":
            self.runtime_names.add(node.id)
        symbol = self._private_bindings.get(node.id)
        if symbol is not None:
            usage = "private-runtime" if self._mode == "runtime" else self._mode
            self.dependencies.add(Dependency(symbol, usage, node.lineno))

    def visit_Attribute(self, node: ast.Attribute) -> None:
        if (
            self._mode == "runtime"
            and node.attr.startswith("_")
            and not (node.attr.startswith("__") and node.attr.endswith("__"))
            and node.attr not in self._local_private_attributes
        ):
            self.dependencies.add(
                Dependency(f"attribute:{node.attr}", "private-runtime", node.lineno)
            )
        self.generic_visit(node)

    def visit_Constant(self, node: ast.Constant) -> None:
        if self._mode != "runtime" or not isinstance(node.value, str):
            return
        for selector in INTERNAL_SELECTOR.findall(node.value):
            self.dependencies.add(
                Dependency(f"selector-engine:{selector}", "private-runtime", node.lineno)
            )


class ModuleAnalysis:
    def __init__(self, path: Path) -> None:
        self.path = path
        self.tree = ast.parse(path.read_bytes(), filename=str(path))
        self.private_bindings = _private_import_bindings(self.tree)
        self.local_private_attributes = _locally_defined_private_attributes(self.tree)
        self.functions: dict[str, ast.FunctionDef | ast.AsyncFunctionDef] = {}
        self.assignments: dict[str, ast.AST] = {}
        self._index_module()

    @property
    def module_collection_dependencies(self) -> list[str]:
        return sorted(set(self.private_bindings.values()))

    def _index_module(self) -> None:
        for node in self.tree.body:
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                self.functions[node.name] = node
            elif isinstance(node, ast.ClassDef):
                for child in node.body:
                    if isinstance(child, (ast.FunctionDef, ast.AsyncFunctionDef)):
                        self.functions[f"{node.name}::{child.name}"] = child
            elif isinstance(node, (ast.Assign, ast.AnnAssign)):
                value = node.value
                targets = node.targets if isinstance(node, ast.Assign) else [node.target]
                if value is None:
                    continue
                for target in targets:
                    for name in _assigned_names(target):
                        self.assignments[name] = value

    def invocation(
        self, node_id: str
    ) -> tuple[ast.FunctionDef | ast.AsyncFunctionDef, str, list[Dependency]]:
        function_key = _function_key(node_id)
        node = self.functions.get(function_key)
        if node is None:
            raise SystemExit(f"cannot locate collected Python test {node_id} in {self.path}")
        collector = self._collector()
        collector.collect_function(node)
        dependencies = set(collector.dependencies)
        helper_dependencies: set[Dependency] = set()
        visited: set[str] = set()
        for name in collector.runtime_names:
            helper_dependencies.update(self._resolve_helper(name, visited))
        dependencies.update(helper_dependencies)
        normalized = sorted(dependencies)
        classification = _classification(normalized)
        return node, classification, normalized

    def _collector(self) -> _UsageCollector:
        return _UsageCollector(self.private_bindings, self.local_private_attributes)

    def _resolve_helper(self, name: str, visited: set[str]) -> set[Dependency]:
        if name in visited:
            return set()
        visited.add(name)
        node: ast.AST | None = self.assignments.get(name)
        function = self.functions.get(name)
        collector = self._collector()
        if node is not None:
            collector.visit(node)
        elif function is not None and not name.startswith("test_"):
            collector.collect_function(function)
        else:
            return set()
        dependencies = {
            Dependency(dependency.symbol, "private-test-helper", dependency.line)
            for dependency in collector.dependencies
        }
        for referenced_name in collector.runtime_names:
            dependencies.update(self._resolve_helper(referenced_name, visited))
        return dependencies


def _assigned_names(node: ast.AST) -> list[str]:
    if isinstance(node, ast.Name):
        return [node.id]
    if isinstance(node, (ast.Tuple, ast.List)):
        return [name for child in node.elts for name in _assigned_names(child)]
    return []


def _function_key(node_id: str) -> str:
    parts = node_id.split("::")
    if len(parts) < 2:
        raise SystemExit(f"invalid collected Python test ID: {node_id}")
    parts[-1] = PARAMETERS.sub("", parts[-1])
    return "::".join(parts[1:])


def _base_id(node_id: str) -> str:
    parts = node_id.split("::")
    if len(parts) < 2:
        raise SystemExit(f"invalid collected Python test ID: {node_id}")
    parts[-1] = PARAMETERS.sub("", parts[-1])
    return "::".join(parts)


def _pytest_parameters(node_id: str, node: ast.FunctionDef | ast.AsyncFunctionDef) -> list[str]:
    positional = [argument.arg for argument in (*node.args.posonlyargs, *node.args.args)]
    if len(node_id.split("::")) > 2 and positional[:1] in (["self"], ["cls"]):
        positional = positional[1:]
    parameters = list(positional)
    if node.args.vararg is not None:
        parameters.append(f"*{node.args.vararg.arg}")
    parameters.extend(argument.arg for argument in node.args.kwonlyargs)
    if node.args.kwarg is not None:
        parameters.append(f"**{node.args.kwarg.arg}")
    return parameters


def _classification(dependencies: list[Dependency]) -> str:
    usages = {dependency.usage for dependency in dependencies}
    if "private-runtime" in usages:
        return "implementation-internal"
    if "private-test-helper" in usages:
        return "public-api-with-private-test-helper"
    if "type-annotation" in usages:
        return "public-api-with-private-type-annotation"
    return "public-api-candidate"


def _load_applicability_policy(path: Path) -> dict[str, ApplicabilityRule]:
    payload = cast(dict[str, Any], json.loads(path.read_text()))
    if payload.get("schema") != 1:
        raise SystemExit("unsupported Python invocation-applicability schema")
    raw_rules_value = payload.get("rules")
    if not isinstance(raw_rules_value, list):
        raise SystemExit("Python invocation-applicability policy lacks rules")
    raw_rules = cast(list[object], raw_rules_value)
    rules: dict[str, ApplicabilityRule] = {}
    for raw_rule in raw_rules:
        if not isinstance(raw_rule, dict):
            raise SystemExit("Python invocation-applicability rule is not an object")
        rule_payload = cast(dict[str, object], raw_rule)
        rule_id = rule_payload.get("id")
        applicability = rule_payload.get("applicability")
        classification = rule_payload.get("classification")
        reason = rule_payload.get("reason")
        if not isinstance(rule_id, str) or not rule_id:
            raise SystemExit("Python invocation-applicability rule lacks an ID")
        if rule_id in rules:
            raise SystemExit(f"duplicate Python applicability rule: {rule_id}")
        if applicability not in {"applicable", "non-applicable"}:
            raise SystemExit(f"invalid applicability for {rule_id}: {applicability}")
        if not isinstance(classification, str) or not classification:
            raise SystemExit(f"invalid classification for {rule_id}")
        if not isinstance(reason, str) or not reason.strip():
            raise SystemExit(f"missing applicability reason for {rule_id}")
        if applicability == "non-applicable":
            if classification != "implementation-internal":
                raise SystemExit(
                    f"non-applicable rule must remain implementation-internal: {rule_id}"
                )
        elif not classification.startswith("public-api-with-private-"):
            raise SystemExit(
                f"applicable private rule must retain a qualified public class: {rule_id}"
            )
        rules[rule_id] = cast(ApplicabilityRule, rule_payload)
    return rules


def _reviewed_classification(
    node_id: str,
    syntactic_classification: str,
    policy: dict[str, ApplicabilityRule],
) -> tuple[str, str, str | None]:
    rule = policy.get(_base_id(node_id))
    if syntactic_classification == "implementation-internal":
        if rule is None:
            raise SystemExit(f"direct private dependency lacks an applicability rule: {node_id}")
        return rule["classification"], rule["applicability"], rule["reason"]
    if rule is not None:
        raise SystemExit(
            f"applicability rule no longer points to a direct private dependency: {node_id}"
        )
    return syntactic_classification, "applicable", None


def main() -> None:
    root = Path(__file__).parents[1]
    parser = argparse.ArgumentParser()
    parser.add_argument("upstream", type=Path)
    parser.add_argument(
        "--manifest",
        type=Path,
        default=root / "packages/pwright-python/tests/official/manifest.json",
    )
    parser.add_argument(
        "--inventory",
        type=Path,
        default=root / ".work/official-conformance/playwright-python-v1.58.0-profiled.jsonl",
    )
    parser.add_argument(
        "--applicability-policy",
        type=Path,
        default=root / "packages/pwright-python/tests/official/invocation-applicability.json",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=root / ".work/official-conformance/"
        "playwright-python-v1.58.0-profiled-invocation-classifications.jsonl",
    )
    parser.add_argument(
        "--outcomes",
        type=Path,
        help="optional complete outcome ledger to cross-tabulate without modifying it",
    )
    args = parser.parse_args()

    upstream = args.upstream.resolve()
    manifest = cast(Manifest, json.loads(args.manifest.read_text()))
    if manifest["schema"] != 1:
        raise SystemExit(f"unsupported manifest schema: {manifest['schema']}")
    _verify_upstream(upstream, manifest)

    inventory = args.inventory.resolve()
    expected_records, inventory_digest = _verify_inventory(inventory, manifest["commit"])
    applicability_policy_path = args.applicability_policy.resolve()
    applicability_policy = _load_applicability_policy(applicability_policy_path)
    applicability_policy_digest = _file_sha256(applicability_policy_path)
    outcomes = args.outcomes.resolve() if args.outcomes is not None else None
    outcome_digest = _verify_outcomes(outcomes, expected_records) if outcomes is not None else None
    outcome_records = iter(_records(outcomes)) if outcomes is not None else None

    analyses: dict[str, ModuleAnalysis] = {}
    output = args.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    classification_counts: Counter[str] = Counter()
    syntactic_classification_counts: Counter[str] = Counter()
    applicability_counts: Counter[str] = Counter()
    dependency_counts: Counter[str] = Counter()
    pytest_parameter_counts: Counter[str] = Counter()
    pytest_parameter_applicability_counts: dict[str, Counter[str]] = {}
    pytest_parameter_outcome_counts: dict[str, Counter[str]] = {}
    parameterless_invocations = 0
    parameterless_applicability_counts: Counter[str] = Counter()
    parameterless_outcome_counts: Counter[str] = Counter()
    outcome_classification_counts: Counter[str] = Counter()
    outcome_applicability_counts: Counter[str] = Counter()
    digest = hashlib.sha256()
    observed_ids: set[str] = set()
    reviewed_base_ids: set[str] = set()
    observed = 0
    with output.open("wb") as stream:
        for record in _records(inventory):
            node_id = cast(str, record.get("id"))
            source_file = cast(str, record.get("file"))
            if not node_id or not source_file:
                raise SystemExit("inventory record lacks id or file")
            if node_id in observed_ids:
                raise SystemExit(f"inventory repeats expanded test ID: {node_id}")
            observed_ids.add(node_id)
            analysis = analyses.get(source_file)
            if analysis is None:
                source_path = upstream / source_file
                if not source_path.is_file():
                    raise SystemExit(f"inventory source is missing: {source_file}")
                analysis = ModuleAnalysis(source_path)
                analyses[source_file] = analysis
            node, syntactic_classification, dependencies = analysis.invocation(node_id)
            classification, applicability, applicability_reason = _reviewed_classification(
                node_id, syntactic_classification, applicability_policy
            )
            pytest_parameters = _pytest_parameters(node_id, node)
            pytest_parameter_counts.update(pytest_parameters)
            for parameter in pytest_parameters:
                pytest_parameter_applicability_counts.setdefault(parameter, Counter())[
                    applicability
                ] += 1
            if not pytest_parameters:
                parameterless_invocations += 1
                parameterless_applicability_counts[applicability] += 1
            if syntactic_classification == "implementation-internal":
                reviewed_base_ids.add(_base_id(node_id))

            if outcome_records is not None:
                try:
                    outcome_record = next(outcome_records)
                except StopIteration as error:
                    raise SystemExit("outcome ledger ended before the inventory") from error
                if outcome_record.get("id") != node_id:
                    raise SystemExit(
                        "outcome ledger order differs from the inventory: "
                        f"{outcome_record.get('id')} != {node_id}"
                    )
                outcome = cast(str, outcome_record.get("outcome"))
                outcome_classification_counts[f"{outcome}:{classification}"] += 1
                outcome_applicability_counts[f"{outcome}:{applicability}"] += 1
                for parameter in pytest_parameters:
                    pytest_parameter_outcome_counts.setdefault(parameter, Counter())[outcome] += 1
                if not pytest_parameters:
                    parameterless_outcome_counts[outcome] += 1

            audit_record: dict[str, object] = {
                "id": node_id,
                "file": source_file,
                "api": record.get("api"),
                "sourceStartLine": node.lineno,
                "sourceEndLine": node.end_lineno,
                "pytestParameters": pytest_parameters,
                "sourceClassification": record.get("sourceClassification"),
                "moduleCollectionDependencies": analysis.module_collection_dependencies,
                "syntacticClassification": syntactic_classification,
                "invocationClassification": classification,
                "applicability": applicability,
                "applicabilityReason": applicability_reason,
                "privateDependencies": [dependency.as_json() for dependency in dependencies],
            }
            payload = _stable_json_line(audit_record)
            stream.write(payload)
            digest.update(payload)
            classification_counts[classification] += 1
            syntactic_classification_counts[syntactic_classification] += 1
            applicability_counts[applicability] += 1
            dependency_counts.update(
                f"{dependency.usage}:{dependency.symbol}" for dependency in dependencies
            )
            observed += 1

    if observed != expected_records:
        raise SystemExit(
            "classified invocation count changed unexpectedly: "
            f"expected={expected_records} observed={observed}"
        )
    if reviewed_base_ids != set(applicability_policy):
        raise SystemExit(
            "Python applicability policy coverage differs from the direct-private "
            f"tests: reviewed={sorted(reviewed_base_ids)} "
            f"policy={sorted(applicability_policy)}"
        )
    if outcome_records is not None:
        try:
            next(outcome_records)
        except StopIteration:
            pass
        else:
            raise SystemExit("outcome ledger has records after the inventory ended")

    summary = {
        "schema": 1,
        "repository": manifest["repository"],
        "tag": manifest["tag"],
        "commit": manifest["commit"],
        "inventory": str(inventory),
        "inventorySha256": inventory_digest,
        "applicabilityPolicy": str(applicability_policy_path),
        "applicabilityPolicySha256": applicability_policy_digest,
        "records": observed,
        "sourceModules": len(analyses),
        "syntacticClassificationCounts": dict(sorted(syntactic_classification_counts.items())),
        "invocationClassificationCounts": dict(sorted(classification_counts.items())),
        "applicabilityCounts": dict(sorted(applicability_counts.items())),
        "privateDependencyCounts": dict(sorted(dependency_counts.items())),
        "pytestParameterCounts": dict(sorted(pytest_parameter_counts.items())),
        "pytestParameterApplicabilityCounts": _nested_counts(pytest_parameter_applicability_counts),
        "pytestParameterOutcomeCounts": _nested_counts(pytest_parameter_outcome_counts),
        "parameterlessInvocations": parameterless_invocations,
        "parameterlessApplicabilityCounts": dict(
            sorted(parameterless_applicability_counts.items())
        ),
        "parameterlessOutcomeCounts": dict(sorted(parameterless_outcome_counts.items())),
        "outcomeLedger": str(outcomes) if outcomes is not None else None,
        "outcomeLedgerSha256": outcome_digest,
        "outcomeClassificationCounts": dict(sorted(outcome_classification_counts.items())),
        "outcomeApplicabilityCounts": dict(sorted(outcome_applicability_counts.items())),
        "ledgerSha256": digest.hexdigest(),
        "ledger": str(output),
    }
    summary_path = output.with_suffix(".summary.json")
    summary_path.write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
