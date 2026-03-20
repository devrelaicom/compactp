mod commands;
mod input;
mod output;

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "compactp", about = "Compact language parser", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output format
    #[arg(long, default_value = "human", global = true)]
    format: OutputFormat,

    /// Pretty-print JSON output
    #[arg(long, global = true)]
    pretty: bool,

    /// Show timing information
    #[arg(long, global = true)]
    timing: bool,

    /// Filename to use when reading from stdin
    #[arg(long, global = true)]
    stdin_filename: Option<String>,

    /// Maximum parse errors before the parser stops recovery
    #[arg(long, global = true)]
    max_errors: Option<usize>,

    /// Disable error recovery
    #[arg(long, global = true)]
    no_recover: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Tokenize and print token stream
    Lex {
        /// Input files or directories
        paths: Vec<PathBuf>,
    },
    /// Parse and report diagnostics
    Parse {
        /// Input files or directories
        paths: Vec<PathBuf>,
    },
    /// Dump concrete syntax tree
    Cst {
        /// Input files or directories
        paths: Vec<PathBuf>,
    },
    /// Report token/node counts and parse time
    Stats {
        /// Input files or directories
        paths: Vec<PathBuf>,
    },
}

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Human,
    Json,
}

fn main() {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            e.print().ok();
            // Exit code 3 for invalid CLI usage per design spec
            std::process::exit(3);
        }
    };

    let json = matches!(cli.format, OutputFormat::Json);

    let exit_code = match cli.command {
        Commands::Lex { ref paths } => run_on_inputs(paths, &cli, |source, name| {
            commands::lex::run(source, name, json, cli.timing)
        }),
        Commands::Parse { ref paths } => run_on_inputs(paths, &cli, |source, name| {
            commands::parse::run(
                source,
                name,
                json,
                cli.timing,
                cli.no_recover,
                cli.max_errors,
            )
        }),
        Commands::Cst { ref paths } => run_on_inputs(paths, &cli, |source, name| {
            commands::cst::run(source, name, json, cli.timing)
        }),
        Commands::Stats { ref paths } => run_on_inputs(paths, &cli, |source, name| {
            commands::stats::run(source, name, json, cli.timing)
        }),
    };

    std::process::exit(exit_code);
}

fn run_on_inputs<F>(paths: &[PathBuf], cli: &Cli, mut run_fn: F) -> i32
where
    F: FnMut(&str, &str) -> i32,
{
    let inputs = match input::resolve_inputs(paths, cli.stdin_filename.as_deref()) {
        Ok(inputs) => inputs,
        Err(e) => {
            eprintln!("error: {e}");
            return 2; // IO error
        }
    };

    let mut worst_exit = 0;

    for input in inputs {
        match input {
            input::InputSource::File(path) => {
                let source = match std::fs::read_to_string(&path) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("error: {}: {e}", path.display());
                        worst_exit = worst_exit.max(2);
                        continue;
                    }
                };
                let name = path.display().to_string();
                let code = run_fn(&source, &name);
                worst_exit = worst_exit.max(code);
            }
            input::InputSource::Stdin { filename } => {
                let source = match std::io::read_to_string(std::io::stdin()) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("error: reading stdin: {e}");
                        worst_exit = worst_exit.max(2);
                        continue;
                    }
                };
                let name = filename.as_deref().unwrap_or("<stdin>");
                let code = run_fn(&source, name);
                worst_exit = worst_exit.max(code);
            }
        }
    }

    worst_exit
}
