#!/usr/bin/env python3
"""Coding Opus playbooks: read, pick and resolve the strategy the lead follows.

Coding Agent runs the workers; this script only answers "which playbook, which phases, which
model and effort for each tier". Python 3.9+, standard library only.
"""

from __future__ import annotations

import argparse
import copy
import datetime as dt
import functools
import json
import math
import os
import re
import shutil
import sys
import time
import uuid
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

MODEL_TIERS = ("cheap", "standard", "strong")
PASSIVE_DEFAULTS = {"checkEveryMinutes": 15, "spendStepUsd": 5, "prefer": ["codex", "claude"], "askAboveWorkers": 2}


POLICY_VERSION = 1
MAX_WORKERS = 16
ID_PATTERN = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$")
SAFE_NAME_PATTERN = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:/-]{0,127}$")
PROVIDER_ALIASES = {"antigravity": "agy"}
BUILTIN_PROVIDERS = ("codex", "claude", "agy", "kimi")
ATOMIC_REPLACE_ATTEMPTS = 5
ATOMIC_REPLACE_BACKOFF_SECONDS = 0.025

STRATEGIES: Dict[str, Dict[str, Any]] = {
    "auto": {
        "maxWorkers": MAX_WORKERS,
        "maxConcurrent": 4,
        "description": "Choose the smallest strategy supported by the plan.",
    },
    "economy": {
        "maxWorkers": 1,
        "maxConcurrent": 1,
        "description": "Minimize agent usage with one concise worker and no speculative fan-out.",
    },
    "balanced": {
        "maxWorkers": 4,
        "maxConcurrent": 2,
        "description": "Use a small general-purpose batch for independent work.",
    },
    "quality": {
        "maxWorkers": 1,
        "maxConcurrent": 1,
        "description": "Run writer, reviewer, and repair work as separate sequential waves.",
    },
    "research": {
        "maxWorkers": 8,
        "maxConcurrent": 4,
        "description": "Compare independent read-only investigations with compact evidence.",
    },
    "parallel": {
        "maxWorkers": 8,
        "maxConcurrent": 4,
        "description": "Implement disjoint units concurrently; every writer uses a worktree.",
    },
    "rescue": {
        "maxWorkers": 1,
        "maxConcurrent": 1,
        "description": "Resume one interrupted or failed provider session without restarting the work.",
    },
}
DEFAULT_POLICY: Dict[str, Any] = {
    "version": POLICY_VERSION,
    "onboarded": False,
    "backend": "auto",
    "defaultStrategy": "auto",
    "providerSwitch": "ask",
    "providerOrder": list(BUILTIN_PROVIDERS),
    "conserveProviders": [],
    "maxWorkers": MAX_WORKERS,
    "maxConcurrent": 4,
    "budget": {},
    "modelTiers": {},
    "trust": {"allowUnsafe": False, "cwdRoots": [], "workerEnv": []},
}


class PlanError(ValueError):
    pass


def now_iso() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat()


def atomic_write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temp = path.with_name(f".{path.name}.{uuid.uuid4().hex}.tmp")
    try:
        temp.write_text(text, encoding="utf-8")
        for attempt in range(ATOMIC_REPLACE_ATTEMPTS):
            try:
                os.replace(temp, path)
                return
            except PermissionError:
                if attempt == ATOMIC_REPLACE_ATTEMPTS - 1:
                    raise
                time.sleep(ATOMIC_REPLACE_BACKOFF_SECONDS * (2 ** attempt))
    finally:
        if temp.exists():
            temp.unlink()


def atomic_write_json(path: Path, value: Any) -> None:
    atomic_write_text(path, json.dumps(value, indent=2, ensure_ascii=False) + "\n")


def read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise PlanError(f"cannot read {path}: {error}") from error
    except json.JSONDecodeError as error:
        raise PlanError(f"invalid JSON in {path}: {error}") from error


def read_optional_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None


def skill_version() -> Optional[str]:
    try:
        text = (Path(__file__).resolve().parent.parent / "SKILL.md").read_text(encoding="utf-8")
    except OSError:
        return None
    frontmatter = text.split("\n---", 1)[0]
    match = re.search(r'^\s+version:\s*"?([0-9][0-9.]*)"?\s*$', frontmatter, re.MULTILINE)
    return match.group(1) if match else None


