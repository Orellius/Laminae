use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use laminae_cortex::{Cortex, CortexConfig, EditRecord, LearnedInstruction};

// ---------------------------------------------------------------------------
// Test data
// ---------------------------------------------------------------------------

/// Generate a batch of realistic edit records where the user shortens AI output.
fn shortened_edits(count: usize) -> Vec<EditRecord> {
    let pairs = [
        (
            "It's important to note that Rust provides memory safety without garbage collection, which is a significant advantage in systems programming.",
            "Rust provides memory safety without garbage collection.",
        ),
        (
            "Furthermore, it should be noted that the borrow checker is one of Rust's most distinctive features and it helps prevent data races at compile time.",
            "The borrow checker prevents data races at compile time.",
        ),
        (
            "In conclusion, one could argue that Rust's type system is quite comprehensive and enables developers to write safer code overall.",
            "Rust's type system enables safer code.",
        ),
        (
            "Moving forward, it's worth noting that async/await in Rust has matured significantly and is now production-ready for most use cases.",
            "Async/await in Rust is production-ready.",
        ),
        (
            "At the end of the day, the reality is that Rust's ecosystem is growing rapidly and the community is incredibly supportive of newcomers.",
            "Rust's ecosystem is growing fast.",
        ),
    ];

    (0..count)
        .map(|i| {
            let (orig, edited) = pairs[i % pairs.len()];
            EditRecord::new(orig, edited)
        })
        .collect()
}

/// Generate mixed edits (some shortened, some expanded, some tone-shifted).
fn mixed_edits(count: usize) -> Vec<EditRecord> {
    let pairs = [
        // Shortened
        (
            "It's important to note that testing is crucial for software quality.",
            "Testing matters.",
        ),
        // Expanded
        (
            "Tests help.",
            "Tests help catch regressions early and give confidence when refactoring complex codebases.",
        ),
        // Tone stronger
        (
            "Maybe consider using integration tests.",
            "Always use integration tests.",
        ),
        // Removed question
        (
            "Good approach. What do you think?",
            "Good approach.",
        ),
        // AI phrase removed
        (
            "It's worth noting that benchmarks should be reproducible.",
            "Benchmarks should be reproducible.",
        ),
    ];

    (0..count)
        .map(|i| {
            let (orig, edited) = pairs[i % pairs.len()];
            EditRecord::new(orig, edited)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

fn bench_track_edit(c: &mut Criterion) {
    let mut group = c.benchmark_group("track_edit");

    group.bench_function("single", |b| {
        let mut cortex = Cortex::new(CortexConfig::default());
        b.iter(|| {
            cortex.track_edit(
                black_box("It's important to note that Rust is fast."),
                black_box("Rust is fast."),
            )
        })
    });

    group.bench_function("throughput_100", |b| {
        b.iter(|| {
            let mut cortex = Cortex::new(CortexConfig::default());
            for _ in 0..100 {
                cortex.track_edit(
                    black_box("AI generated verbose output here."),
                    black_box("Concise version."),
                );
            }
        })
    });

    group.finish();
}

fn bench_detect_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("detect_patterns");

    for count in [10, 50, 100, 500] {
        let edits = shortened_edits(count);
        let cortex = Cortex::new(CortexConfig {
            min_edits_for_detection: 2,
            min_pattern_frequency: 10.0,
            ..Default::default()
        })
        .with_edits(edits);

        group.bench_with_input(BenchmarkId::new("shortened", count), &count, |b, _| {
            b.iter(|| cortex.detect_patterns())
        });
    }

    for count in [10, 50, 100, 500] {
        let edits = mixed_edits(count);
        let cortex = Cortex::new(CortexConfig {
            min_edits_for_detection: 2,
            min_pattern_frequency: 10.0,
            ..Default::default()
        })
        .with_edits(edits);

        group.bench_with_input(BenchmarkId::new("mixed", count), &count, |b, _| {
            b.iter(|| cortex.detect_patterns())
        });
    }

    group.finish();
}

fn bench_get_prompt_block(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_prompt_block");

    // Empty store
    group.bench_function("empty", |b| {
        let cortex = Cortex::new(CortexConfig::default());
        b.iter(|| cortex.get_prompt_block())
    });

    // With instructions
    let instructions: Vec<LearnedInstruction> = (0..8)
        .map(|i| LearnedInstruction {
            text: format!("Learned instruction number {i}: keep output concise and direct"),
            source_count: (8 - i) as u32,
            added: chrono::Utc::now(),
        })
        .collect();

    group.bench_function("with_8_instructions", |b| {
        let cortex = Cortex::new(CortexConfig::default()).with_instructions(instructions.clone());
        b.iter(|| cortex.get_prompt_block())
    });

    // With many instructions (needs ranking/truncation)
    let many_instructions: Vec<LearnedInstruction> = (0..50)
        .map(|i| LearnedInstruction {
            text: format!("Instruction {i}: some learned preference about writing style"),
            source_count: (50 - i) as u32,
            added: chrono::Utc::now(),
        })
        .collect();

    group.bench_function("with_50_instructions", |b| {
        let cortex =
            Cortex::new(CortexConfig::default()).with_instructions(many_instructions.clone());
        b.iter(|| cortex.get_prompt_block())
    });

    group.finish();
}

fn bench_stats(c: &mut Criterion) {
    let mut group = c.benchmark_group("stats");

    for count in [10, 100, 500] {
        let edits = mixed_edits(count);
        let cortex = Cortex::new(CortexConfig {
            min_edits_for_detection: 2,
            min_pattern_frequency: 10.0,
            ..Default::default()
        })
        .with_edits(edits);

        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            b.iter(|| cortex.stats())
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_track_edit,
    bench_detect_patterns,
    bench_get_prompt_block,
    bench_stats,
);
criterion_main!(benches);
