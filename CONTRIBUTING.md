# Contributing to Azra

Thank you for your interest in Azra! The project is public, MIT licensed and
hosted at https://github.com/11nawid/azravibe.vibe. The website and hosted
playground live at https://azravibe.com/vibe.

## Current contributor policy

At this stage, direct commit/push access is intentionally limited to the
project owner and the AI assistant working with them. Everyone else is welcome
to:

- open **issues** for bugs, ideas, documentation fixes and questions;
- propose changes through **pull requests**, which the owner reviews and merges;
- use, fork, study and build on the language under the MIT license.

This keeps the early language design coherent while the project is young.
Contributor access may be expanded later; open an issue to discuss it.

## Before you open a pull request

1. Search existing issues first — bug reports should include a minimal `.vibe`
   script, the exact command, expected output and actual output.
2. Keep changes focused. One PR = one concern (feature, fix or docs).
3. Run the checks locally:

   ```bash
   cargo build
   cargo test
   cargo fmt --check
   azra test tests/vibe-test/11_test_framework.vibe
   ```

4. If you change the language, update `docs/language.vibe.txt`, `README.md` and
   the relevant module in `std/` or `tests/vibe-test/`.
5. If you add a built-in, add tests in `tests/features.rs` (or a `.vibe` suite)
   and document the new syntax in the spec.

## Commit message rules

We use **Conventional Commits**:

```
<type>(<scope>): <short imperative summary>

<body: what and why, wrapped at ~72 columns>

<footer: Issue references, e.g. Closes #123>
```

Allowed `type` values:

| Type | Use for |
|------|---------|
| `feat` | New language feature, built-in or CLI command |
| `fix` | Bug fix |
| `docs` | Documentation only |
| `test` | Adding/fixing tests |
| `refactor` | Code change with no behaviour change |
| `perf` | Performance improvement |
| `chore` | Tooling, CI, dependency or housekeeping |

Examples:

```
feat(math): add atan2 and lerp for vec3
fix(lexer): keep hash comments inside indented blocks
docs(spec): document package manifests and test blocks
```

Rules:

- one logical change per commit;
- no merge commits from rebasing feature branches (squash when merging);
- never commit secrets, personal data, `target/` or `.azra/packages/`. The
  checked-in `playground/azra.wasm` is the one intentional build artifact: it
  powers the hosted playground and must be rebuilt when the runtime changes;
- keep commits signed-off only if you own the copyright to your contribution
  (`git commit -s` is optional but welcome).

## Branch naming

`feat/<short-name>`, `fix/<short-name>`, `docs/<short-name>`,
`test/<short-name>`. Keep branches short-lived.

## Code style

- Rust: `cargo fmt` output; no new third-party dependencies without discussion.
- Azra: 4-space indentation, no brackets, English keywords. Use `note:` for
  comments and prefer clear intermediate variables over dense expressions.
- Tests must be deterministic; avoid wall-clock or network dependencies.

## Reporting security issues

Do **not** open a public issue for a vulnerability. Use GitHub's private
"Report a vulnerability" flow on the repository (see `SECURITY.md`).

## License

By contributing you agree that your contribution is licensed under the MIT
License, the same as the rest of the project.

