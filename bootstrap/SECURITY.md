# Azra build security — honest position

**No executable is truly non-reverse-engineerable.** Anyone who controls the
machine can dump memory, single-step, patch binaries, or emulate the runtime.
What Azra can do is make casual decompilation and tampering expensive.

## Implemented in Azra (Round 2)

- `protect.vibe` seals bytecode bundles with **HMAC-SHA256** using a build
  secret. Any modification to `app.azb` fails with a clear integrity error.
- The payload is base64url-encoded so plain bytecode is not visible at a glance.
- `codegen.vibe` emits raw PE32+ bytes in Azra with no symbols or debug data.

## Planned hardening

- Per-build random keys (not a fixed build key) and key derivation.
- Encrypted .text section with a runtime decryption stub.
- Control-flow flattening and junk-byte insertion in emitted code.
- Anti-debug checks and runtime self-integrity verification.
- Code virtualization for sensitive routines.

## Limits

HMAC/base64 are obfuscation and integrity, not encryption against a determined
attacker with the binary. Real secrecy requires encryption with a key held off
the machine, which cannot protect software from its legitimate user.
