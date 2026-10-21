"""Real-process coverage for the adapted development supervisor (stdlib only)."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest

RUNNER = Path(__file__).with_name("runner.py")
spec = importlib.util.spec_from_file_location("runner", RUNNER)
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


def eventually(predicate, timeout=15):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = predicate()
        if result:
            return result
        time.sleep(0.05)
    raise AssertionError("Timed out waiting for supervisor state")


class RunnerTests(unittest.TestCase):
    def test_snapshot_ignores_build_outputs_annotations_and_editor_temporary_files(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "src").mkdir()
            (root / "src/main.rs").write_text("fn main() {}")
            (root / "target/demo").mkdir(parents=True)
            before = runner.snapshot(root)
            (root / "target/demo/feedback.json").write_text("a saved annotation")
            (root / "src/main.rs.swp").write_text("editor save")
            self.assertEqual(before, runner.snapshot(root))
            (root / "src/main.rs").write_text("fn main() { println!(\"change\"); }")
            self.assertNotEqual(before, runner.snapshot(root))
            (root / "src/main.rs").unlink()
            self.assertNotEqual(before, runner.snapshot(root))

    def test_off_failure_recovery_and_shutdown_with_a_real_cargo_app(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "src").mkdir()
            (root / "Cargo.toml").write_text('[package]\nname="reload-probe"\nversion="0.1.0"\nedition="2021"\n')
            (root / "Cargo.lock").write_text('version = 4\n\n[[package]]\nname = "reload-probe"\nversion = "0.1.0"\n')
            source = r'''use std::{env,fs::OpenOptions,io::Write,thread,time::Duration};
fn main() {
 let mut out = OpenOptions::new().create(true).append(true).open(env::var("PROBE_LOG").unwrap()).unwrap();
 writeln!(out, "{}|{}|{}",std::process::id(),env::var("GPUI_AGENTATION_HOT_RELOAD").unwrap(),env::args().nth(1).unwrap()).unwrap();
 loop { thread::sleep(Duration::from_millis(50)); }
}'''
            main = root / "src/main.rs"
            main.write_text(source)
            log = root / "starts"
            environment = dict(os.environ, PROBE_LOG=str(log))
            # A separate target tree avoids locks from the main workspace checks.
            environment["CARGO_TARGET_DIR"] = str(root / "target")
            with (root / "supervisor.log").open("w+") as output:
                process = subprocess.Popen([sys.executable, str(RUNNER), "--manifest-path", str(root / "Cargo.toml"),
                                            "--bin", "reload-probe", "--", "spaces & $ stay literal"],
                                           env=environment, stdout=output, stderr=output)
                try:
                    records = lambda: log.read_text().splitlines() if log.exists() else []
                    eventually(records)
                    first, directory, argument = records()[0].split("|", 2)
                    self.assertEqual(argument, "spaces & $ stay literal")
                    control = Path(directory)
                    self.assertEqual((control / "enabled").read_text(), "0")
                    main.write_text(source + "\n// saved while off\n")
                    time.sleep(1.1)
                    self.assertEqual(len(records()), 1)
                    runner.write_state(control, "enabled", "1")
                    eventually(lambda: (control / "status").read_text() == "Watching")
                    main.write_text("this will not compile")
                    eventually(lambda: (control / "status").read_text().startswith("Build failed"))
                    self.assertEqual(len(records()), 1)
                    os.kill(int(first), 0)  # Build errors must keep the existing app alive.
                    main.write_text(source + "\n// fixed source\n")
                    eventually(lambda: len(records()) == 2)
                    second = records()[1].split("|", 1)[0]
                    self.assertNotEqual(first, second)
                    runner.write_state(control, "enabled", "0")
                    eventually(lambda: (control / "status").read_text() == "Off")
                    main.write_text(source + "\n// another save while off\n")
                    time.sleep(1.1)
                    self.assertEqual(len(records()), 2)
                    process.terminate()
                    self.assertEqual(process.wait(timeout=10), 0)
                    self.assertFalse(control.exists())
                    with self.assertRaises(ProcessLookupError):
                        os.kill(int(second), 0)
                finally:
                    if process.poll() is None:
                        process.terminate()
                        process.wait(timeout=10)
                    if process.returncode not in (0, None):
                        output.seek(0)
                        print(output.read())


if __name__ == "__main__":
    unittest.main()
