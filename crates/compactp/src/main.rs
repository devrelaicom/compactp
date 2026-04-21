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
        /// Paths to watch
        paths: Vec<PathBuf>,
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
    Lex,
    Parse,
    Cst,
    Ast,
    Diag,
    Stats,
}

fn main() {
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
            eprintln!("{}", err.message());
            std::process::exit(err.exit_code());
        }
    }
}
