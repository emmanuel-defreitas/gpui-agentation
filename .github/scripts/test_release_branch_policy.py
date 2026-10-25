"""Run the actual policy workflow shell blocks with a recording GitHub CLI stub."""
import os
from pathlib import Path
import subprocess
import tempfile
import textwrap
import unittest

WORKFLOWS = Path(__file__).resolve().parents[1] / "workflows"
MODEL = "ai/v6-01-annotation-model"
TOOLS = "ai/v6-02-annotation-tools"
RELEASE = "release/v6.0.0"


class BranchPolicyTests(unittest.TestCase):
    def run_policy(self, workflow, head, base, head_repo="owner/repo", title="feat: annotations [1/3]"):
        script = (WORKFLOWS / workflow).read_text(encoding="utf-8").split("        run: |\n", 1)[1]
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            gh = directory / "gh"
            gh.write_text('#!/bin/sh\nprintf "%s\\n" "$*" >> "$GH_CALLS"\n', encoding="utf-8")
            gh.chmod(0o755)
            calls = directory / "calls"
            env = dict(os.environ, PATH=f"{directory}:{os.environ['PATH']}", GH_CALLS=str(calls),
                       GH_REPO="owner/repo", HEAD_REPO=head_repo, PR_NUMBER="123", PR_URL="https://example.invalid/123",
                       HEAD_REF=head, BASE_REF=base, TITLE=title)
            result = subprocess.run(["bash", "-c", textwrap.dedent(script)], env=env,
                                    capture_output=True, encoding="utf-8")
            return result, calls.read_text(encoding="utf-8") if calls.exists() else ""

    def test_exact_stack_edges_and_sequential_dev_retargets_are_allowed(self):
        for head, base in [(MODEL, "dev"), (TOOLS, MODEL), (RELEASE, TOOLS),
                           (TOOLS, "dev"), (RELEASE, "dev"), ("dev", "next"), ("next", "main")]:
            with self.subTest(head=head, base=base):
                result, calls = self.run_policy("pr-base-policy.yml", head, base)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(calls, "")

    def test_unapproved_edges_and_fork_aliases_are_retargeted(self):
        for head, base, repo in [(TOOLS, MODEL, "fork/repo"), (RELEASE, MODEL, "owner/repo"),
                                 ("ai/unapproved", TOOLS, "owner/repo"), (TOOLS, "next", "owner/repo")]:
            with self.subTest(head=head, base=base, repo=repo):
                result, calls = self.run_policy("pr-base-policy.yml", head, base, repo)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn("pr edit 123 --base dev", calls)

    def test_stack_heads_cannot_bypass_production_lane(self):
        for head in (MODEL, TOOLS, RELEASE):
            result, calls = self.run_policy("pr-base-policy.yml", head, "main")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("pr close 123", calls)

    def test_stack_branches_do_not_create_tracking_issues(self):
        for head in (MODEL, TOOLS, RELEASE):
            result, calls = self.run_policy("pr-branch-enforcement.yml", head, "dev")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(calls, "")

    def test_stack_titles_keep_conventional_semver_prefixes(self):
        for title, valid in [("feat: add textboxes [1/3]", True), ("chore: prepare v6 [3/3]", True),
                             ("[1/3] add textboxes", False), ("✨ feat: textboxes", False)]:
            result, _ = self.run_policy("pr-semver.yml", MODEL, "dev", title=title)
            self.assertEqual(result.returncode == 0, valid, result.stderr)


if __name__ == "__main__":
    unittest.main()
