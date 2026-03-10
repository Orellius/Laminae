use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use laminae_persona::{VoiceFilter, VoiceFilterConfig};

// ---------------------------------------------------------------------------
// Test data
// ---------------------------------------------------------------------------

fn clean_text(len: usize) -> String {
    let base = "Ship fast, break things. Fix it in prod. Nobody reads your README anyway. ";
    base.repeat((len / base.len()) + 1)[..len].to_string()
}

fn ai_heavy_text(len: usize) -> String {
    let base = "It's important to note that the landscape of AI is multifaceted. Furthermore, one could argue that leveraging these paradigms fosters a more robust and comprehensive approach. Moreover, navigating this dynamic realm requires a nuanced understanding. ";
    base.repeat((len / base.len()) + 1)[..len].to_string()
}

fn meta_commentary_text() -> &'static str {
    "The post highlights an important trend in the technology landscape that underscores the significance of AI safety."
}

fn trailing_question_text() -> &'static str {
    "Strong take on Rust adoption. The ecosystem is growing fast. What do you think?"
}

fn em_dash_text() -> &'static str {
    "China is winning the AI race \u{2014} and nobody sees it. The implications are vast \u{2014} affecting every sector \u{2014} from defense to education."
}

fn multi_paragraph_text() -> &'static str {
    "First paragraph with important points about the topic.\n\nSecond paragraph that expands on the idea with more details.\n\nThird paragraph wrapping things up."
}

// ---------------------------------------------------------------------------
// VoiceFilter.check() benchmarks
// ---------------------------------------------------------------------------

fn bench_voice_filter_clean(c: &mut Criterion) {
    let filter = VoiceFilter::new(VoiceFilterConfig::default());

    let mut group = c.benchmark_group("voice_filter/clean");
    for size in [100, 500, 1_000, 5_000] {
        let text = clean_text(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &text, |b, text| {
            b.iter(|| filter.check(black_box(text)))
        });
    }
    group.finish();
}

fn bench_voice_filter_ai_heavy(c: &mut Criterion) {
    let filter = VoiceFilter::new(VoiceFilterConfig::default());

    let mut group = c.benchmark_group("voice_filter/ai_heavy");
    for size in [100, 500, 1_000, 5_000] {
        let text = ai_heavy_text(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &text, |b, text| {
            b.iter(|| filter.check(black_box(text)))
        });
    }
    group.finish();
}

fn bench_voice_filter_specific_violations(c: &mut Criterion) {
    let filter = VoiceFilter::new(VoiceFilterConfig {
        reject_trailing_questions: true,
        fix_em_dashes: true,
        reject_multi_paragraph: true,
        ..Default::default()
    });

    let mut group = c.benchmark_group("voice_filter/violations");

    group.bench_function("meta_commentary", |b| {
        b.iter(|| filter.check(black_box(meta_commentary_text())))
    });

    group.bench_function("trailing_question", |b| {
        b.iter(|| filter.check(black_box(trailing_question_text())))
    });

    group.bench_function("em_dashes", |b| {
        b.iter(|| filter.check(black_box(em_dash_text())))
    });

    group.bench_function("multi_paragraph", |b| {
        b.iter(|| filter.check(black_box(multi_paragraph_text())))
    });

    group.finish();
}

fn bench_voice_filter_with_limits(c: &mut Criterion) {
    let mut group = c.benchmark_group("voice_filter/with_limits");

    let filter = VoiceFilter::new(VoiceFilterConfig {
        max_sentences: 2,
        max_chars: 280,
        ..Default::default()
    });

    group.bench_function("under_limit", |b| {
        b.iter(|| filter.check(black_box("Short and sweet.")))
    });

    group.bench_function("over_sentence_limit", |b| {
        b.iter(|| {
            filter.check(black_box(
                "First sentence. Second sentence. Third sentence. Fourth sentence. Fifth sentence.",
            ))
        })
    });

    group.bench_function("over_char_limit", |b| {
        let long = clean_text(500);
        b.iter(|| filter.check(black_box(&long)))
    });

    group.finish();
}

fn bench_voice_filter_custom_phrases(c: &mut Criterion) {
    let mut group = c.benchmark_group("voice_filter/custom_phrases");

    let extra: Vec<String> = (0..60)
        .map(|i| format!("custom ai phrase number {i}"))
        .collect();

    let filter = VoiceFilter::new(VoiceFilterConfig {
        extra_ai_phrases: extra,
        ..Default::default()
    });

    group.bench_function("clean_with_60_extra", |b| {
        let text = clean_text(500);
        b.iter(|| filter.check(black_box(&text)))
    });

    group.bench_function("match_extra_phrase", |b| {
        b.iter(|| {
            filter.check(black_box(
                "This contains custom ai phrase number 42 somewhere in it.",
            ))
        })
    });

    group.finish();
}

fn bench_voice_filter_full_pipeline(c: &mut Criterion) {
    let filter = VoiceFilter::new(VoiceFilterConfig {
        reject_trailing_questions: true,
        fix_em_dashes: true,
        reject_multi_paragraph: true,
        max_sentences: 3,
        max_chars: 500,
        ..Default::default()
    });

    let mut group = c.benchmark_group("voice_filter/full_pipeline");

    // Worst case: triggers every layer
    let worst_case = "The post highlights why it\u{2019}s important to note that the landscape is multifaceted \u{2014} and furthermore, this paradigm shift underscores a robust approach.\n\nMoreover, navigating these dynamics requires nuanced understanding. What do you think?";

    group.bench_function("worst_case_all_layers", |b| {
        b.iter(|| filter.check(black_box(worst_case)))
    });

    // Best case: passes all layers immediately
    group.bench_function("best_case_clean", |b| {
        b.iter(|| filter.check(black_box("Ship it. Fix it later.")))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_voice_filter_clean,
    bench_voice_filter_ai_heavy,
    bench_voice_filter_specific_violations,
    bench_voice_filter_with_limits,
    bench_voice_filter_custom_phrases,
    bench_voice_filter_full_pipeline,
);
criterion_main!(benches);
