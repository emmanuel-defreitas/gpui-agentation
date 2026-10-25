"""Publish reviewed MCP metadata after its Cargo version is publicly available."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import time
from urllib.error import HTTPError, URLError
from urllib.parse import quote
from urllib.request import Request, urlopen

REGISTRY = "https://registry.modelcontextprotocol.io"
OFFICIAL = "io.modelcontextprotocol.registry/official"


class Pending(RuntimeError):
    """A just-published version is not visible yet."""


def read_json(url):
    request = Request(url, headers={
        "User-Agent": "gpui-agentation-release (+https://github.com/emmanuel-defreitas/gpui-agentation)",
        "Cache-Control": "no-cache",
    })
    try:
        with urlopen(request, timeout=15) as response:
            return json.load(response)
    except HTTPError as error:
        error.close()
        if error.code == 404:
            return None
        raise


def retry(operation, attempts=6, delay=5):
    for attempt in range(attempts):
        try:
            return operation()
        except (Pending, URLError, TimeoutError) as error:
            # Authentication, validation and other permanent HTTP errors fail fast.
            if isinstance(error, HTTPError) and error.code != 429 and error.code < 500:
                raise
            if attempt + 1 == attempts:
                raise
            time.sleep(delay)


def require_crate(package):
    name, version = package["identifier"], package["version"]
    url = f"https://crates.io/api/v1/crates/{quote(name, safe='')}/{quote(version, safe='')}"
    response = read_json(url)
    if response is None:
        raise Pending(f"Cargo package {name}@{version} is not visible yet")
    published = response["version"]
    if published["num"] != version or published["yanked"]:
        raise ValueError(f"Cargo package {name}@{version} must exist and not be yanked")


def verify_listing(expected, response):
    if response is None:
        raise Pending("MCP Registry listing is not visible yet")
    if response["server"] != expected:
        raise ValueError("Existing MCP Registry metadata differs from reviewed server.json")
    if response["_meta"][OFFICIAL]["status"] != "active":
        raise ValueError("MCP Registry version must be active")


def publish(manifest, publisher="mcp-publisher", check_only=False):
    expected = json.loads(manifest.read_text(encoding="utf-8"))
    retry(lambda: require_crate(expected["packages"][0]))
    url = (f"{REGISTRY}/v0.1/servers/{quote(expected['name'], safe='')}/versions/"
           f"{quote(expected['version'], safe='')}")
    existing = retry(lambda: read_json(url))
    if existing is not None:
        # A retry must never overwrite conflicting or inactive immutable metadata.
        verify_listing(expected, existing)
        print("Matching Registry version already exists; skipping publication")
    elif not check_only:
        subprocess.run([publisher, "publish", str(manifest)], check=True)
    retry(lambda: verify_listing(expected, read_json(url)))
    print(f"Verified active MCP Registry version {expected['name']}@{expected['version']}: {url}")
    if summary := os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(summary, "a", encoding="utf-8") as file:
            file.write(f"Verified [MCP Registry {expected['version']}]({url}).\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--server-json", type=Path, default=Path("server.json"))
    parser.add_argument("--publisher", default="mcp-publisher")
    parser.add_argument("--check-only", action="store_true", help="Verify without publishing")
    args = parser.parse_args()
    publish(args.server_json, args.publisher, args.check_only)


if __name__ == "__main__":
    main()
