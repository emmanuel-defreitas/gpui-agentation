# Automatic MCP Registry publication

The `Publish MCP Bridge` workflow now chains its jobs as follows:

```text
verify → publish to crates.io → publish and verify MCP Registry
```

The Registry job requires a successful Cargo publish and `publish=true`.
It checks out the same verified commit, reuses the main-only `crates-io`
environment, and obtains short-lived authentication through GitHub OIDC.
The official publisher release and Linux archive SHA-256 are pinned together;
no persistent Registry token or new third-party action is required.

The public Cargo version must exist and not be yanked. The final Registry
version must be active and exactly match the reviewed `server.json`.
Transient reads and propagation delays receive bounded retries. Writes are
not blindly retried. A matching existing version is a successful no-op;
conflicting or inactive metadata stops the job.

If only the Registry job fails after a Cargo upload, rerun that failed job.
Do not republish the crate. Dry runs skip the Registry job entirely. The
successful job records a link to the verified listing in its run summary.

Validation uses `make version-check version-test`, `actionlint`, the pinned
publisher archive digest, and read-only verification of an existing listing.
An authenticated GitHub OIDC publication can run only after this workflow
reaches protected `main` and a Cargo publication succeeds.
