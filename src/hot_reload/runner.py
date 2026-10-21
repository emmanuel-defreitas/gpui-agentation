#!/usr/bin/env python3
"""Build and restart a debug Cargo app, controlled by Agentation's settings.

Adapted from exegia/gpui-corpora scripts/dev.py at a240711296727d8379dfb9477d3cff99f162bab5.
Copyright (c) 2026 hmziqrs. MIT license: see LICENSE beside this file.
"""
import argparse
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time

WATCH_DIRS = ("src", "crates", "examples", "demo", "locales", "assets")
WATCH_FILES = ("Cargo.toml", "Cargo.lock", "build.rs", "rust-toolchain.toml")
IGNORED_DIRS = {"target", ".git", "node_modules", "__pycache__"}
CONTROL_ENV = "GPUI_AGENTATION_HOT_RELOAD"


def snapshot(root, ignored_paths=()):
    paths = [root / name for name in WATCH_FILES]
    for name in WATCH_DIRS:
        if (root / name).is_symlink() or (root / name).resolve() in ignored_paths:
            continue
        for directory, dirs, files in os.walk(root / name, followlinks=False):
            dirs[:] = [d for d in dirs if not d.startswith(".") and d not in IGNORED_DIRS
                       and not (Path(directory) / d).is_symlink()
                       and (Path(directory) / d).resolve() not in ignored_paths]
            paths.extend(Path(directory) / name for name in files
                         if not name.startswith(".") and not name.endswith(("~", ".swp", ".tmp")))
    result = {}
    for path in paths:
        try:
            if path.is_symlink():
                continue
            stat = path.stat()
            result[str(path.relative_to(root))] = (stat.st_mtime_ns, stat.st_size)
        except FileNotFoundError:
            pass
    return result


def stop(process):
    if process is None or process.poll() is not None:
        return
    try:
        if os.name == "posix":
            os.killpg(process.pid, signal.SIGTERM)
        else:
            process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            if os.name == "posix":
                os.killpg(process.pid, signal.SIGKILL)
            else:
                process.kill()
            process.wait()
    except ProcessLookupError:
        pass


def write_state(directory, name, value):
    temporary = directory / (name + ".tmp")
    temporary.write_text(value, encoding="utf-8")
    temporary.replace(directory / name)


def is_enabled(directory):
    return (directory / "enabled").read_text(encoding="utf-8").strip() == "1"


def supervise(options, app_args):
    manifest = options.manifest_path.resolve()
    metadata = subprocess.run([os.environ.get("CARGO", "cargo"), "metadata", "--locked", "--no-deps", "--format-version", "1",
                               "--manifest-path", str(manifest)], check=True, capture_output=True, text=True)
    metadata = json.loads(metadata.stdout)
    root = Path(metadata["workspace_root"])
    ignored_paths = (Path(metadata["target_directory"]).resolve(),)
    command = [os.environ.get("CARGO", "cargo"), "build", "--locked", "--manifest-path", str(manifest),
               "--message-format=json-render-diagnostics"]
    if options.package:
        command += ["--package", options.package]
    command += ["--example" if options.example else "--bin", options.example or options.bin]
    if options.features:
        command += ["--features", options.features]
    if options.no_default_features:
        command += ["--no-default-features"]
    app = builder = None
    with tempfile.TemporaryDirectory(prefix="gpui-agentation-reload-") as temporary:
        control = Path(temporary)
        write_state(control, "enabled", "0")
        write_state(control, "status", "Off")
        environment = dict(os.environ, **{CONTROL_ENV: str(control)})

        def build(initial=False):
            nonlocal builder
            write_state(control, "status", "Building")
            print("[dev] Building…", flush=True)
            with tempfile.TemporaryFile(mode="w+", encoding="utf-8") as output:
                builder = subprocess.Popen(command, cwd=root, stdout=output, start_new_session=True)
                while builder.poll() is None:
                    if not initial and not is_enabled(control):
                        stop(builder)
                        write_state(control, "status", "Off")
                        return None
                    time.sleep(0.1)
                output.seek(0)
                executable = None
                for line in output:
                    try:
                        event = json.loads(line)
                    except json.JSONDecodeError:
                        continue
                    if event.get("reason") == "compiler-message":
                        print(event["message"].get("rendered", ""), file=sys.stderr, end="")
                    if (event.get("reason") == "compiler-artifact"
                            and event.get("target", {}).get("name") == (options.example or options.bin)
                            and event.get("executable")):
                        executable = event["executable"]
                if builder.returncode or executable is None:
                    write_state(control, "status", "Build failed — save a fix to retry")
                    print("[dev] Build failed; keeping the current app.", flush=True)
                    return None
                return executable

        try:
            executable = build(initial=True)
            if executable is None:
                return 1
            app = subprocess.Popen([executable, *app_args], cwd=root, env=environment, start_new_session=True)
            write_state(control, "status", "Off")
            print("[dev] Open Settings → Hot reload to watch saves. Ctrl+C stops the app.", flush=True)
            previous = snapshot(root, ignored_paths)
            changed_at = None
            was_enabled = False
            while app.poll() is None:
                time.sleep(0.25)
                enabled = is_enabled(control)
                current = snapshot(root, ignored_paths) if enabled else previous
                if enabled != was_enabled:
                    previous = snapshot(root, ignored_paths)
                    changed_at = None
                    was_enabled = enabled
                    write_state(control, "status", "Watching" if enabled else "Off")
                elif enabled:
                    if current != previous:
                        previous = current
                        changed_at = time.monotonic()
                    elif changed_at is not None and time.monotonic() - changed_at >= 0.4:
                        changed_at = None
                        executable = build()
                        if executable and is_enabled(control) and app.poll() is None:
                            stop(app)
                            app = subprocess.Popen([executable, *app_args], cwd=root, env=environment,
                                                   start_new_session=True)
                            write_state(control, "status", "Watching")
                            print(f"[dev] Restarted app (PID {app.pid}).", flush=True)
            return app.returncode
        except KeyboardInterrupt:
            print("\n[dev] Stopping…", flush=True)
            return 0
        finally:
            stop(builder)
            stop(app)


def main(argv=None):
    argv = list(sys.argv[1:] if argv is None else argv)
    split = argv.index("--") if "--" in argv else len(argv)
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--manifest-path", type=Path, default=Path("Cargo.toml"))
    target = parser.add_mutually_exclusive_group(required=True)
    target.add_argument("--bin")
    target.add_argument("--example")
    parser.add_argument("--package")
    parser.add_argument("--features")
    parser.add_argument("--no-default-features", action="store_true")
    options = parser.parse_args(argv[:split])
    def interrupted(_signal, _frame):
        raise KeyboardInterrupt
    signal.signal(signal.SIGTERM, interrupted)
    return supervise(options, argv[split + 1:])


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"[dev] {error}", file=sys.stderr)
        sys.exit(1)
