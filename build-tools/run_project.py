#!/usr/bin/env python3
"""Compile explicit authored C# inputs and run the existing headless scene player.

Implements the independently reviewed INFRA-PROJECT-COMPILATION-001 DESIGN.
This is build infrastructure, not Unity project/compiler compatibility.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import signal
import subprocess
import sys
import tempfile
import threading
import time
import xml.etree.ElementTree as ET
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SDK_VERSION = "8.0.425"
MIB = 1024 * 1024
LOG_LIMIT = 4 * MIB
MANIFEST_KEYS = {"schema_version", "sources", "scene", "bindings", "ticks"}


class ProjectFailure(Exception):
    """A reported infrastructure failure with an explicit stage and exit class."""

    def __init__(self, stage: str, message: str, *, blocked: bool = False):
        super().__init__(message)
        self.stage = stage
        self.blocked = blocked


def file_identity(path: Path, role: str) -> dict:
    digest = hashlib.sha256()
    size = 0
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(64 * 1024), b""):
            digest.update(chunk)
            size += len(chunk)
    return {"role": role, "path": str(path), "size": size, "sha256": digest.hexdigest()}


def read_bounded(path: Path, limit: int, stage: str) -> bytes:
    with path.open("rb") as stream:
        data = stream.read(limit + 1)
    if len(data) > limit:
        raise ProjectFailure(stage, f"file exceeds {limit} bytes: {path}")
    return data


def project_file(project: Path, relative: object) -> Path:
    if not isinstance(relative, str) or not relative or "\0" in relative or "\\" in relative:
        raise ProjectFailure("inputs", "project paths must be nonempty relative forward-slash strings")
    parts = relative.split("/")
    if relative.startswith("/") or any(part in {"", ".", ".."} for part in parts):
        raise ProjectFailure("inputs", f"invalid relative project path: {relative}")
    candidate = (project / relative).resolve(strict=True)
    if not candidate.is_relative_to(project) or not candidate.is_file():
        raise ProjectFailure("inputs", f"project path escapes the root or is not a regular file: {relative}")
    return candidate


def unique_object(pairs: list[tuple[str, object]]) -> dict:
    result = {}
    for key, value in pairs:
        if key in result:
            raise ProjectFailure("manifest", f"duplicate JSON key: {key}")
        result[key] = value
    return result


def reject_constant(value: str) -> None:
    raise ProjectFailure("manifest", f"non-finite JSON number: {value}")


def load_manifest(project: Path, relative: str) -> tuple[dict, list[dict], list[bytes]]:
    path = project_file(project, relative)
    raw = read_bounded(path, MIB, "manifest")
    manifest = json.loads(raw.decode("utf-8"), object_pairs_hook=unique_object, parse_constant=reject_constant)
    if not isinstance(manifest, dict) or set(manifest) != MANIFEST_KEYS:
        raise ProjectFailure(
            "manifest",
            "manifest keys must be exactly schema_version, sources, scene, bindings, ticks",
        )
    if type(manifest["schema_version"]) is not int or manifest["schema_version"] != 1:
        raise ProjectFailure("manifest", "unsupported schema_version; expected integer 1")
    sources = manifest["sources"]
    ticks = manifest["ticks"]
    if not isinstance(sources, list) or not 1 <= len(sources) <= 256:
        raise ProjectFailure("manifest", "sources must contain 1..256 explicit C# paths")
    if not isinstance(ticks, list) or not 1 <= len(ticks) <= 256:
        raise ProjectFailure("manifest", "ticks must contain 1..256 signed 64-bit integers")
    if any(type(tick) is not int or not -(2**63) <= tick < 2**63 for tick in ticks):
        raise ProjectFailure("manifest", "ticks must be signed 64-bit integers")

    inputs = [
        {
            "role": "manifest",
            "path": str(path),
            "relative_path": relative,
            "size": len(raw),
            "sha256": hashlib.sha256(raw).hexdigest(),
        }
    ]
    source_bytes = []
    seen = set()
    total = 0
    for index, source in enumerate(sources):
        source_path = project_file(project, source)
        if source_path.suffix != ".cs" or source_path in seen:
            raise ProjectFailure("inputs", f"source must be a distinct resolved .cs file: {source}")
        seen.add(source_path)
        data = read_bounded(source_path, MIB, "inputs")
        data.decode("utf-8")
        if b"\0" in data:
            raise ProjectFailure("inputs", f"source contains NUL: {source}")
        total += len(data)
        if total > 16 * MIB:
            raise ProjectFailure("inputs", "aggregate C# sources exceed 16 MiB")
        source_bytes.append(data)
        inputs.append(
            {
                "role": "source",
                "path": str(source_path),
                "relative_path": source,
                "snapshot": f"sources/{index:04d}.cs",
                "size": len(data),
                "sha256": hashlib.sha256(data).hexdigest(),
            }
        )
    for role in ("scene", "bindings"):
        selected = project_file(project, manifest[role])
        read_bounded(selected, 16 * MIB, "inputs")
        identity = file_identity(selected, role)
        identity["relative_path"] = manifest[role]
        inputs.append(identity)
    return manifest, inputs, source_bytes


def trusted_file(value: str, role: str, *, executable: bool = False) -> Path:
    supplied = Path(value)
    if not supplied.is_absolute():
        raise ProjectFailure("toolchain", f"{role} must be an absolute path", blocked=True)
    try:
        path = supplied.resolve(strict=True)
    except OSError as error:
        raise ProjectFailure("toolchain", f"{role} is unavailable: {error}", blocked=True) from error
    if not path.is_file() or (executable and not os.access(path, os.X_OK)):
        raise ProjectFailure("toolchain", f"{role} is not a regular usable file: {path}", blocked=True)
    return path


def select_hostfxr(sdk_root: Path) -> Path:
    names = {"darwin": "libhostfxr.dylib", "linux": "libhostfxr.so"}
    if sys.platform not in names or os.name != "posix":
        raise ProjectFailure("toolchain", "only POSIX macOS and Linux are supported", blocked=True)
    if not (sdk_root / "sdk" / SDK_VERSION).is_dir():
        raise ProjectFailure("toolchain", f"selected SDK root lacks sdk/{SDK_VERSION}", blocked=True)
    versions = []
    fxr_root = sdk_root / "host" / "fxr"
    if fxr_root.is_dir():
        for directory in fxr_root.iterdir():
            if re.fullmatch(r"8\.\d+\.\d+", directory.name):
                library = directory / names[sys.platform]
                if library.is_file():
                    version = tuple(int(part) for part in directory.name.split("."))
                    versions.append((version, library.resolve(strict=True)))
    if not versions:
        raise ProjectFailure("toolchain", "selected SDK root has no stable 8.x hostfxr", blocked=True)
    return max(versions)[1]


def create_output(value: str, project: Path) -> Path:
    supplied = Path(value).absolute()
    parent = supplied.parent.resolve(strict=True)
    output = parent / supplied.name
    if output.exists() or output.is_symlink():
        raise ProjectFailure("output", f"output already exists: {output}")
    if output.is_relative_to(project) or project.is_relative_to(output):
        raise ProjectFailure("output", "output and project must not contain each other")
    output.mkdir(mode=0o700)
    return output


def require_controlled_path(path: Path) -> None:
    current = ROOT
    for part in path.relative_to(ROOT).parts:
        current = current / part
        if current.is_symlink():
            raise ProjectFailure("toolchain", f"controlled managed input contains a symlink: {current}")


def snapshot_toolchain(output: Path) -> list[dict]:
    interop = ROOT / "managed" / "Interop" / "OpenUnityNative.cs"
    require_controlled_path(interop)
    files = [interop]
    for directory in ("UnityEngine", "Scripting", "Hosting"):
        source_root = ROOT / "managed" / directory
        require_controlled_path(source_root)
        if not source_root.is_dir():
            raise ProjectFailure("toolchain", f"controlled managed directory is missing: {source_root}")
        for parent, directories, filenames in os.walk(source_root, followlinks=False):
            directories[:] = [name for name in directories if name not in {"bin", "obj"}]
            for name in directories:
                require_controlled_path(Path(parent) / name)
            for name in filenames:
                path = Path(parent) / name
                if path.suffix in {".cs", ".csproj"}:
                    require_controlled_path(path)
                    files.append(path)
    identities = []
    for source in sorted(files):
        relative = source.relative_to(ROOT)
        if source.is_symlink() or not source.is_file():
            raise ProjectFailure("toolchain", f"controlled managed input is not a regular non-symlink file: {source}")
        destination = output / "toolchain" / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        data = source.read_bytes()
        destination.write_bytes(data)
        identities.append(
            {
                "role": "managed-source",
                "path": str(source),
                "snapshot": str(destination),
                "size": len(data),
                "sha256": hashlib.sha256(data).hexdigest(),
            }
        )
    return identities


def write_project(output: Path, inputs: list[dict], source_bytes: list[bytes]) -> None:
    (output / "sources").mkdir()
    sources = [item for item in inputs if item["role"] == "source"]
    for source, data in zip(sources, source_bytes, strict=True):
        (output / source["snapshot"]).write_bytes(data)

    project = ET.Element("Project", Sdk="Microsoft.NET.Sdk")
    properties = ET.SubElement(project, "PropertyGroup")
    values = {
        "TargetFramework": "net8.0",
        "AssemblyName": "Assembly-CSharp",
        "OutputType": "Library",
        "AllowUnsafeBlocks": "true",
        "Nullable": "disable",
        "ImplicitUsings": "disable",
        "EnableDefaultCompileItems": "false",
    }
    for name, value in values.items():
        ET.SubElement(properties, name).text = value
    items = ET.SubElement(project, "ItemGroup")
    for source in sources:
        ET.SubElement(items, "Compile", Include=source["snapshot"])
    references = (
        "toolchain/managed/UnityEngine/UnityEngine.CoreModule.csproj",
        "toolchain/managed/Hosting/OpenUnity.ManagedHosting.csproj",
    )
    for reference in references:
        ET.SubElement(items, "ProjectReference", Include=reference)
    ET.indent(project)
    ET.ElementTree(project).write(output / "Assembly-CSharp.csproj", encoding="utf-8", xml_declaration=True)
    sdk = {"sdk": {"version": SDK_VERSION, "rollForward": "disable", "allowPrerelease": False}}
    (output / "global.json").write_text(json.dumps(sdk, indent=2) + "\n", encoding="utf-8")
    (output / "NuGet.Config").write_text(
        '<?xml version="1.0" encoding="utf-8"?>\n'
        "<configuration><packageSources><clear /></packageSources></configuration>\n",
        encoding="utf-8",
    )


def child_environment(output: Path, sdk_root: Path) -> dict[str, str]:
    environment = {}
    for name in ("HOME", "LANG", "LC_ALL", "LC_CTYPE", "TZ", "SYSTEMROOT", "OPENUNITY_TRACE"):
        if name in os.environ:
            environment[name] = os.environ[name]
    for name in ("cli-home", "packages", "tmp"):
        (output / name).mkdir()
    environment.update(
        {
            "PATH": os.pathsep.join((str(sdk_root), "/usr/bin", "/bin")),
            "DOTNET_ROOT": str(sdk_root),
            "DOTNET_CLI_HOME": str(output / "cli-home"),
            "DOTNET_CLI_TELEMETRY_OPTOUT": "1",
            "DOTNET_SKIP_FIRST_TIME_EXPERIENCE": "1",
            "DOTNET_NOLOGO": "1",
            "DOTNET_MULTILEVEL_LOOKUP": "0",
            "NUGET_PACKAGES": str(output / "packages"),
            "TMPDIR": str(output / "tmp"),
        }
    )
    return environment


def failure_text(error: BaseException) -> str:
    return f"{type(error).__name__}: {error}"[:2048]


def append_cleanup_error(errors: list[dict], operation: str, error: BaseException) -> None:
    errors.append({"operation": operation, "error": failure_text(error)})


def signal_group(process: subprocess.Popen, selected_signal: int, errors: list[dict]) -> None:
    try:
        os.killpg(process.pid, selected_signal)
    except ProcessLookupError:
        pass
    except OSError as error:
        append_cleanup_error(errors, f"process-group signal {selected_signal}", error)


def terminate_group(process: subprocess.Popen) -> list[dict]:
    errors = []
    signal_group(process, signal.SIGTERM, errors)
    try:
        process.wait(timeout=2)
    except subprocess.TimeoutExpired:
        pass
    except OSError as error:
        append_cleanup_error(errors, "wait after SIGTERM", error)
    signal_group(process, signal.SIGKILL, errors)
    try:
        process.wait(timeout=2)
    except (OSError, subprocess.TimeoutExpired) as error:
        append_cleanup_error(errors, "reap after SIGKILL", error)
    return errors


class ChildLog:
    """Own one bounded diagnostic stream and its optional drain thread."""

    def __init__(self, path: Path, overflow: threading.Event, failed: threading.Event, limit: int):
        self.path = path
        self.overflow = overflow
        self.failed = failed
        self.limit = limit
        self.destination = path.open("xb")
        self.pipe = None
        self.reader = None
        self.errors = []

    def start(self, pipe) -> None:
        self.pipe = pipe
        self.reader = threading.Thread(target=self.drain, daemon=True)
        self.reader.start()

    def is_alive(self) -> bool:
        return self.reader is not None and self.reader.is_alive()

    def drain(self) -> None:
        written = 0
        marker = b"\n[OpenUnity: diagnostic byte limit exceeded]\n"
        try:
            for data in iter(lambda: self.pipe.read(64 * 1024), b""):
                remaining = max(0, self.limit - written)
                retained = data[:remaining]
                self.write(retained)
                written += len(retained)
                if len(data) > remaining:
                    self.overflow.set()
                    if marker:
                        self.write(marker)
                        marker = b""
            self.destination.flush()
        except (OSError, ValueError) as error:
            self.errors.append({"stream": str(self.path), "error": failure_text(error)})
            self.failed.set()

    def write(self, data: bytes) -> None:
        if self.destination.write(data) != len(data):
            raise OSError("incomplete child diagnostic write")

    def cleanup(self) -> list[dict]:
        errors = []
        if self.reader is not None and self.reader.ident is not None:
            try:
                self.reader.join(timeout=2)
            except RuntimeError as error:
                append_cleanup_error(errors, f"join reader {self.path.name}", error)
        if self.is_alive():
            errors.append({"operation": f"close {self.path.name}", "error": "child pipe reader remained active"})
            return errors
        if self.pipe is not None and not self.pipe.closed:
            try:
                self.pipe.close()
            except OSError as error:
                append_cleanup_error(errors, f"close pipe {self.path.name}", error)
        try:
            self.destination.close()
        except OSError as error:
            append_cleanup_error(errors, f"close log {self.path.name}", error)
        return errors


class ChildExecution:
    """Keep launched process ownership and diagnostics through partial failures."""

    def __init__(self, result: dict, environment: dict[str, str], log_limit: int):
        self.result = result
        self.environment = environment
        self.log_limit = log_limit
        self.process = None
        self.logs = []
        self.overflow = threading.Event()
        self.interrupted = threading.Event()
        self.log_failed = threading.Event()
        self.started = time.monotonic()

    def defer_interrupt(self, signum, frame) -> None:
        self.interrupted.set()

    def open_logs(self) -> None:
        for name in ("stdout", "stderr"):
            self.logs.append(ChildLog(Path(self.result[name]), self.overflow, self.log_failed, self.log_limit))

    def launch(self) -> None:
        try:
            self.process = subprocess.Popen(
                self.result["argv"],
                cwd=self.result["cwd"],
                env=self.environment,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                start_new_session=True,
            )
        except (OSError, ValueError) as error:
            self.result["launch_error"] = failure_text(error)
            return
        self.result["launched"] = True
        for output, pipe in zip(self.logs, (self.process.stdout, self.process.stderr), strict=True):
            output.pipe = pipe
        for output in self.logs:
            output.start(output.pipe)

    def wait(self, timeout_seconds: float) -> None:
        if self.process is None:
            return
        while self.process.poll() is None or any(output.is_alive() for output in self.logs):
            if self.interrupted.is_set():
                self.result["interrupted"] = True
                return
            if self.log_failed.is_set():
                return
            if self.overflow.is_set():
                self.result["overflow"] = True
                return
            if time.monotonic() - self.started >= timeout_seconds:
                self.result["timed_out"] = True
                return
            time.sleep(0.02)

    def cleanup_process(self) -> list[dict]:
        if self.process is None:
            return []
        unfinished = self.process.poll() is None or any(output.is_alive() for output in self.logs)
        bounded_failure = self.result["timed_out"] or self.overflow.is_set() or self.log_failed.is_set()
        if unfinished or bounded_failure or self.interrupted.is_set() or self.result.get("log_error"):
            return terminate_group(self.process)
        return []

    def cleanup_owned_resources(self) -> list[dict]:
        errors = []
        try:
            errors.extend(self.cleanup_process())
        except (OSError, RuntimeError, subprocess.SubprocessError) as error:
            append_cleanup_error(errors, "process cleanup", error)
        for output in self.logs:
            try:
                errors.extend(output.cleanup())
            except (OSError, RuntimeError) as error:
                append_cleanup_error(errors, f"log cleanup {output.path.name}", error)
        return errors

    def finish(self) -> None:
        errors = self.cleanup_owned_resources()
        self.result["cleanup_errors"] = errors
        if errors:
            self.result["cleanup_error"] = errors[0]["error"]
        self.result["log_errors"] = [error for output in self.logs for error in output.errors]
        if self.result["log_errors"] and not self.result.get("log_error"):
            self.result["log_error"] = self.result["log_errors"][0]["error"]
        if self.process is not None:
            self.result["exit_code"] = self.process.returncode
        self.result["interrupted"] = self.result["interrupted"] or self.interrupted.is_set()
        self.result["overflow"] = self.result["overflow"] or self.overflow.is_set()
        self.result["duration_seconds"] = time.monotonic() - self.started


def run_child(
    argv: list[str],
    cwd: Path,
    environment: dict[str, str],
    log_directory: Path,
    stage: str,
    timeout_seconds: float,
    log_limit: int = LOG_LIMIT,
) -> dict:
    """Retain bounded real child output and reap the owned process group."""
    stdout = log_directory / f"{stage}.stdout.log"
    stderr = log_directory / f"{stage}.stderr.log"
    result = {
        "stage": stage,
        "argv": argv,
        "cwd": str(cwd),
        "exit_code": None,
        "timed_out": False,
        "overflow": False,
        "stdout": str(stdout),
        "stderr": str(stderr),
        "launched": False,
        "interrupted": False,
    }
    execution = ChildExecution(result, environment, log_limit)
    previous_handler = signal.signal(signal.SIGINT, execution.defer_interrupt)
    try:
        log_directory.mkdir(exist_ok=True)
        execution.open_logs()
        execution.launch()
        execution.wait(timeout_seconds)
    except KeyboardInterrupt:
        result["interrupted"] = True
    except (OSError, RuntimeError, ValueError) as error:
        result["log_error"] = failure_text(error)
    finally:
        execution.finish()
        try:
            signal.signal(signal.SIGINT, previous_handler)
        except (OSError, ValueError) as error:
            append_cleanup_error(result["cleanup_errors"], "restore SIGINT handler", error)
            result.setdefault("cleanup_error", failure_text(error))
    return result


def require_child(result: dict, *, blocked: bool = False) -> None:
    problem = None
    if not result["launched"]:
        problem = result.get("launch_error") or result.get("log_error") or "child did not launch"
    elif result["interrupted"]:
        problem = "child execution interrupted"
    elif result["timed_out"]:
        problem = "child exceeded its time limit"
    elif result["overflow"]:
        problem = "child exceeded its diagnostic byte limit"
    elif result.get("log_error"):
        problem = result["log_error"]
    elif result["exit_code"] != 0:
        problem = f"child exited {result['exit_code']}; see retained diagnostics"
    elif result.get("cleanup_error"):
        problem = result["cleanup_error"]
    if problem is not None:
        bounded_failure = (
            result["interrupted"]
            or result["timed_out"]
            or result["overflow"]
            or "cleanup_error" in result
            or "log_error" in result
        )
        raise ProjectFailure(result["stage"], problem, blocked=blocked and not bounded_failure)


def require_stable_inputs(inputs: list[dict]) -> None:
    for original in inputs:
        try:
            path = Path(original["path"])
            if path.stat().st_size != original["size"]:
                raise ProjectFailure("input-stability", f"selected input changed: {original['path']}")
            data = read_bounded(path, original["size"], "input-stability")
        except OSError as error:
            raise ProjectFailure("input-stability", f"selected input became unavailable: {original['path']}") from error
        if hashlib.sha256(data).hexdigest() != original["sha256"]:
            raise ProjectFailure("input-stability", f"selected input changed: {original['path']}")


@dataclass(frozen=True)
class BuildInputs:
    project: Path
    manifest: dict
    identities: list[dict]
    sources: list[bytes]
    sdk: Path
    player: Path
    native: Path
    hostfxr: Path


@dataclass(frozen=True)
class BuildWorkspace:
    inputs: BuildInputs
    output: Path
    environment: dict[str, str]

    def run(self, report: dict, stage: str, argv: list[str], timeout: float, *, blocked: bool = False) -> dict:
        report["stage"] = stage
        result = run_child(argv, self.output, self.environment, self.output / "logs", stage, timeout)
        report["children"].append(result)
        if stage == "scene":
            report["scene_launch_occurred"] = result["launched"]
        require_child(result, blocked=blocked)
        return result


def load_build_inputs(arguments: argparse.Namespace, report: dict) -> BuildInputs:
    project = Path(arguments.project).resolve(strict=True)
    if not project.is_dir():
        raise ProjectFailure("inputs", "project root must be an existing directory")
    manifest, identities, sources = load_manifest(project, arguments.manifest)
    report["manifest"] = manifest
    report["ticks"] = manifest["ticks"]
    report["inputs"] = identities
    sdk = trusted_file(arguments.dotnet, "dotnet", executable=True)
    player = trusted_file(arguments.player, "player", executable=True)
    native = trusted_file(arguments.native_library, "native-library")
    hostfxr = select_hostfxr(sdk.parent)
    report["toolchain"] = [
        file_identity(sdk, "dotnet"),
        file_identity(hostfxr, "hostfxr"),
        file_identity(player, "player"),
        file_identity(native, "native-library"),
    ]
    return BuildInputs(project, manifest, identities, sources, sdk, player, native, hostfxr)


def prepare_workspace(arguments: argparse.Namespace, inputs: BuildInputs, report: dict) -> BuildWorkspace:
    output = create_output(arguments.output, inputs.project)
    report["output_directory"] = str(output)
    report["stage"] = "snapshot"
    report["toolchain"].extend(snapshot_toolchain(output))
    write_project(output, inputs.identities, inputs.sources)
    for name in ("Assembly-CSharp.csproj", "global.json", "NuGet.Config"):
        report["toolchain"].append(file_identity(output / name, "generated-build-input"))
    environment = child_environment(output, inputs.sdk.parent)
    environment["OPENUNITY_NATIVE_LIBRARY"] = str(inputs.native)
    return BuildWorkspace(inputs, output, environment)


def compile_sources(workspace: BuildWorkspace, report: dict) -> None:
    sdk = str(workspace.inputs.sdk)
    version = workspace.run(report, "sdk", [sdk, "--version"], 30, blocked=True)
    selected_version = Path(version["stdout"]).read_text(encoding="utf-8").strip()
    if selected_version != SDK_VERSION:
        raise ProjectFailure("sdk", f"expected SDK {SDK_VERSION}, got {selected_version[:128]}", blocked=True)
    options = [
        "--disable-build-servers",
        "--verbosity",
        "minimal",
        "-noAutoResponse",
        "-nodeReuse:false",
        "-p:ImportDirectoryBuildProps=false",
        "-p:ImportDirectoryBuildTargets=false",
        "-p:UseSharedCompilation=false",
        "-p:NuGetAudit=false",
    ]
    workspace.run(
        report, "restore", [sdk, "restore", "Assembly-CSharp.csproj", "--configfile", "NuGet.Config", *options], 300
    )
    workspace.run(
        report, "build", [sdk, "build", "Assembly-CSharp.csproj", "--no-restore", "--output", "bin", *options], 300
    )


def collect_artifacts(output: Path, report: dict) -> dict[str, Path]:
    report["stage"] = "artifacts"
    names = {
        "assembly": "Assembly-CSharp.dll",
        "core": "UnityEngine.CoreModule.dll",
        "scripting": "UnityEngine.ScriptingModule.dll",
        "hosting": "OpenUnity.ManagedHosting.dll",
        "runtime-config": "OpenUnity.ManagedHosting.runtimeconfig.json",
    }
    artifacts = {}
    for role, name in names.items():
        path = output / "bin" / name
        if not path.is_file() or path.stat().st_size == 0:
            raise ProjectFailure("artifacts", f"missing fresh build artifact: {name}")
        artifacts[role] = path
        report["artifacts"].append(file_identity(path, role))
    return artifacts


def scene_command(inputs: BuildInputs, artifacts: dict[str, Path]) -> list[str]:
    selected = {item["role"]: item["path"] for item in inputs.identities if item["role"] in {"scene", "bindings"}}
    return [
        str(inputs.player),
        "--run-scene",
        str(inputs.sdk.parent),
        str(inputs.hostfxr),
        str(artifacts["hosting"]),
        str(artifacts["runtime-config"]),
        str(artifacts["assembly"]),
        str(inputs.project),
        selected["scene"],
        selected["bindings"],
        *(str(tick) for tick in inputs.manifest["ticks"]),
    ]


def record_failure(report: dict, error: BaseException, *, stage: str | None = None) -> None:
    selected_stage = error.stage if isinstance(error, ProjectFailure) else stage or report["stage"]
    message = str(error)[:2048] if isinstance(error, ProjectFailure) else failure_text(error)
    if isinstance(error, KeyboardInterrupt):
        message = "execution interrupted"
    if report["error"] is not None:
        report.setdefault("secondary_errors", []).append({"stage": selected_stage, "error": message})
        return
    report["status"] = "blocked" if isinstance(error, ProjectFailure) and error.blocked else "failed"
    report["stage"] = selected_stage
    report["error"] = message


def run_scene_checked(workspace: BuildWorkspace, report: dict, artifacts: dict[str, Path]) -> None:
    require_stable_inputs(workspace.inputs.identities)
    try:
        workspace.run(report, "scene", scene_command(workspace.inputs, artifacts), 120)
    except (ProjectFailure, OSError, ValueError, KeyboardInterrupt) as error:
        record_failure(report, error)
    try:
        require_stable_inputs(workspace.inputs.identities)
    except (ProjectFailure, OSError, ValueError) as error:
        record_failure(report, error, stage="input-stability")


def execute(arguments: argparse.Namespace, report: dict) -> None:
    inputs = load_build_inputs(arguments, report)
    workspace = prepare_workspace(arguments, inputs, report)
    compile_sources(workspace, report)
    artifacts = collect_artifacts(workspace.output, report)
    run_scene_checked(workspace, report, artifacts)
    if report["error"] is None:
        report["stage"] = "complete"
        report["status"] = "passed"


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("project", "manifest", "output", "dotnet", "player", "native-library"):
        parser.add_argument(f"--{name}", required=True)
    return parser.parse_args()


def new_report() -> dict:
    return {
        "schema_version": 1,
        "status": "failed",
        "stage": "inputs",
        "error": None,
        "output_directory": None,
        "manifest": None,
        "ticks": [],
        "inputs": [],
        "toolchain": [],
        "artifacts": [],
        "children": [],
        "scene_launch_occurred": False,
        "unity_behavior_verified": False,
    }


def serialize_report(report: dict) -> str:
    return json.dumps(report, indent=2, ensure_ascii=False) + "\n"


class ReportCommitFailure(ProjectFailure):
    """A failed report transaction whose candidate is retained for recovery."""

    def __init__(self, path: Path, errors: list[dict]):
        super().__init__("report", "could not retain owned result: " + errors[0]["error"])
        self.path = path
        self.errors = errors


def write_report_candidate(output: Path, serialized: str) -> Path:
    descriptor, name = tempfile.mkstemp(prefix=".run-result-", suffix=".json", dir=output)
    path = Path(name)
    errors = []
    try:
        data = memoryview(serialized.encode("utf-8"))
        while data:
            written = os.write(descriptor, data)
            if written <= 0:
                raise OSError("report write made no progress")
            data = data[written:]
        os.fsync(descriptor)
    except (OSError, KeyboardInterrupt) as error:
        append_cleanup_error(errors, "write/fsync report candidate", error)
    try:
        os.close(descriptor)
    except OSError as error:
        append_cleanup_error(errors, "close report candidate", error)
    if errors:
        raise ReportCommitFailure(path, errors)
    return path


def commit_report(report: dict) -> None:
    if report["output_directory"] is None:
        return
    output = Path(report["output_directory"])
    path = write_report_candidate(output, serialize_report(report))
    try:
        os.replace(path, output / "run-result.json")
    except (OSError, KeyboardInterrupt) as error:
        errors = []
        append_cleanup_error(errors, "commit report candidate", error)
        raise ReportCommitFailure(path, errors) from error


def retain_report(report: dict) -> None:
    try:
        commit_report(report)
    except ReportCommitFailure as error:
        record_failure(report, error)
        report["status"] = "failed"
        report["report_recovery_path"] = str(error.path)
        report["report_commit_errors"] = error.errors
    except OSError as error:
        record_failure(report, error, stage="report")
        report["status"] = "failed"


def main() -> int:
    arguments = parse_arguments()
    report = new_report()
    try:
        execute(arguments, report)
    except (ProjectFailure, OSError, ValueError, KeyboardInterrupt) as error:
        record_failure(report, error)
    retain_report(report)
    sys.stdout.write(serialize_report(report))
    return {"passed": 0, "failed": 1, "blocked": 2}[report["status"]]


if __name__ == "__main__":
    raise SystemExit(main())
