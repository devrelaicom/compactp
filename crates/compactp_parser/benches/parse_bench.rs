use criterion::{Criterion, criterion_group, criterion_main};

fn bench_lex_small(c: &mut Criterion) {
    let source = "pragma language_version >= 0.23.0;\nexport ledger count: Counter;\nexport circuit increment(): [] { count.increment(1); }\n";
    c.bench_function("lex small", |b| {
        b.iter(|| compactp_lexer::lex(source));
    });
}

fn bench_parse_small(c: &mut Criterion) {
    let source = "pragma language_version >= 0.23.0;\nexport ledger count: Counter;\nexport circuit increment(): [] { count.increment(1); }\n";
    c.bench_function("parse small", |b| {
        b.iter(|| compactp_parser::parse(source));
    });
}

fn bench_parse_medium(c: &mut Criterion) {
    // Build a moderately complex source file
    let mut source = String::from("pragma language_version >= 0.23.0;\n\n");
    for i in 0..20 {
        source.push_str(&format!(
            "export circuit func_{i}(x: Field, y: Field): Field {{\n  const result = x + y * {i} as Field;\n  return result;\n}}\n\n"
        ));
    }
    c.bench_function("parse medium (20 circuits)", |b| {
        b.iter(|| compactp_parser::parse(&source));
    });
}

/// Left-associative operator chains and postfix chains grow a *left-deep
/// spine*: every extension wraps the whole tree built so far. Tree
/// construction was O(n²) in that length until issue #22, and no
/// benchmark covered the shape, which is why it shipped.
///
/// Two lengths a factor of four apart, so the ratio between them reads
/// the asymptotics straight off the output: linear input scales the
/// time by 4, quadratic by 16.
///
/// `max_depth` is raised deliberately. At the default the depth charge
/// truncates the spine after a few hundred extensions, so a chain
/// benchmark run with `parse()` measures the cap and reports a flat
/// line no matter how tree construction behaves.
///
/// Unlike `tests/chain_scaling.rs`, which leaks its trees, these are
/// dropped every iteration — and rowan drops recursively, so the longer
/// case unwinds ~4,000 levels. `ParseOptions::max_depth` puts a release
/// frame at roughly 650 bytes, so that is ~2.6 MiB against criterion's
/// 8 MiB main thread. Comfortable, but it is the reason these lengths
/// are not raised further.
fn bench_parse_chain(c: &mut Criterion) {
    const PRE: &str =
        "pragma language_version >= 0.23.0;\nexport pure circuit f(x: Field): Field { return ";
    const POST: &str = "; }\n";

    fn opts() -> compactp_parser::ParseOptions {
        compactp_parser::ParseOptions {
            max_depth: 100_000,
            ..Default::default()
        }
    }

    for terms in [1_000usize, 4_000] {
        let mut body = String::from("x");
        for _ in 1..terms {
            body.push_str("+x");
        }
        let source = format!("{PRE}{body}{POST}");
        c.bench_function(&format!("parse infix chain ({terms} terms)"), |b| {
            b.iter(|| compactp_parser::parse_with(&source, opts()));
        });

        let mut body = String::from("x");
        for _ in 1..terms {
            body.push_str(".a");
        }
        let source = format!("{PRE}{body}{POST}");
        c.bench_function(&format!("parse member chain ({terms} links)"), |b| {
            b.iter(|| compactp_parser::parse_with(&source, opts()));
        });
    }
}

criterion_group!(
    benches,
    bench_lex_small,
    bench_parse_small,
    bench_parse_medium,
    bench_parse_chain
);
criterion_main!(benches);
