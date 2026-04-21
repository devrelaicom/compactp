use crate::Cli;
use crate::error::CliError;
use notify_debouncer_full::{DebounceEventResult, new_debouncer, notify::RecursiveMode};
use std::io::IsTerminal;
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;

pub fn run(cli: &Cli, command: &crate::WatchableCommand) -> Result<i32, CliError> {
    let paths = command.paths();
    if paths.is_empty() {
        return Err(CliError::usage("watch requires at least one path"));
    }

    let (tx, rx) = channel::<DebounceEventResult>();
    let mut debouncer = new_debouncer(Duration::from_millis(200), None, tx)
        .map_err(|err| CliError::internal(format!("failed to create watch debouncer: {err}")))?;

    for path in paths {
        debouncer
            .watch(path, RecursiveMode::Recursive)
            .map_err(|err| CliError::io(format!("failed to watch {}: {err}", path.display())))?;
    }

    let interactive = std::io::stdout().is_terminal();

    // Initial run so the user sees output before the first file change.
    if let Err(err) = crate::commands::run_watchable(cli, command) {
        eprintln!("{}", err.message());
    }

    for result in rx {
        match result {
            Ok(events) => {
                let changed = changed_compact_paths(
                    events
                        .iter()
                        .flat_map(|event| event.paths.iter().map(|path| path.as_path())),
                );
                if changed.is_empty() {
                    continue;
                }

                if matches!(cli.format, crate::OutputFormat::Human) && interactive {
                    print!("\x1B[2J\x1B[H");
                }
                if matches!(cli.format, crate::OutputFormat::Human) {
                    println!("changed: {}", changed.join(", "));
                }

                if let Err(err) = crate::commands::run_watchable(cli, command) {
                    eprintln!("{}", err.message());
                }
            }
            Err(errors) => {
                for error in errors {
                    eprintln!("watch error: {error}");
                }
            }
        }
    }

    Ok(0)
}

fn changed_compact_paths<'a>(paths: impl Iterator<Item = &'a Path>) -> Vec<String> {
    let mut changed = paths
        .filter(|path| path.extension().is_some_and(|ext| ext == "compact"))
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    changed.sort();
    changed.dedup();
    changed
}

#[cfg(test)]
mod tests {
    use super::changed_compact_paths;
    use std::path::Path;

    #[test]
    fn filters_non_compact_paths_from_watch_events() {
        let changed = changed_compact_paths(
            [
                Path::new("tests/fixtures/input.compact"),
                Path::new("tests/fixtures/input.compact"),
                Path::new("tests/fixtures/output.json"),
            ]
            .into_iter(),
        );
        assert_eq!(changed, vec!["tests/fixtures/input.compact"]);
    }
}
