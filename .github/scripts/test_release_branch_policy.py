"""Exercise the actual promotion gate shell before any checkout or GitHub mutation."""
import os
from pathlib import Path
import subprocess
import textwrap
import unittest

WORKFLOWS = Path(__file__).resolve().parents[1] / "workflows"


class BranchPolicyTests(unittest.TestCase):
    def run_policy(self, head, base, head_repo="owner/repo"):
        workflow = (WORKFLOWS / "next-preview.yml").read_text(encoding="utf-8")
        script = workflow.split("        run: |\n", 1)[1].split("      - uses:", 1)[0]
        env = dict(os.environ, GH_REPO="owner/repo", HEAD_REPO=head_repo,
                   HEAD_REF=head, BASE_REF=base)
        return subprocess.run(["bash", "-c", textwrap.dedent(script)], env=env,
                              capture_output=True, encoding="utf-8")

    def test_only_the_two_promotion_edges_are_allowed(self):
        for head, base in [("dev", "next"), ("next", "main")]:
            with self.subTest(head=head, base=base):
                result = self.run_policy(head, base)
                self.assertEqual(result.returncode, 0, result.stderr)

    def test_other_edges_cannot_bypass_the_promotion_lane(self):
        for head, base in [("feat/example", "next"), ("dev", "main"),
                           ("release/v6.0.0", "main"), ("main", "next"),
                           ("next", "dev")]:
            with self.subTest(head=head, base=base):
                result = self.run_policy(head, base)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("Only dev", result.stderr)

    def test_forks_with_matching_branch_names_are_rejected(self):
        for head, base in [("dev", "next"), ("next", "main")]:
            with self.subTest(head=head, base=base):
                result = self.run_policy(head, base, "fork/repo")
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("this repository", result.stderr)


if __name__ == "__main__":
    unittest.main()
