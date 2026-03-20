use std::path::{Path, PathBuf};

/// Resolved input source for parsing.
pub enum InputSource {
    File(PathBuf),
    Stdin { filename: Option<String> },
}

/// Resolve CLI path arguments into a deterministic list of .compact files.
///
/// - Single files are included as-is
/// - Directories are walked recursively, collecting *.compact files in alphabetical order
/// - "-" is treated as stdin
pub fn resolve_inputs(
    paths: &[PathBuf],
    stdin_filename: Option<&str>,
) -> Result<Vec<InputSource>, std::io::Error> {
    let mut inputs = Vec::new();

    if paths.is_empty() || (paths.len() == 1 && paths[0].as_os_str() == "-") {
        inputs.push(InputSource::Stdin {
            filename: stdin_filename.map(String::from),
        });
        return Ok(inputs);
    }

    for path in paths {
        if path.as_os_str() == "-" {
            inputs.push(InputSource::Stdin {
                filename: stdin_filename.map(String::from),
            });
        } else if path.is_dir() {
            let mut files: Vec<PathBuf> = Vec::new();
            collect_compact_files(path, &mut files)?;
            files.sort();
            for f in files {
                inputs.push(InputSource::File(f));
            }
        } else {
            inputs.push(InputSource::File(path.clone()));
        }
    }

    Ok(inputs)
}

fn collect_compact_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), std::io::Error> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_compact_files(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "compact") {
            out.push(path);
        }
    }
    Ok(())
}