def default_policy_path() -> Path:
    configured = os.environ.get("CODING_OPUS_POLICY")
    if configured:
        return Path(configured).expanduser()
    if os.name == "nt":
        base = Path(os.environ.get("APPDATA", Path.home() / "AppData" / "Roaming"))
        path = base / "CodingOpus" / "coding-opus-orchestrate.json"
        migrate_legacy_state(base / "Alethe", path.parent, {"alethe-": "coding-opus-"})
        return path
    base = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config"))
    path = base / "coding-opus" / "orchestrate.json"
    migrate_legacy_state(base / "alethe", path.parent, {"alethe-": "coding-opus-"})
    return path


def migrate_legacy_state(legacy: Path, target: Path, renames: Dict[str, str]) -> None:
    """Primeira execução depois da troca de nome: copia o estado da Alethe para a pasta nova.

    Copia, nunca move: a pasta antiga continua intacta para o app ou a skill Alethe originais.
    Só roda quando a pasta nova ainda não existe e a antiga existe.
    """
    if target.exists() or not legacy.is_dir():
        return
    try:
        shutil.copytree(legacy, target, symlinks=True)
        for child in list(target.iterdir()):
            for old, new in renames.items():
                if child.is_file() and child.name.startswith(old):
                    child.rename(child.with_name(new + child.name[len(old):]))
        current = target / "orchestrations" / "current.json"
        if current.is_file():  # a orquestração atual passa a apontar para a cópia nova
            data = json.loads(current.read_text(encoding="utf-8-sig"))
            folder = str(data.get("folder") or "")
            if folder.lower().startswith(str(legacy).lower()):
                data["folder"] = str(target) + folder[len(str(legacy)):]
                current.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
        (target / "MIGRATED_FROM.txt").write_text(f"Copiado de {legacy}\n", encoding="utf-8")
    except (OSError, ValueError):
        pass  # sem migração, o runner segue com padrões; nada é perdido na pasta antiga


PLAYBOOKS_FILE = "playbooks.yaml"
USER_PLAYBOOKS_FILE = "coding-opus-playbooks.yaml"


def bundled_playbooks_path() -> Path:
    return Path(__file__).resolve().parent.parent / PLAYBOOKS_FILE


def user_playbooks_path(policy_path: Path) -> Path:
    configured = os.environ.get("CODING_OPUS_PLAYBOOKS")
    if configured:
        return Path(configured).expanduser()
    return policy_path.with_name(USER_PLAYBOOKS_FILE)


def effective_playbooks_path(policy_path: Path) -> Path:
    user = user_playbooks_path(policy_path)
    return user if user.is_file() else bundled_playbooks_path()


PLAYBOOKS_VERSION = 1
PLAYBOOK_TOP_FIELDS = {
    "version", "goal", "setup", "principles", "providers", "models", "adapters", "routes",
    "tiers", "quota", "budget", "profile", "profiles", "pickOrder", "playbooks", "routing", "sources", "passive",
}
PLAYBOOK_PHASE_FIELDS = {
    "id", "role", "who", "tier", "provider", "model", "effort", "readOnly", "workers", "isolate",
    "runner", "does", "returns", "limit", "gate", "if", "with", "after", "then", "rules",
}
PLAYBOOK_FIELDS = {"name", "goal", "pickWhen", "saves", "avoidWhen", "source", "next", "phases"}
PLAYBOOK_PROFILE_FIELDS = {"name", "summary", "order", "quota", "tiers"}
PLAYBOOK_PHASE_ENUMS = {
    "who": ("lead", "worker"), "tier": MODEL_TIERS,
    "runner": tuple(name for name in STRATEGIES if name != "auto"), "then": ("next", "recheck", "done"),
}
PLAYBOOK_QUOTA_FIELDS = {
    "criticalThreshold", "stopAt", "whenLow", "onBothCritical", "whenSpare", "spareWithinHours", "spareMinLeft",
    "planEnds",
}
PLAYBOOK_MAX_DEPTH = 100
PLAYBOOK_CACHE_SIZE = 16


class _PlaybookMap(dict):
    def __init__(self, line: int) -> None:
        super().__init__()
        self.line = line
        self.key_lines: Dict[str, int] = {}


