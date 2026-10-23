"""Check all current release references against the root Cargo package version."""
from release_version import ROOT, check_references

try:
    check_references(ROOT)
except ValueError as error:
    raise SystemExit(str(error)) from error
