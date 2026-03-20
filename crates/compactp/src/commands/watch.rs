use notify_debouncer_full::{DebouncedEvent, new_debouncer};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

pub fn run(
    paths: &[PathBuf],
    run_fn: impl Fn(&[PathBuf]),
) -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = mpsc::channel();
    let mut debouncer = new_debouncer(Duration::from_millis(200), None, tx)?;

    for path in paths {
        let watch_path = if path.as_os_str() == "-" {
            continue;
        } else {
            path.clone()
        };
        debouncer.watch(&watch_path, notify::RecursiveMode::Recursive)?;
    }

    // Initial run
    eprintln!("Watching {} path(s) for changes...", paths.len());
    run_fn(paths);

    // Watch loop
    loop {
        match rx.recv() {
            Ok(Ok(events)) => {
                let changed: Vec<&PathBuf> = events
                    .iter()
                    .flat_map(|e: &DebouncedEvent| e.paths.iter())
                    .filter(|p| p.extension().is_some_and(|ext| ext == "compact"))
                    .collect();

                if !changed.is_empty() {
                    eprintln!("\n--- File changed, re-running... ---\n");
                    run_fn(paths);
                }
            }
            Ok(Err(errors)) => {
                for e in errors {
                    eprintln!("Watch error: {e}");
                }
            }
            Err(e) => {
                eprintln!("Channel error: {e}");
                break;
            }
        }
    }

    Ok(())
}