class _PlaybookYamlReader:
    def __init__(self, text: str) -> None:
        self.lines: List[Tuple[int, int, str]] = []
        for number, raw in enumerate(text.splitlines(), 1):
            indent = len(raw) - len(raw.lstrip(" \t"))
            if "\t" in raw[:indent]:
                self.fail(number, "tabs are not supported for indentation")
            content = self.uncomment(raw[indent:]).rstrip()
            if content:
                self.lines.append((number, indent, content))
        self.index = 0

    def fail(self, line: int, message: str) -> None:
        raise PlanError(f"line {line}: {message}")

    def uncomment(self, text: str) -> str:
        quote = None
        index = 0
        previous = ""
        while index < len(text):
            char = text[index]
            if quote:
                if char == "\\" and quote == '"':
                    index += 2
                    continue
                if char == quote:
                    if quote == "'" and text[index:index + 2] == "''":
                        index += 2
                        continue
                    quote = None
            elif char == "#" and (index == 0 or text[index - 1].isspace()):
                return text[:index]
            elif char in "\"'" and (not previous or previous in ":,[{-"):
                quote = char
            if not char.isspace():
                previous = char
            index += 1
        return text

    def entry(self, text: str, line: int) -> Tuple[str, str]:
        quote = None
        index = 0
        while index < len(text):
            char = text[index]
            if quote:
                if char == "\\" and quote == '"':
                    index += 2
                    continue
                if char == quote:
                    if quote == "'" and text[index:index + 2] == "''":
                        index += 2
                        continue
                    quote = None
            elif index == 0 and char in "\"'":
                quote = char
            elif char == ":" and (index + 1 == len(text) or text[index + 1].isspace()):
                key = self.value(text[:index].strip(), line)
                if not isinstance(key, str) or not key or key == "<<":
                    self.fail(line, "mapping keys must be non-empty strings; merges are unsupported")
                return key, text[index + 1:].strip()
            index += 1
        self.fail(line, "expected a block mapping entry")

    def value(self, text: str, line: int, depth: int = 0) -> Any:
        value, end = self.flow_value(text, 0, line, depth, False)
        if text[end:].strip():
            self.fail(line, "unexpected text after value")
        return value

    def flow_value(self, text: str, start: int, line: int, depth: int, flow: bool) -> Tuple[Any, int]:
        if depth > PLAYBOOK_MAX_DEPTH:
            self.fail(line, "YAML nesting is too deep")
        index = start
        while index < len(text) and text[index].isspace():
            index += 1
        if index == len(text):
            self.fail(line, "missing scalar value")
        char = text[index]
        if char in "\"'":
            return self.quoted(text, index, line)
        if char in "[{":
            mapping = char == "{"
            closing = "}" if mapping else "]"
            result: Any = _PlaybookMap(line) if mapping else []
            index += 1
            while index < len(text) and text[index].isspace():
                index += 1
            if index < len(text) and text[index] == closing:
                return result, index + 1
            while index < len(text):
                if mapping:
                    if text[index] in "\"'":
                        key, index = self.quoted(text, index, line)
                    else:
                        end = index
                        while end < len(text) and text[end] not in ":,{}[]":
                            end += 1
                        key = self.scalar(text[index:end].strip(), line)
                        index = end
                    while index < len(text) and text[index].isspace():
                        index += 1
                    if not isinstance(key, str) or not key or key == "<<":
                        self.fail(line, "flow mapping keys must be non-empty strings")
                    if index == len(text) or text[index] != ":":
                        self.fail(line, "expected ':' in flow mapping")
                    if key in result:
                        self.fail(line, f"duplicate mapping key {key!r}")
                    value, index = self.flow_value(text, index + 1, line, depth + 1, True)
                    result[key] = value
                    result.key_lines[key] = line
                else:
                    value, index = self.flow_value(text, index, line, depth + 1, True)
                    result.append(value)
                while index < len(text) and text[index].isspace():
                    index += 1
                if index < len(text) and text[index] == closing:
                    return result, index + 1
                if index == len(text) or text[index] != ",":
                    self.fail(line, f"expected ',' or '{closing}'")
                index += 1
                while index < len(text) and text[index].isspace():
                    index += 1
                if index == len(text) or text[index] == closing:
                    self.fail(line, "missing value after ','")
            self.fail(line, f"unclosed flow collection, expected '{closing}'")
        end = index
        while end < len(text) and (not flow or text[end] not in ",]}{"):
            end += 1
        return self.scalar(text[index:end].strip(), line), end

    def quoted(self, text: str, start: int, line: int) -> Tuple[str, int]:
        quote = text[start]
        result = []
        index = start + 1
        escapes = {
            "0": "\0", "a": "\a", "b": "\b", "t": "\t", "n": "\n", "v": "\v", "f": "\f",
            "r": "\r", "e": "\x1b", " ": " ", '"': '"', "/": "/", "\\": "\\",
            "N": "\x85", "_": "\xa0", "L": "\u2028", "P": "\u2029",
        }
        while index < len(text):
            char = text[index]
            if char == quote:
                if quote == "'" and text[index:index + 2] == "''":
                    result.append("'")
                    index += 2
                    continue
                return "".join(result), index + 1
            if char == "\\" and quote == '"':
                index += 1
                if index == len(text):
                    self.fail(line, "unfinished quoted escape")
                escape = text[index]
                if escape in ("x", "u", "U"):
                    count = {"x": 2, "u": 4, "U": 8}[escape]
                    digits = text[index + 1:index + 1 + count]
                    if len(digits) != count or not re.fullmatch(r"[0-9a-fA-F]+", digits):
                        self.fail(line, "invalid Unicode escape")
                    codepoint = int(digits, 16)
                    if codepoint > 0x10ffff or 0xd800 <= codepoint <= 0xdfff:
                        self.fail(line, "invalid Unicode code point")
                    result.append(chr(codepoint))
                    index += count + 1
                    continue
                if escape not in escapes:
                    self.fail(line, f"unsupported escape \\{escape}")
                result.append(escapes[escape])
                index += 1
                continue
            result.append(char)
            index += 1
        self.fail(line, "unterminated quoted string")

    def scalar(self, text: str, line: int) -> Any:
        if not text or text[0] in "&*!|>@%\x60,]}{[?:" or text in ("---", "...", "-"):
            self.fail(line, "unsupported YAML syntax or missing scalar")
        if re.search(r":(?:\s|$)", text):
            self.fail(line, "unexpected mapping in scalar")
        if text in ("true", "false"):
            return text == "true"
        if text.lower() in ("null", "~", "yes", "no", "on", "off", "true", "false", ".inf", "+.inf", "-.inf", ".nan"):
            self.fail(line, "quote this scalar; only true/false booleans are supported")
        if re.fullmatch(r"[-+]?(?:0|[1-9][0-9]*)", text):
            return int(text)
        if re.fullmatch(r"[-+]?(?:[0-9]+\.[0-9]*|\.[0-9]+|[0-9]+)(?:[eE][-+]?[0-9]+)?", text):
            value = float(text)
            if not math.isfinite(value):
                self.fail(line, "non-finite numbers are unsupported")
            return value
        if re.match(r"[-+]?[0-9]", text) and re.fullmatch(r"[-+]?[0-9_:xXoObBa-fA-F.]+", text):
            self.fail(line, "unsupported numeric syntax; use a decimal number or quote it")
        return text

    def block_entry(self, text: str, line: int, indent: int, depth: int) -> Tuple[str, Any]:
        key, value = self.entry(text, line)
        if value:
            return key, self.value(value, line)
        if self.index == len(self.lines):
            self.fail(line, f"{key} needs a value; use {{}} or [] for an empty collection")
        _, child_indent, child_text = self.lines[self.index]
        indentless_list = child_indent == indent and child_text.startswith("- ")
        if child_indent <= indent and not indentless_list:
            self.fail(line, f"{key} needs a value; use {{}} or [] for an empty collection")
        return key, self.block(child_indent, depth + 1)

    def block(self, indent: int, depth: int = 0) -> Any:
        if depth > PLAYBOOK_MAX_DEPTH:
            self.fail(self.lines[self.index][0], "YAML nesting is too deep")
        line, actual_indent, text = self.lines[self.index]
        is_list = text == "-" or text.startswith("- ")
        result: Any = [] if is_list else _PlaybookMap(line)
        while self.index < len(self.lines):
            line, actual_indent, text = self.lines[self.index]
            if actual_indent < indent:
                break
            if actual_indent != indent:
                self.fail(line, "inconsistent indentation")
            item_is_list = text == "-" or text.startswith("- ")
            if is_list and not item_is_list:
                break
            if not is_list and item_is_list:
                self.fail(line, "cannot mix a block list and mapping")
            self.index += 1
            if not is_list:
                key, value = self.block_entry(text, line, indent, depth)
                if key in result:
                    self.fail(line, f"duplicate mapping key {key!r}")
                result[key] = value
                result.key_lines[key] = line
                continue
            item = text[1:].strip()
            if not item:
                self.fail(line, "block list items need a scalar or mapping")
            mapping_item = item[0] not in "[{" and bool(re.search(r":(?:\s|$)", item))
            if item[0] in "\"'":
                _, end = self.quoted(item, 0, line)
                mapping_item = bool(re.match(r":(?:\s|$)", item[end:].lstrip()))
            if not mapping_item:
                result.append(self.value(item, line))
                continue
            mapping_indent = indent + text.index(item)
            entry = _PlaybookMap(line)
            key, value = self.block_entry(item, line, mapping_indent, depth + 1)
            entry[key] = value
            entry.key_lines[key] = line
            if self.index < len(self.lines) and self.lines[self.index][1] > indent:
                continuation = self.block(mapping_indent, depth + 1)
                if not isinstance(continuation, dict):
                    self.fail(self.lines[self.index - 1][0], "expected mapping fields after list item")
                for key, value in continuation.items():
                    if key in entry:
                        self.fail(continuation.key_lines[key], f"duplicate mapping key {key!r}")
                    entry[key] = value
                    entry.key_lines[key] = continuation.key_lines[key]
            result.append(entry)
        return result


