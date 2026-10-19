#!/usr/bin/env bash

set -euo pipefail

rm -f uv.lock

rm -rf .venv || true

uv venv

uv lock && uv sync
