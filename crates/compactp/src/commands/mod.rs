pub mod ast;
pub mod cst;
pub mod diag;
pub mod lex;
pub mod parse;
pub mod stats;
pub mod watch;

use crate::Cli;
use crate::error::CliError;

pub fn run(cli: Cli) -> Result<i32, CliError> {
    match &cli.command {
        crate::Commands::Lex { paths } => lex::run(&cli, paths),
        crate::Commands::Parse { paths } => parse::run(&cli, paths),
        crate::Commands::Cst { paths } => cst::run(&cli, paths),
        crate::Commands::Ast { paths } => ast::run(&cli, paths),
        crate::Commands::Diag { paths } => diag::run(&cli, paths),
        crate::Commands::Stats { paths } => stats::run(&cli, paths),
        crate::Commands::Watch { command } => watch::run(&cli, command),
    }
}

pub(crate) fn run_watchable(cli: &Cli, command: &crate::WatchableCommand) -> Result<i32, CliError> {
    match command {
        crate::WatchableCommand::Lex { paths } => lex::run(cli, paths),
        crate::WatchableCommand::Parse { paths } => parse::run(cli, paths),
        crate::WatchableCommand::Cst { paths } => cst::run(cli, paths),
        crate::WatchableCommand::Ast { paths } => ast::run(cli, paths),
        crate::WatchableCommand::Diag { paths } => diag::run(cli, paths),
        crate::WatchableCommand::Stats { paths } => stats::run(cli, paths),
    }
}
