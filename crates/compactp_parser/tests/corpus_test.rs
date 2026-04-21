use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Parse every `.compact` file under `tests/corpus/` and enforce:
///
/// 1. No panic (reaching the end of each iteration proves this).
/// 2. Byte-exact lossless round-trip (`root.text() == source`).
/// 3. The set of files that produce "unexpected" errors (i.e. not under a
///    `negative/` directory) exactly matches the checked-in
///    `tests/corpus_known_failures.txt` manifest.
///
/// If you fix the grammar and a file starts parsing cleanly, remove it from
/// the manifest. If a change introduces a new failure, the test fails with
/// the diff so regressions cannot sneak through. This is what CONSTITUTION §V
/// calls "the corpus is the contract."
#[test]
fn parse_entire_corpus_and_diff_known_failures() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/ dir")
        .parent()
        .expect("workspace root");
    let corpus_dir = workspace_root.join("tests/corpus");
    let manifest_path = workspace_root.join("tests/corpus_known_failures.txt");

    if !corpus_dir.exists() {
        eprintln!(
            "corpus directory not found at {}; skipping",
            corpus_dir.display()
        );
        return;
    }

    let expected = load_manifest(&manifest_path);

    let mut total = 0usize;
    let mut actual: BTreeSet<String> = BTreeSet::new();
    let mut first_errors: Vec<(String, Vec<String>)> = Vec::new();

    for entry in WalkDir::new(&corpus_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file() && e.path().extension().is_some_and(|ext| ext == "compact")
        })
    {
        total += 1;
        let source = match std::fs::read_to_string(entry.path()) {
            Ok(s) => s,
            Err(err) => panic!(
                "failed to read corpus fixture {}: {err}",
                entry.path().display()
            ),
        };
        let result = compactp_parser::parse(&source);

        let root = compactp_syntax::SyntaxNode::new_root(result.green);
        assert_eq!(
            root.kind(),
            compactp_syntax::SyntaxKind::SOURCE_FILE,
            "root node should be SOURCE_FILE for {}",
            entry.path().display()
        );
        assert_eq!(
            root.text().to_string(),
            source,
            "lossless round-trip failed for {}",
            entry.path().display()
        );

        if !result.errors.is_empty() {
            let is_negative = entry
                .path()
                .components()
                .any(|c| c.as_os_str() == "negative");
            if !is_negative {
                let rel = relative_to_corpus(&corpus_dir, entry.path());
                let msgs: Vec<String> = result.errors[..result.errors.len().min(3)]
                    .iter()
                    .map(|d| d.message.clone())
                    .collect();
                first_errors.push((rel.clone(), msgs));
                actual.insert(rel);
            }
        }
    }

    let new_failures: Vec<&String> = actual.difference(&expected).collect();
    let fixed_files: Vec<&String> = expected.difference(&actual).collect();

    if !new_failures.is_empty() || !fixed_files.is_empty() {
        let mut msg = String::new();
        msg.push_str(&format!(
            "corpus drift: parsed {total} files; expected {} known failures, observed {}.\n",
            expected.len(),
            actual.len(),
        ));
        if !new_failures.is_empty() {
            msg.push_str(
                "\nnew failures (fix the grammar or add to tests/corpus_known_failures.txt):\n",
            );
            for path in &new_failures {
                msg.push_str(&format!("  + {path}\n"));
                if let Some((_, errs)) = first_errors.iter().find(|(p, _)| p == *path) {
                    for e in errs {
                        msg.push_str(&format!("      {e}\n"));
                    }
                }
            }
        }
        if !fixed_files.is_empty() {
            msg.push_str(
                "\nnewly-parsing files (remove these from tests/corpus_known_failures.txt):\n",
            );
            for path in &fixed_files {
                msg.push_str(&format!("  - {path}\n"));
            }
        }
        panic!("{msg}");
    }

    eprintln!(
        "corpus ok: {total} files parsed, {} known failures, 0 regressions",
        actual.len()
    );
}

fn load_manifest(path: &Path) -> BTreeSet<String> {
    let text = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(err) => panic!(
            "failed to read known-failures manifest {}: {err}",
            path.display()
        ),
    };
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToOwned::to_owned)
        .collect()
}

fn relative_to_corpus(corpus_dir: &Path, path: &Path) -> String {
    let rel: PathBuf = path
        .strip_prefix(corpus_dir)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| path.to_path_buf());
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}