def parse_playbooks_yaml(text: str) -> Any:
    reader = _PlaybookYamlReader(text)
    if not reader.lines:
        raise PlanError("line 1: playbooks must not be empty")
    if reader.lines[0][1] != 0:
        reader.fail(reader.lines[0][0], "top-level indentation must be zero")
    if len(reader.lines) == 1 and reader.lines[0][2][0] in "[{":
        return reader.value(reader.lines[0][2], reader.lines[0][0])
    value = reader.block(0)
    if reader.index < len(reader.lines):
        reader.fail(reader.lines[reader.index][0], "unexpected indentation")
    return value


def playbook_structure_errors(data: Any) -> List[str]:
    errors: List[str] = []

    def problem(owner: Any, key: str, label: str, message: str) -> None:
        line = getattr(owner, "key_lines", {}).get(key, getattr(owner, "line", 1))
        errors.append(f"line {line}: {label} {message}")

    def mapping(value: Any, owner: Any, key: str, label: str, allowed: Any = None) -> bool:
        if not isinstance(value, dict):
            problem(owner, key, label, "must be a mapping")
            return False
        if allowed is not None:
            for field in value:
                if field not in allowed:
                    problem(value, field, f"{label}.{field}", "is an unknown field")
        return True

    def strings(value: Any, owner: Any, key: str, label: str) -> bool:
        if not isinstance(value, list) or any(not isinstance(item, str) or not item for item in value):
            problem(owner, key, label, "must be a list of non-empty strings")
            return False
        return True

    def named(value: Any, owner: Any, key: str, label: str) -> None:
        if not isinstance(value, str) or not SAFE_NAME_PATTERN.fullmatch(value) or value.startswith("-"):
            problem(owner, key, label, "must be a safe model or effort name")

    def tiers(value: Any, owner: Any, key: str, label: str) -> None:
        if not mapping(value, owner, key, label, MODEL_TIERS):
            return
        for tier, entries in value.items():
            where = f"{label}.{tier}"
            if not mapping(entries, value, tier, where):
                continue
            for provider, spec in entries.items():
                entry_label = f"{where}.{provider}"
                if provider not in providers:
                    problem(entries, provider, entry_label, "is not in providers")
                if mapping(spec, entries, provider, entry_label, ("model", "effort")):
                    for field, entry in spec.items():
                        named(entry, spec, field, f"{entry_label}.{field}")

    def quota(value: Any, owner: Any, key: str, label: str) -> None:
        if not mapping(value, owner, key, label, PLAYBOOK_QUOTA_FIELDS):
            return
        enums = {
            "whenLow": ("switch-provider", "keep"), "onBothCritical": ("ask", "run-cheapest", "block"),
            "whenSpare": ("spend-strong", "prefer", "ignore"),
        }
        for field, choices in enums.items():
            if field in value and value[field] not in choices:
                problem(value, field, f"{label}.{field}", f"must be one of {', '.join(choices)}")
        for field, upper in (("criticalThreshold", 100), ("stopAt", 100), ("spareWithinHours", 168), ("spareMinLeft", 100)):
            if field not in value:
                continue
            number = value[field]
            if type(number) is not int or not 1 <= number <= upper:
                problem(value, field, f"{label}.{field}", f"must be an integer between 1 and {upper}")
        if "planEnds" in value:
            mapping(value["planEnds"], value, "planEnds", f"{label}.planEnds")

    if not mapping(data, data, "", "playbooks", PLAYBOOK_TOP_FIELDS):
        return errors
    if type(data.get("version")) is not int or data["version"] != PLAYBOOKS_VERSION:
        problem(data, "version", "version", f"must be {PLAYBOOKS_VERSION}")
    providers = data.get("providers")
    if not strings(providers, data, "providers", "providers"):
        providers = []
    elif not providers or len(set(providers)) != len(providers):
        problem(data, "providers", "providers", "must list each provider once and cannot be empty")
    for provider in providers:
        if not re.fullmatch(r"[a-z][a-z0-9-]{0,31}", provider):
            problem(data, "providers", "providers", f"contains invalid provider id {provider!r}")
    tiers(data.get("tiers"), data, "tiers", "tiers")
    if "passive" in data and mapping(data["passive"], data, "passive", "passive", PASSIVE_DEFAULTS):
        passive = data["passive"]
        for field in ("checkEveryMinutes", "spendStepUsd"):
            if field in passive:
                value = passive[field]
                if type(value) not in (int, float) or not math.isfinite(value) or value <= 0:
                    problem(passive, field, f"passive.{field}", "must be a positive finite number")
        if "askAboveWorkers" in passive and (type(passive["askAboveWorkers"]) is not int or passive["askAboveWorkers"] < 1):
            problem(passive, "askAboveWorkers", "passive.askAboveWorkers", "must be a positive integer")
        if "prefer" in passive and strings(passive["prefer"], passive, "prefer", "passive.prefer"):
            prefer = passive["prefer"]
            if not prefer or len(set(prefer)) != len(prefer) or any(provider not in providers for provider in prefer):
                problem(passive, "prefer", "passive.prefer", "must list distinct configured providers and cannot be empty")
    for field in ("principles", "sources"):
        if field in data:
            strings(data[field], data, field, field)
    for field in ("setup", "models", "adapters", "routes", "routing"):
        if field in data:
            mapping(data[field], data, field, field)
    if "quota" in data:
        quota(data["quota"], data, "quota", "quota")
    if "budget" in data and mapping(data["budget"], data, "budget", "budget", ("warnAt", "whenLow")):
        budget = data["budget"]
        if "warnAt" in budget and (type(budget["warnAt"]) is not int or not 1 <= budget["warnAt"] <= 100):
            problem(budget, "warnAt", "budget.warnAt", "must be an integer between 1 and 100")
        if "whenLow" in budget and budget["whenLow"] not in ("finish-and-report", "downgrade-tier", "stop"):
            problem(budget, "whenLow", "budget.whenLow", "must be finish-and-report, downgrade-tier or stop")
    profiles = data.get("profiles", {})
    if mapping(profiles, data, "profiles", "profiles"):
        active = data.get("profile")
        if not isinstance(active, str) or active not in profiles:
            problem(data, "profile", "profile", "must name an existing profile")
        for profile, value in profiles.items():
            label = f"profiles.{profile}"
            if not mapping(value, profiles, profile, label, PLAYBOOK_PROFILE_FIELDS):
                continue
            if "tiers" in value:
                tiers(value["tiers"], value, "tiers", f"{label}.tiers")
            if "quota" in value:
                quota(value["quota"], value, "quota", f"{label}.quota")
            if "order" in value and strings(value["order"], value, "order", f"{label}.order"):
                order = value["order"]
                if len(set(order)) != len(order) or any(provider not in providers for provider in order):
                    problem(value, "order", f"{label}.order", "must contain unique ids from providers")
    books = data.get("playbooks")
    if mapping(books, data, "playbooks", "playbooks"):
        for book_id, book in books.items():
            label = f"playbooks.{book_id}"
            if not mapping(book, books, book_id, label, PLAYBOOK_FIELDS):
                continue
            for field in PLAYBOOK_FIELDS - {"phases"}:
                if field in book and (not isinstance(book[field], str) or not book[field]):
                    problem(book, field, f"{label}.{field}", "must be a non-empty string")
            phases = book.get("phases")
            if not isinstance(phases, list):
                problem(book, "phases", f"{label}.phases", "must be a list")
                continue
            seen = set()
            for phase in phases:
                where = f"{label}.phases"
                if not mapping(phase, book, "phases", where, PLAYBOOK_PHASE_FIELDS):
                    continue
                phase_id = phase.get("id")
                if not isinstance(phase_id, str) or not ID_PATTERN.fullmatch(phase_id):
                    problem(phase, "id", f"{where}.id", "must be a safe phase id")
                elif phase_id in seen:
                    problem(phase, "id", f"{where}.id", f"duplicates {phase_id}")
                else:
                    seen.add(phase_id)
                for field, choices in PLAYBOOK_PHASE_ENUMS.items():
                    if field in phase and phase[field] not in choices:
                        problem(phase, field, f"{where}.{field}", f"must be one of {', '.join(choices)}")
                for field in ("readOnly", "isolate"):
                    if field in phase and not isinstance(phase[field], bool):
                        problem(phase, field, f"{where}.{field}", "must be true or false")
                if "workers" in phase and (type(phase["workers"]) is not int or not 1 <= phase["workers"] <= MAX_WORKERS):
                    problem(phase, "workers", f"{where}.workers", f"must be an integer between 1 and {MAX_WORKERS}")
                if "provider" in phase and phase["provider"] not in [*providers, "auto"]:
                    problem(phase, "provider", f"{where}.provider", "must be auto or an id from providers")
                for field in ("model", "effort"):
                    if field in phase:
                        named(phase[field], phase, field, f"{where}.{field}")
                if "rules" not in phase:
                    continue
                if not isinstance(phase["rules"], list):
                    problem(phase, "rules", f"{where}.rules", "must be a list")
                    continue
                for rule in phase["rules"]:
                    if not mapping(rule, phase, "rules", f"{where}.rules", ("if", "use")):
                        continue
                    use = rule.get("use")
                    if not mapping(use, rule, "use", f"{where}.rules.use", ("tier", "provider", "model", "effort")):
                        continue
                    if "tier" in use and use["tier"] not in MODEL_TIERS:
                        problem(use, "tier", f"{where}.rules.use.tier", f"must be one of {', '.join(MODEL_TIERS)}")
                    for field in ("model", "effort"):
                        if field in use:
                            named(use[field], use, field, f"{where}.rules.use.{field}")
    picks = data.get("pickOrder", [])
    if not isinstance(picks, list):
        problem(data, "pickOrder", "pickOrder", "must be a list")
        return errors
    for pick in picks:
        if not mapping(pick, data, "pickOrder", "pickOrder", ("when", "playbook")):
            continue
        target = pick.get("playbook")
        if not isinstance(target, str) or not isinstance(books, dict) or target not in books:
            problem(pick, "playbook", "pickOrder.playbook", "must name an existing playbook")
        if not isinstance(pick.get("when"), str) or not pick["when"]:
            problem(pick, "when", "pickOrder.when", "must be a non-empty string")
    return errors


