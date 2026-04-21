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
/// - Explicitly-named files (symlinks included) are always followed — build
///   systems frequently stage sources as symlinks, and the user opted in by
///   naming the path directly.
/// - Directories are walked recursively, collecting only `*.compact` files.
///   Symlinks encountered during the walk are skipped to prevent cycles from
///   exhausting the stack or FD table, and a warning is printed to stderr so
///   users notice that their symlinked subtrees were not parsed.
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
    // Explicit paths follow symlinks (`fs::metadata`) — the user named the
    // path directly, so they opted in.
    let meta = fs::metadata(path)
        .map_err(|err| CliError::io(format!("failed to read {}: {err}", path.display())))?;
    let ty = meta.file_type();

    if ty.is_file() {
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
            // Visible warning so users notice that a symlinked subtree was
            // skipped rather than silently producing a smaller output.
            eprintln!(
                "warning: skipping symlink at {} (pass the path explicitly to follow)",
                path.display()
            );
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
