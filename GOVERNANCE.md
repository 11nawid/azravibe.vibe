# Governance

Azra is an open-source project maintained in the public repository
https://github.com/11nawid/azravibe.vibe. The language website is
https://azravibe.com/vibe.

## Roles

- **Project owner** — makes final decisions about language design, releases and
  repository access. Currently the original author.
- **AI assistant** — implements, tests and documents changes alongside the
  owner. It has commit access but does not set project policy.
- **Contributors** — anyone who reports issues, proposes PRs, writes examples,
  teaches the language or improves documentation. See `CONTRIBUTING.md`.

At this early stage, direct push access is limited to the owner and the AI
assistant. Community work is accepted through issues and pull requests and is
merged by the owner. This will be revisited as the project grows.

## Decision making

- Small, reversible changes (typos, tests, docs) can be merged after review.
- Language syntax or semantics changes require a clear rationale, an update to
  `docs/language.vibe.txt`, and tests in `tests/` or `tests/vibe-test/`.
- Breaking changes should be discussed in an issue first and recorded in
  `CHANGELOG.md`.
- Disagreements are resolved by the project owner after open discussion.

## Releases

Releases are tagged with semantic versions (`v0.x.y`). Each release should
include a `CHANGELOG.md` entry, passing CI and updated documentation. The
WebAssembly playground binary is rebuilt for releases that change the runtime.

## License

MIT. See `LICENSE`.