@functools.lru_cache(maxsize=PLAYBOOK_CACHE_SIZE)
def _cached_playbooks(path: str, modified: int, size: int) -> Tuple[str, Dict[str, Any]]:
    text = read_playbooks(Path(path))
    try:
        data = parse_playbooks_yaml(text)
        errors = playbook_structure_errors(data)
    except PlanError as error:
        raise PlanError(f"invalid playbooks at {path}: {error}") from error
    if errors:
        raise PlanError(f"invalid playbooks at {path}: " + "; ".join(errors))
    return text, data


def loaded_playbooks(policy_path: Path) -> Tuple[str, Dict[str, Any]]:
    path = effective_playbooks_path(policy_path).resolve()
    try:
        status = path.stat()
    except OSError as error:
        raise PlanError(f"cannot read playbooks at {path}: {error}") from error
    return _cached_playbooks(str(path), status.st_mtime_ns, status.st_size)


def playbook_settings(policy_path: Optional[Path] = None) -> Dict[str, Any]:
    _, parsed = loaded_playbooks(policy_path or default_policy_path())
    data = copy.deepcopy(parsed)
    profile = data["profiles"][data["profile"]]
    data["tiers"] = {
        tier: {
            provider: {**data["tiers"].get(tier, {}).get(provider, {}), **profile.get("tiers", {}).get(tier, {}).get(provider, {})}
            for provider in data["providers"]
            if provider in data["tiers"].get(tier, {}) or provider in profile.get("tiers", {}).get(tier, {})
        }
        for tier in MODEL_TIERS
    }
    order = profile.get("order", data["providers"])
    data["providers"] = [*order, *(provider for provider in data["providers"] if provider not in order)]
    data["quota"] = {**data.get("quota", {}), **profile.get("quota", {})}
    data["passive"] = {**copy.deepcopy(PASSIVE_DEFAULTS), **data.get("passive", {})}
    return data


