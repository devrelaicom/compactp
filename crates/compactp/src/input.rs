use crate::error::CliError;
use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

/// A resolved input: filename label plus loaded source text.
#[derive(Debug, Clone)]
pub struct InputFile {
    pub label: String,
    pub source: String,
}

/// Resolve CLI path arguments into a deterministic list of loaded .compact files.
///
/// - Empty paths, or a single "-" argument, read from stdin
/// - Single files are included as-is
/// - Directories are walked recursively, collecting *.compact files in sorted order
pub fn resolve_inputs(
    paths: &[PathBuf],
    stdin_filename: Option<&str>,
) -> Result<Vec<InputFile>, CliError> {
    if paths.is_empty() || (paths.len() == 1 && paths[0].as_os_str() == "-") {
        return Ok(vec![read_stdin(stdin_filename)?]);
    }

    let mut files = BTreeSet::new();
    for path in paths {
        if path.as_os_str() == "-" {
            return Ok(vec![read_stdin(stdin_filename)?]);
        }
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
    let meta = fs::metadata(path)
        .map_err(|err| CliError::io(format!("failed to read {}: {err}", path.display())))?;
    if meta.is_file() {
        if path.extension().is_some_and(|ext| ext == "compact") {
            files.insert(path.to_path_buf());
        } else {
            // Explicitly-named single file: include regardless of extension.
            files.insert(path.to_path_buf());
        }
        return Ok(());
    }

    if meta.is_dir() {
        for entry in fs::read_dir(path).map_err(|err| {
            CliError::io(format!(
                "failed to read directory {}: {err}",
                path.display()
            ))
        })? {
            let entry = entry
                .map_err(|err| CliError::io(format!("failed to read directory entry: {err}")))?;
            collect_dir_entry(&entry.path(), files)?;
        }
        return Ok(());
    }

    Err(CliError::io(format!(
        "unsupported input path {}",
        path.display()
    )))
}

fn collect_dir_entry(path: &Path, files: &mut BTreeSet<PathBuf>) -> Result<(), CliError> {
    let meta = fs::metadata(path)
        .map_err(|err| CliError::io(format!("failed to read {}: {err}", path.display())))?;
    if meta.is_dir() {
        for entry in fs::read_dir(path).map_err(|err| {
            CliError::io(format!(
                "failed to read directory {}: {err}",
                path.display()
            ))
        })? {
            let entry = entry
                .map_err(|err| CliError::io(format!("failed to read directory entry: {err}")))?;
            collect_dir_entry(&entry.path(), files)?;
        }
    } else if meta.is_file() && path.extension().is_some_and(|ext| ext == "compact") {
        files.insert(path.to_path_buf());
    }
    Ok(())
}
