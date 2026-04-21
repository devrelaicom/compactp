//! Typed CLI error with stable exit codes.
//!
//! Every user-facing failure path in the compactp binary constructs a
//! [`CliError`] and returns it via `Result`. `main` maps the error back to
//! a process exit code using [`CliError::exit_code`], so the exit-code
//! table defined in the CONSTITUTION stays authoritative:
//!
//! | Code | Constructor                  | Meaning                              |
//! |------|------------------------------|--------------------------------------|
//! | 1    | [`CliError::runtime`]        | Parse or runtime failure             |
//! | 2    | [`CliError::io`]             | I/O error (unreadable file, stdin…)  |
//! | 3    | [`CliError::usage`]          | Usage error (invalid flag, bad args) |
//! | 4    | [`CliError::internal`]       | Internal invariant violation         |

#[derive(Debug, Clone)]
pub struct CliError {
    exit_code: i32,
    message: String,
}

// Constructors and accessors below are wired step by step across the CLI.
// Step 1 introduces the type and its serde/io conversions; Step 2 wires every
// remaining variant into main and the command dispatch. The `dead_code` allow
// is removed in Step 2 once every constructor has at least one caller.
#[allow(dead_code)]
impl CliError {
    pub fn runtime(message: impl Into<String>) -> Self {
        Self {
            exit_code: 1,
            message: message.into(),
        }
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self {
            exit_code: 2,
            message: message.into(),
        }
    }

    pub fn usage(message: impl Into<String>) -> Self {
        Self {
            exit_code: 3,
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            exit_code: 4,
            message: message.into(),
        }
    }

    pub fn exit_code(&self) -> i32 {
        self.exit_code
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CliError {}

impl From<std::io::Error> for CliError {
    fn from(err: std::io::Error) -> Self {
        CliError::io(format!("io error: {err}"))
    }
}

impl From<serde_json::Error> for CliError {
    fn from(err: serde_json::Error) -> Self {
        CliError::runtime(format!("failed to serialize output: {err}"))
    }
}
