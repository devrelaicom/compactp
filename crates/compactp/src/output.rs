use crate::error::CliError;
use serde::Serialize;

/// JSON output envelope wrapping all command output.
///
/// Every JSON payload includes metadata for version tracking and downstream consumers.
#[derive(Debug, Clone, Serialize)]
pub struct OutputEnvelope<T: Serialize> {
    pub tool_version: &'static str,
    pub schema_version: u32,
    pub language_version: &'static str,
    pub input: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timing_ms: Option<f64>,
    pub data: T,
}

pub const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SCHEMA_VERSION: u32 = 1;
pub const LANGUAGE_VERSION: &str = "0.22.0";

impl<T: Serialize> OutputEnvelope<T> {
    pub fn new(input: String, data: T, timing_ms: Option<f64>) -> Self {
        Self {
            tool_version: TOOL_VERSION,
            schema_version: SCHEMA_VERSION,
            language_version: LANGUAGE_VERSION,
            input,
            timing_ms,
            data,
        }
    }
}

pub fn print_json<T: Serialize>(value: &T, pretty: bool) -> Result<(), CliError> {
    let rendered = if pretty {
        serde_json::to_string_pretty(value)?
    } else {
        serde_json::to_string(value)?
    };
    println!("{rendered}");
    Ok(())
}

/// Resolve the user's `--color` choice against the current stdout terminal
/// state. `auto` activates ANSI colour only when stdout is a terminal;
/// `always`/`never` override unconditionally.
pub fn use_color(choice: crate::ColorChoice) -> bool {
    use std::io::IsTerminal;
    match choice {
        crate::ColorChoice::Always => true,
        crate::ColorChoice::Never => false,
        crate::ColorChoice::Auto => std::io::stdout().is_terminal(),
    }
}
