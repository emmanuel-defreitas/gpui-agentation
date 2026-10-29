# Contributing

Contributions are what make the open source community such an amazing place to
learn, inspire, and create. Any contributions you make are **greatly appreciated**.

If you have a suggestion that would make this better, please fork the repo and
create a pull request. You can also simply open an issue with the tag
"enhancement". Don't forget to give the project a star! Thanks again!

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/123-amazing-feature`)
3. Commit your Changes (`git commit -m 'feat: add amazing feature'`)
4. Push to the Branch (`git push origin feature/123-amazing-feature`)
5. Open a Pull Request

The native library supports macOS only. Changes target `dev`, then promote
through `next` to `main`. Run `make verify` (or its `make ci` alias) locally on
macOS before merging, and rerun it on the release candidate before promotion.
Run `make site-check` when website files change. Use `make help` for commands.

GitHub Actions only validates promotion metadata and handles promotion/release
operations. It does not run Rust compilation, tests, Clippy, website checks,
AI reviews, or issue automation. Maintainers review the local verification
results; a green promotion workflow confirms metadata, not test success.

Before promotion, run `make release-prepare` to calculate the next stable version
from conventional commits and synchronize Cargo, the lockfile, Registry metadata,
documentation, skills, and site references. Settings, CLI, and MCP versions derive
from Cargo at build time. Review that diff and update `CHANGELOG.md` before
committing through the promotion PRs. Promotion checks and release tagging reject mismatched
versions. See the [release policy](.github/BRANCH-AND-RELEASE-POLICY.md).
