use crate::error::CliError;
use std::collections::BTreeSet;
use std::fs;
use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};

/// A resolved input: filename label plus loaded source text.
#[derive(Debug, Clone)]
pub struct InputFile {
    pub label: String,
    pub source: String,
}

/// Resolve CLI path arguments into a deterministic list of loaded .compact files.
///
/// Rules:
/// - Empty `paths` reads from stdin (unless stdin is an interactive terminal,
///   in which case the CLI errors rather than hang).
/// - A single `-` in `paths` reads from stdin. Mixing `-` with other paths is
///   a usage error — we refuse to silently drop either side.
/// - Files are included when explicitly named (extension-independent).
/// - Directories are walked recursively, collecting only `*.compact` files.
///   The walk follows regular directories and files but does not traverse
///   symlinks, so cycles cannot exhaust the stack or the FD table.
pub fn resolve_inputs(
    paths: &[PathBuf],
    stdin_filename: Option<&str>,
) -> Result<Vec<InputFile>, CliError> {
    let has_stdin_marker = paths.iter().any(|p| p.as_os_str() == "-");
    let has_other_path = paths.iter().any(|p| p.as_os_str() != "-");

    if has_stdin_marker && has_other_path {
        return Err(CliError::usage(
            "cannot mix `-` (stdin) with other input paths; pass one or the other",
        ));
    }

    if paths.is_empty() {
        if std::io::stdin().is_terminal() {
            return Err(CliError::usage(
                "no input paths provided and stdin is a terminal; pass a path, pipe input, or use `-`",
            ));
        }
        return Ok(vec![read_stdin(stdin_filename)?]);
    }

    if has_stdin_marker {
        return Ok(vec![read_stdin(stdin_filename)?]);
    }

    let mut files = BTreeSet::new();
    for path in paths {
        collect(path, &mut files)?;
    }

    files
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path)
                .map_err(|err| CliError::io(format!("failed to read {}: {err}", path.display())))?;
            Ok(InputFile {
                label: path.display().to_string(),
                source,
            })
        })
        .collect()
}

fn read_stdin(stdin_filename: Option<&str>) -> Result<InputFile, CliError> {
    let mut source = String::new();
    std::io::stdin()
        .read_to_string(&mut source)
        .map_err(|err| CliError::io(format!("failed to read stdin: {err}")))?;
    Ok(InputFile {
        label: stdin_filename.unwrap_or("<stdin>").to_string(),
        source,
    })
}

fn collect(path: &Path, files: &mut BTreeSet<PathBuf>) -> Result<(), CliError> {
    let meta = fs::symlink_metadata(path)
        .map_err(|err| CliError::io(format!("failed to read {}: {err}", path.display())))?;
    let ty = meta.file_type();

    if ty.is_symlink() {
        // Explicit path that resolves to a symlink: don't follow. Give the user
        // a precise message rather than dereferencing and risking a cycle.
        return Err(CliError::io(format!(
            "refusing to follow symlink at {}",
            path.display()
        )));
    }

    if ty.is_file() {
        // Explicitly-named file: include regardless of extension.
        files.insert(path.to_path_buf());
        return Ok(());
    }

    if ty.is_dir() {
        walk_dir(path, files)?;
        return Ok(());
    }

    Err(CliError::io(format!(
        "unsupported input path {}",
        path.display()
    )))
}

fn walk_dir(dir: &Path, files: &mut BTreeSet<PathBuf>) -> Result<(), CliError> {
    for entry in fs::read_dir(dir)
        .map_err(|err| CliError::io(format!("failed to read directory {}: {err}", dir.display())))?
    {
        let entry =
            entry.map_err(|err| CliError::io(format!("failed to read directory entry: {err}")))?;
        let path = entry.path();
        let ty = entry
            .file_type()
            .map_err(|err| CliError::io(format!("failed to stat {}: {err}", path.display())))?;

        if ty.is_symlink() {
            // Skip symlinks in directory walks: the parent directory may contain
            // loops or point out of the tree.
            continue;
        }
        if ty.is_dir() {
            walk_dir(&path, files)?;
        } else if ty.is_file() && path.extension().is_some_and(|ext| ext == "compact") {
            files.insert(path);
        }
    }
    Ok(())
}
