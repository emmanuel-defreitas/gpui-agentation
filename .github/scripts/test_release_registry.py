"""Exercise publication ordering, safe retries and public Registry verification."""
import contextlib
import io
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import Mock, patch
from urllib.error import HTTPError, URLError

import publish_mcp_registry as registry


class RegistryPublishingTests(unittest.TestCase):
    def setUp(self):
        self.enterContext(contextlib.redirect_stdout(io.StringIO()))
        self.enterContext(patch.dict(registry.os.environ, {"GITHUB_STEP_SUMMARY": ""}))
        folder = Path(self.enterContext(tempfile.TemporaryDirectory()))
        self.manifest = folder / "server.json"
        self.expected = json.loads((Path(__file__).resolve().parents[2] / "server.json").read_text())
        self.manifest.write_text(json.dumps(self.expected), encoding="utf-8")
        self.crate = {"version": {"num": self.expected["version"], "yanked": False}}
        self.listing = {"server": self.expected, "_meta": {registry.OFFICIAL: {"status": "active"}}}
        self.sleep = self.enterContext(patch.object(registry.time, "sleep"))
        self.run = self.enterContext(patch.object(registry.subprocess, "run"))

    def test_publish_waits_for_cargo_then_verifies_registry_visibility(self):
        with patch.object(registry, "read_json", side_effect=[
            None, self.crate, None, None, self.listing,
        ]):
            registry.publish(self.manifest, "/verified/mcp-publisher")
        self.run.assert_called_once_with(
            ["/verified/mcp-publisher", "publish", str(self.manifest)], check=True)
        self.assertEqual(self.sleep.call_count, 2)

    def test_missing_or_yanked_crate_never_publishes_registry(self):
        for response in [None, {"version": {"num": self.expected["version"], "yanked": True}}]:
            with self.subTest(response=response), patch.object(registry, "read_json", return_value=response):
                with self.assertRaises((registry.Pending, ValueError)):
                    registry.publish(self.manifest)
        self.run.assert_not_called()

    def test_matching_existing_listing_is_safe_to_retry(self):
        with patch.object(registry, "read_json", side_effect=[self.crate, self.listing, self.listing]):
            registry.publish(self.manifest)
        self.run.assert_not_called()

    def test_conflicting_or_inactive_listing_is_never_overwritten(self):
        conflicting = {**self.listing, "server": {**self.expected, "description": "Other metadata"}}
        inactive = {**self.listing, "_meta": {registry.OFFICIAL: {"status": "deprecated"}}}
        for response in [conflicting, inactive]:
            with self.subTest(response=response), patch.object(registry, "read_json", side_effect=[self.crate, response]):
                with self.assertRaises(ValueError):
                    registry.publish(self.manifest)
        self.run.assert_not_called()

    def test_check_only_verifies_without_writing(self):
        with patch.object(registry, "read_json", side_effect=[self.crate, None] + [None] * 6):
            with self.assertRaises(registry.Pending):
                registry.publish(self.manifest, check_only=True)
        self.run.assert_not_called()

    def test_publish_failure_propagates_without_retrying_the_write(self):
        self.run.side_effect = subprocess.CalledProcessError(1, "mcp-publisher")
        with patch.object(registry, "read_json", side_effect=[self.crate, None]):
            with self.assertRaises(subprocess.CalledProcessError):
                registry.publish(self.manifest)
        self.assertEqual(self.run.call_count, 1)

    def test_transient_reads_retry_but_authentication_errors_fail_fast(self):
        transient = [URLError("temporarily unavailable"), HTTPError("url", 429, "rate limit", {}, None),
                     HTTPError("url", 503, "unavailable", {}, None)]
        for error in transient:
            if isinstance(error, HTTPError):
                self.addCleanup(error.close)
            with self.subTest(error=error):
                operation = Mock(side_effect=[error, "ready"])
                self.assertEqual(registry.retry(operation), "ready")
                self.assertEqual(operation.call_count, 2)
        unauthorized = HTTPError("url", 401, "unauthorized", {}, None)
        self.addCleanup(unauthorized.close)
        operation = Mock(side_effect=unauthorized)
        with self.assertRaises(HTTPError):
            registry.retry(operation)
        self.assertEqual(operation.call_count, 1)

    def test_missing_registry_record_closes_the_response(self):
        body = io.BytesIO(b"not found")
        error = HTTPError("url", 404, "not found", {}, body)
        with patch.object(registry, "urlopen", side_effect=error):
            self.assertIsNone(registry.read_json("https://registry.modelcontextprotocol.io/version"))
        self.assertTrue(body.closed)


if __name__ == "__main__":
    unittest.main()
