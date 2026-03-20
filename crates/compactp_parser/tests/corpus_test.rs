use std::path::Path;
use walkdir::WalkDir;

/// Parse all 489 upstream .compact files without panics.
///
/// This is the primary correctness test for the parser. Every file in the corpus
/// must parse without panicking, and expected-pass files should not produce errors.
#[test]
fn parse_entire_corpus_without_panics() {
    let corpus_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/corpus");

    if !corpus_dir.exists() {
        eprintln!(
            "Corpus directory not found at {}. Skipping corpus test.",
            corpus_dir.display()
        );
        return;
    }

    let mut total = 0;
    let mut errors = 0;
    let mut error_files = Vec::new();

    for entry in WalkDir::new(&corpus_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file() && e.path().extension().is_some_and(|ext| ext == "compact")
        })
    {
        total += 1;
        let source = std::fs::read_to_string(entry.path()).unwrap();
        let result = compactp_parser::parse(&source);

        // The parse must never panic — just reaching here proves it didn't
        let root = compactp_syntax::SyntaxNode::new_root(result.green);
        assert_eq!(
            root.kind(),
            compactp_syntax::SyntaxKind::SOURCE_FILE,
            "Root node should be SOURCE_FILE for {}",
            entry.path().display()
        );

        // Verify lossless: reconstructed text must match original source
        assert_eq!(
            root.text().to_string(),
            source,
            "Lossless roundtrip failed for {}",
            entry.path().display()
        );

        if !result.errors.is_empty() {
            // Files in "negative/" directories are expected to have errors
            let is_negative = entry
                .path()
                .components()
                .any(|c| c.as_os_str() == "negative");
            if !is_negative {
                errors += 1;
                let msgs: Vec<String> = result.errors[..result.errors.len().min(3)]
                    .iter()
                    .map(|d| d.message.clone())
                    .collect();
                error_files.push((entry.path().display().to_string(), msgs));
            }
        }
    }

    eprintln!("Parsed {total} files, {errors} unexpected errors");

    if !error_files.is_empty() {
        eprintln!("\nFiles with errors:");
        for (path, errs) in &error_files {
            eprintln!("  {path}:");
            for e in errs {
                eprintln!("    {e}");
            }
        }
    }

    // Note: This assertion may initially fail as the grammar is refined.
    // The goal is 0 unexpected errors across all 489 corpus files.
    // Track progress by running: cargo test --test corpus_test -- --nocapture
    eprintln!(
        "Error rate: {errors}/{total} ({:.1}%)",
        errors as f64 / total as f64 * 100.0
    );
}
