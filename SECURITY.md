# Security Policy

## Supported versions

Security fixes are applied to the latest published release and the `main`
branch. The project is pre-1.0, so older versions may not receive backports.

## Reporting a vulnerability

Please **do not** open a public issue for a security problem. Use GitHub's
private vulnerability reporting flow:

1. Go to https://github.com/11nawid/azravibe.vibe/security/advisories/new
2. Describe the issue, the affected version, a minimal reproduction and the
   impact you believe it has.

You should receive an acknowledgement within a few days. We will coordinate a
fix and a public disclosure with you before publishing details.

## Scope notes

- The standalone CLI uses a permissive sandbox (file I/O allowed). Scripts are
  code: only run `.vibe` files you trust.
- Embedders should use `AzraEngine::with_sandbox(SandboxConfig::strict())` for
  untrusted scripts; it disables I/O and caps instructions and call depth.
- The playground compiles the interpreter to WebAssembly and runs entirely in
  the browser; it does not have filesystem access.
