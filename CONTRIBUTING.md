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

Changes target `dev`, then promote through `next` to `main`. Run `make ci` for
checks or `make help` for available commands.

Before promotion, run `make release-prepare` to calculate the next stable version
from conventional commits and synchronize Cargo, the lockfile, Registry metadata,
documentation, skills, and site references. Settings, CLI, and MCP versions derive
from Cargo at build time. Review that diff and update `CHANGELOG.md` before
committing through the promotion PRs. CI and release tagging reject mismatched
versions. See the [release policy](.github/BRANCH-AND-RELEASE-POLICY.md).
