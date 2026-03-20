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
    /// Watch files and re-run on changes
    Watch {
        /// Command to run on changes
        #[command(subcommand)]
        command: WatchCommand,
        /// Paths to watch
        paths: Vec<PathBuf>,
    },
}

#[derive(Subcommand)]
enum WatchCommand {
    /// Watch and parse
    Parse,
    /// Watch and show CST
    Cst,
    /// Watch and show stats
    Stats,
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
            // clap uses DisplayHelp/DisplayVersion for --help/--version (exit 0),
            // and other error kinds for actual usage errors (exit 3 per design spec)
            let code = if e.use_stderr() { 3 } else { 0 };
            e.print().ok();
            std::process::exit(code);
        }
    };

    let json = matches!(cli.format, OutputFormat::Json);

    let pretty = cli.pretty;

    let exit_code = match cli.command {
        Commands::Lex { ref paths } => run_on_inputs(paths, &cli, |source, name| {
            commands::lex::run(source, name, json, cli.timing, pretty)
        }),
        Commands::Parse { ref paths } => run_on_inputs(paths, &cli, |source, name| {
            commands::parse::run(
                source,
                name,
                json,
                cli.timing,
                cli.no_recover,
                cli.max_errors,
                pretty,
            )
        }),
        Commands::Cst { ref paths } => run_on_inputs(paths, &cli, |source, name| {
            commands::cst::run(source, name, json, cli.timing, pretty)
        }),
        Commands::Stats { ref paths } => run_on_inputs(paths, &cli, |source, name| {
            commands::stats::run(source, name, json, cli.timing, pretty)
        }),
        Commands::Watch {
            ref command,
            ref paths,
        } => {
            let cmd = command;
            let json_flag = json;
            let timing_flag = cli.timing;
            let no_recover_flag = cli.no_recover;
            let max_errs = cli.max_errors;
            let pretty_flag = pretty;
            let stdin_fn = cli.stdin_filename.clone();
            if let Err(e) = commands::watch::run(paths, |watch_paths| {
                let inputs =
                    input::resolve_inputs(watch_paths, stdin_fn.as_deref()).unwrap_or_default();
                for inp in inputs {
                    if let input::InputSource::File(path) = inp
                        && let Ok(source) = std::fs::read_to_string(&path)
                    {
                        let name = path.display().to_string();
                        match cmd {
                            WatchCommand::Parse => {
                                commands::parse::run(
                                    &source,
                                    &name,
                                    json_flag,
                                    timing_flag,
                                    no_recover_flag,
                                    max_errs,
                                    pretty_flag,
                                );
                            }
                            WatchCommand::Cst => {
                                commands::cst::run(
                                    &source,
                                    &name,
                                    json_flag,
                                    timing_flag,
                                    pretty_flag,
                                );
                            }
                            WatchCommand::Stats => {
                                commands::stats::run(
                                    &source,
                                    &name,
                                    json_flag,
                                    timing_flag,
                                    pretty_flag,
                                );
                            }
                        }
                    }
                }
            }) {
                eprintln!("Watch error: {e}");
                4 // Internal failure
            } else {
                0
            }
        }
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
