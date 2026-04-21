mod commands;
mod error;
mod input;
mod output;

use clap::{Parser, Subcommand, ValueEnum, error::ErrorKind};
use std::path::PathBuf;

#[derive(Debug, Parser, Clone)]
#[command(name = "compactp", about = "Compact language parser", version)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output format
    #[arg(long, default_value = "human", global = true)]
    format: OutputFormat,

    /// Pretty-print JSON output
    #[arg(long, global = true)]
    pretty: bool,

    /// ANSI color policy for human output
    #[arg(long, default_value = "auto", global = true)]
    color: ColorChoice,

    /// Show timing information
    #[arg(long, global = true)]
    timing: bool,

    /// Filename to use when reading from stdin
    #[arg(long, global = true)]
    stdin_filename: Option<String>,

    /// Cap the number of diagnostics emitted per input
    #[arg(long, global = true)]
    max_diagnostics: Option<usize>,

    /// Maximum parse errors before the parser stops recovery
    #[arg(long, global = true)]
    max_errors: Option<usize>,

    /// Disable error recovery
    #[arg(long, global = true)]
    no_recover: bool,
}

#[derive(Debug, Subcommand, Clone)]
pub enum Commands {
    /// Tokenize and print token stream
    Lex { paths: Vec<PathBuf> },
    /// Parse and report diagnostics
    Parse { paths: Vec<PathBuf> },
    /// Dump concrete syntax tree
    Cst { paths: Vec<PathBuf> },
    /// Dump typed abstract syntax tree
    Ast { paths: Vec<PathBuf> },
    /// Emit diagnostics only
    Diag { paths: Vec<PathBuf> },
    /// Report token/node counts and parse time
    Stats { paths: Vec<PathBuf> },
    /// Watch files and re-run on changes
    Watch {
        /// Command to run on change
        #[command(subcommand)]
        command: WatchableCommand,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Subcommand, Clone)]
pub enum WatchableCommand {
    /// Tokenize on change
    Lex { paths: Vec<PathBuf> },
    /// Parse on change
    Parse { paths: Vec<PathBuf> },
    /// Dump concrete syntax tree on change
    Cst { paths: Vec<PathBuf> },
    /// Dump typed abstract syntax tree on change
    Ast { paths: Vec<PathBuf> },
    /// Emit diagnostics only on change
    Diag { paths: Vec<PathBuf> },
    /// Report stats on change
    Stats { paths: Vec<PathBuf> },
}

impl WatchableCommand {
    pub fn paths(&self) -> &[PathBuf] {
        match self {
            WatchableCommand::Lex { paths }
            | WatchableCommand::Parse { paths }
            | WatchableCommand::Cst { paths }
            | WatchableCommand::Ast { paths }
            | WatchableCommand::Diag { paths }
            | WatchableCommand::Stats { paths } => paths,
        }
    }
}

fn main() {
    reset_sigpipe_to_default();

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            err.print().ok();
            let code = match err.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => 0,
                _ => 3,
            };
            std::process::exit(code);
        }
    };

    match commands::run(cli) {
        Ok(code) => std::process::exit(code),
        Err(err) => {
            // Intentionally `let _ =` — if stderr is also closed we still want
            // to surface the numeric exit code, not trigger a secondary panic.
            let _ = writeln!(std::io::stderr(), "{}", err.message());
            std::process::exit(err.exit_code());
        }
    }
}

/// Restore SIGPIPE to its default action on Unix so downstream pipe closures
/// (e.g. `compactp lex big.compact | head`) terminate the process cleanly
/// with status 141 instead of triggering a Rust panic inside `println!`.
/// Rust's default is to ignore SIGPIPE, which causes `write!` / `println!`
/// to fail with `BrokenPipe` and the standard print macros panic on that
/// error. Most Unix CLI tools reset the signal explicitly.
#[cfg(unix)]
fn reset_sigpipe_to_default() {
    // SAFETY: calling `signal(SIGPIPE, SIG_DFL)` is safe; it just rewrites
    // the signal disposition table. No data-race concerns at program start.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

#[cfg(not(unix))]
fn reset_sigpipe_to_default() {}

use std::io::Write;
