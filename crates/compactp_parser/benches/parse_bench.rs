use criterion::{Criterion, criterion_group, criterion_main};

fn bench_lex_small(c: &mut Criterion) {
    let source = "pragma language_version >= 0.22.0;\nexport ledger count: Counter;\nexport circuit increment(): [] { count.increment(1); }\n";
    c.bench_function("lex small", |b| {
        b.iter(|| compactp_lexer::lex(source));
    });
}

fn bench_parse_small(c: &mut Criterion) {
    let source = "pragma language_version >= 0.22.0;\nexport ledger count: Counter;\nexport circuit increment(): [] { count.increment(1); }\n";
    c.bench_function("parse small", |b| {
        b.iter(|| compactp_parser::parse(source));
    });
}

fn bench_parse_medium(c: &mut Criterion) {
    // Build a moderately complex source file
    let mut source = String::from("pragma language_version >= 0.22.0;\n\n");
    for i in 0..20 {
        source.push_str(&format!(
            "export circuit func_{i}(x: Field, y: Field): Field {{\n  const result = x + y * {i} as Field;\n  return result;\n}}\n\n"
        ));
    }
    c.bench_function("parse medium (20 circuits)", |b| {
        b.iter(|| compactp_parser::parse(&source));
    });
}

criterion_group!(
    benches,
    bench_lex_small,
    bench_parse_small,
    bench_parse_medium
);
criterion_main!(benches);