def read_playbooks(path: Path) -> str:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as error:
        raise PlanError(f"cannot read playbooks at {path}: {error}") from error
    return text


def playbook_doctor(policy_path: Path) -> Dict[str, Any]:
    path = effective_playbooks_path(policy_path)
    try:
        loaded_playbooks(policy_path)
    except PlanError as error:
        return {"path": str(path), "valid": False, "error": str(error)}
    return {"path": str(path), "valid": True, "error": None}


def selected_playbook_block(text: str, parsed: Dict[str, Any], book_id: str) -> str:
    books = parsed["playbooks"]
    lines = text.splitlines()
    start = books.key_lines[book_id] - 1
    first = lines[start]
    if not first.strip().startswith(book_id + ":"):
        return f"{json.dumps(book_id)}: {json.dumps(books[book_id], ensure_ascii=False)}"
    indent = len(first) - len(first.lstrip())
    end = start + 1
    while end < len(lines):
        line = lines[end]
        if line.strip() and not line.lstrip().startswith("#") and len(line) - len(line.lstrip()) <= indent:
            break
        end += 1
    return "\n".join(lines[start:end]).rstrip()


def playbook_output(policy_path: Path, book_id: Optional[str], pick: bool, as_json: bool) -> str:
    text, parsed = loaded_playbooks(policy_path)
    settings = playbook_settings(policy_path)
    if book_id and pick:
        raise PlanError("pass a playbook id or --pick, not both")
    if book_id and book_id not in parsed["playbooks"]:
        raise PlanError(f"unknown playbook {book_id}; choose from {', '.join(parsed['playbooks'])}")
    if pick:
        payload = {
            "profile": settings["profile"], "providers": settings["providers"],
            "pickOrder": settings.get("pickOrder", []),
            "playbooks": [
                {"id": key, "name": book.get("name", key), "pickWhen": book.get("pickWhen", "")}
                for key, book in settings["playbooks"].items()
            ],
        }
        if as_json:
            return json.dumps(payload, indent=2, ensure_ascii=False)
        lines = [f"profile: {payload['profile']}", f"providers: {', '.join(payload['providers'])}"]
        lines.extend(f"pick {index}: {item['when']} -> {item['playbook']}" for index, item in enumerate(payload["pickOrder"], 1))
        lines.extend(f"{item['id']} | {item['name']} | {item['pickWhen']}" for item in payload["playbooks"])
        return "\n".join(lines)
    if not book_id:
        if as_json:
            return json.dumps(parsed, indent=2, ensure_ascii=False)
        return f"# source: {effective_playbooks_path(policy_path)}\n{text.rstrip()}"
    payload = {
        "id": book_id, "playbook": parsed["playbooks"][book_id], "profile": settings["profile"],
        "tiers": settings["tiers"], "providers": settings["providers"],
        "quota": settings.get("quota", {}), "budget": settings.get("budget", {}),
    }
    if as_json:
        return json.dumps(payload, indent=2, ensure_ascii=False)
    lines = [selected_playbook_block(text, parsed, book_id), "", f"profile: {settings['profile']}", "resolved tiers:"]
    for provider in settings["providers"]:
        for tier in MODEL_TIERS:
            spec = settings["tiers"].get(tier, {}).get(provider, {})
            model = spec.get("model", "<CLI default>")
            effort = spec.get("effort", "<CLI default>")
            lines.append(f"  {provider} x {tier} -> model={model} effort={effort}")
    lines.extend([
        f"providers: {', '.join(settings['providers'])}",
        f"quota: {json.dumps(payload['quota'], ensure_ascii=False)}",
        f"budget: {json.dumps(payload['budget'], ensure_ascii=False)}",
    ])
    return "\n".join(lines)


