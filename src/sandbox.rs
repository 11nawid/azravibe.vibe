//! Sandbox / security configuration for embedding Azra in a game client.
//!
//! The CLI runs with a permissive sandbox; a host such as *Azravibe Player*
//! can install [`SandboxConfig::strict`] to disable file/JSON access and cap
//! execution so a runaway script can never hang the game loop.

/// Security and resource limits for script execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxConfig {
    /// Allows `read file`, `write ... to file`, `file exists`, `parse json`
    /// and `to json`. **ON** in the CLI, **OFF** in Azravibe Player.
    pub allow_io: bool,
    /// Maximum statements executed per `eval` / `tick` / `trigger` call.
    /// `usize::MAX` disables the budget (CLI default).
    pub max_instructions_per_tick: usize,
    /// Maximum nested function calls before a stack-overflow error.
    pub max_call_depth: usize,
}

impl SandboxConfig {
    /// CLI default: everything allowed, effectively unlimited instructions.
    pub fn permissive() -> Self {
        SandboxConfig {
            allow_io: true,
            max_instructions_per_tick: usize::MAX,
            max_call_depth: 64,
        }
    }

    /// Embedding default: no file/JSON access, bounded execution.
    pub fn strict() -> Self {
        SandboxConfig {
            allow_io: false,
            max_instructions_per_tick: 100_000,
            max_call_depth: 64,
        }
    }
}

impl Default for SandboxConfig {
    fn default() -> Self {
        SandboxConfig::permissive()
    }
}
