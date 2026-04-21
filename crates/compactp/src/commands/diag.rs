use crate::Cli;
use crate::error::CliError;
use std::path::PathBuf;

// Stub — real implementation in Step 5.
pub fn run(_cli: &Cli, _paths: &[PathBuf]) -> Result<i32, CliError> {
    Err(CliError::usage("diag subcommand not yet implemented"))
}