def init_playbooks(policy_path: Path, force: bool) -> Path:
    target = user_playbooks_path(policy_path)
    if target.exists() and not force:
        raise PlanError(f"{target} already exists; pass --force to replace it with the bundled playbooks")
    text = read_playbooks(bundled_playbooks_path())
    target.parent.mkdir(parents=True, exist_ok=True)
    temporary = target.with_name(f".{target.name}.{uuid.uuid4().hex}.tmp")
    temporary.write_text(text, encoding="utf-8")
    os.replace(temporary, target)
    return target




def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(prog="playbook.py", description=__doc__.splitlines()[0])
    parser.add_argument("--version", action="version", version=f"coding-opus {skill_version() or 'unknown'}")
    parser.add_argument("id", nargs="?", help="print one playbook with its resolved tiers")
    parser.add_argument("--pick", action="store_true", help="print the compact selection index")
    parser.add_argument("--json", action="store_true", help="print structured data")
    parser.add_argument("--path", action="store_true", help="print only the effective file path")
    parser.add_argument("--check", action="store_true", help="validate the effective playbooks file")
    parser.add_argument("--init", action="store_true", help="copy the bundled playbooks for editing")
    parser.add_argument("--force", action="store_true", help="with --init, replace an existing copy")
    parser.add_argument("--policy", type=Path, help=argparse.SUPPRESS)
    args = parser.parse_args(argv)
    policy_path = args.policy or default_policy_path()
    try:
        if args.path:
            print(effective_playbooks_path(policy_path))
        elif args.check:
            result = playbook_doctor(policy_path)
            print(json.dumps(result, indent=2, ensure_ascii=False))
            return 0 if result["valid"] else 1
        elif args.init:
            print(init_playbooks(policy_path, args.force))
        else:
            print(playbook_output(policy_path, args.id, args.pick, args.json))
    except PlanError as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
