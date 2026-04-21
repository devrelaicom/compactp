pub mod ast;
pub mod cst;
pub mod diag;
pub mod lex;
pub mod parse;
pub mod stats;
pub mod watch;

use crate::Cli;
use crate::error::CliError;
use std::path::PathBuf;

pub fn run(cli: Cli) -> Result<i32, CliError> {
    match &cli.command {
        crate::Commands::Lex { paths } => lex::run(&cli, paths),
        crate::Commands::Parse { paths } => parse::run(&cli, paths),
        crate::Commands::Cst { paths } => cst::run(&cli, paths),
        crate::Commands::Ast { paths } => ast::run(&cli, paths),
        crate::Commands::Diag { paths } => diag::run(&cli, paths),
        crate::Commands::Stats { paths } => stats::run(&cli, paths),
        crate::Commands::Watch { command, paths } => watch::run(&cli, command, paths),
    }
}

pub(crate) fn run_watchable(
    cli: &Cli,
    command: &crate::WatchableCommand,
    paths: &[PathBuf],
) -> Result<i32, CliError> {
    match command {
        crate::WatchableCommand::Lex => lex::run(cli, paths),
        crate::WatchableCommand::Parse => parse::run(cli, paths),
        crate::WatchableCommand::Cst => cst::run(cli, paths),
        crate::WatchableCommand::Ast => ast::run(cli, paths),
        crate::WatchableCommand::Diag => diag::run(cli, paths),
        crate::WatchableCommand::Stats => stats::run(cli, paths),
    }
}
