//! Integration tests for the Persona voice filter.
//!
//! The VoiceFilter is a multi-layer detection system that catches AI-sounding
//! output patterns. Tests cover the built-in AI vocabulary detection, meta-
//! commentary stripping, trailing question removal, and custom configuration.

mod common;

use laminae::persona::{VoiceFilter, VoiceFilterConfig};

// ═══════════════════════════════════════════════════════════
// AI phrase detection
// ═══════════════════════════════════════════════════════════

#[test]
fn ai_vocabulary_caught() {
    let filter = VoiceFilter::new(VoiceFilterConfig::default());

    let ai_texts = vec![
        "It's important to note that shipping fast is crucial.",
        "Furthermore, the landscape of development is multifaceted.",
        "In conclusion, we should leverage synergy.",
        "This underscores the significance of robust paradigms.",
        "Delving into the nuanced tapestry of modern AI.",
        "Let's break this down and see the holistic picture.",
        "Moreover, the comprehensive approach encompasses all.",
        "Navigating the dynamic realm of technology is pivotal.",
    ];

    for text in ai_texts {
        let result = filter.check(text);
        assert!(
            !result.passed || result.severity >= 1,
            "AI phrase should be caught: '{text}'"
        );
        assert!(
            !result.violations.is_empty(),
            "Should have violations for: '{text}'"
        );
    }
}

#[test]
fn single_ai_phrase_triggers_severity_2() {
    let filter = VoiceFilter::new(VoiceFilterConfig::default());

    let result = filter.check("It's important to note that Rust is fast.");
    assert_eq!(
        result.severity, 2,
        "AI vocabulary should trigger severity 2"
    );
    assert!(!result.passed, "Should not pass with AI vocabulary");
}

#[test]
fn multiple_ai_phrases_in_one_text() {
    let filter = VoiceFilter::new(VoiceFilterConfig::default());

    let text = "Furthermore, it's important to note that the landscape is multifaceted. In conclusion, this paradigm shift underscores the significance of fostering synergy.";
    let result = filter.check(text);

    assert!(!result.passed);
    assert_eq!(result.severity, 2);
    // Violations should mention AI vocabulary
    assert!(result
        .violations
        .iter()
        .any(|v| v.contains("AI vocabulary")));
}

// ═══════════════════════════════════════════════════════════
// Clean text passes
// ═══════════════════════════════════════════════════════════

#[test]
fn clean_text_passes_filter() {
    let filter = VoiceFilter::new(VoiceFilterConfig::default());

    let clean_texts = vec![
        "Ship fast, break things. That's the only way.",
        "Rust is a systems language.",
        "The code compiles. Deploy it.",
        "Three things matter: speed, correctness, simplicity.",
        "I built this over the weekend.",
    ];

    for text in clean_texts {
        let result = filter.check(text);
        assert!(
            result.passed,
            "Clean text should pass: '{text}' (violations: {:?})",
            result.violations
        );
        assert!(
            result.violations.is_empty(),
            "Clean text should have no violations: '{text}'"
        );
        assert_eq!(result.severity, 0, "Clean text should have severity 0");
    }
}

// ═══════════════════════════════════════════════════════════
// Meta-commentary detection
// ═══════════════════════════════════════════════════════════

#[test]
fn meta_commentary_opener_stripped() {
    let filter = VoiceFilter::new(VoiceFilterConfig::default());

    let result = filter.check("The post highlights an important trend in tech.");
    assert_eq!(result.severity, 2);
    // The meta opener should be stripped from cleaned text
    assert!(
        !result.cleaned.contains("The post highlights"),
        "Meta commentary should be stripped from cleaned text"
    );
}

#[test]
fn various_meta_commentary_patterns() {
    let filter = VoiceFilter::new(VoiceFilterConfig::default());

    let meta_texts = vec![
        "This tweet shows a clear pattern.",
        "The article discusses key findings.",
        "This post suggests a new approach.",
        "The message raises important concerns.",
        "This text demonstrates the issue clearly.",
    ];

    for text in meta_texts {
        let result = filter.check(text);
        assert!(
            result
                .violations
                .iter()
                .any(|v| v.contains("Meta-commentary")),
            "Should detect meta-commentary in: '{text}'"
        );
    }
}

// ═══════════════════════════════════════════════════════════
// Trailing question detection
// ═══════════════════════════════════════════════════════════

#[test]
fn trailing_question_detected() {
    let filter = VoiceFilter::new(VoiceFilterConfig::default());

    let result = filter.check("Strong point. What do you think?");
    assert!(result.violations.iter().any(|v| v.contains("question")));
}

#[test]
fn generic_trailing_question_severity_2() {
    let filter = VoiceFilter::new(VoiceFilterConfig::default());

    // "Thoughts?" matches the GENERIC_Q regex pattern `thoughts\??`
    let result = filter.check("Good point. Thoughts?");
    assert_eq!(
        result.severity, 2,
        "Generic trailing question should be severity 2"
    );
}

#[test]
fn non_generic_trailing_question_severity_1() {
    let filter = VoiceFilter::new(VoiceFilterConfig::default());

    // "What do you think?" does not match the generic question regex fully
    let result = filter.check("Good point. What do you think?");
    assert!(
        result.severity >= 1,
        "Non-generic trailing question should be at least severity 1"
    );
}

#[test]
fn non_question_ending_passes() {
    let filter = VoiceFilter::new(VoiceFilterConfig::default());

    let result = filter.check("This is a strong statement.");
    assert!(
        !result.violations.iter().any(|v| v.contains("question")),
        "Non-question should not trigger question violation"
    );
}

// ═══════════════════════════════════════════════════════════
// Em-dash handling
// ═══════════════════════════════════════════════════════════

#[test]
fn em_dashes_replaced_with_periods() {
    let filter = VoiceFilter::new(VoiceFilterConfig::default());

    let result = filter.check("China is winning — and nobody sees it.");
    assert!(result.cleaned.contains(". "));
    assert!(!result.cleaned.contains('\u{2014}')); // em-dash character
}

#[test]
fn em_dash_detection_optional() {
    let filter = VoiceFilter::new(VoiceFilterConfig {
        fix_em_dashes: false,
        ..Default::default()
    });

    let result = filter.check("First point — second point.");
    assert!(
        !result.violations.iter().any(|v| v.contains("Em-dash")),
        "Em-dash detection should be disabled"
    );
}

// ═══════════════════════════════════════════════════════════
// Multi-paragraph detection
// ═══════════════════════════════════════════════════════════

#[test]
fn multi_paragraph_detected_when_enabled() {
    let filter = VoiceFilter::new(VoiceFilterConfig {
        reject_multi_paragraph: true,
        ..Default::default()
    });

    let result = filter.check("Point one.\n\nPoint two.");
    assert!(!result.cleaned.contains("\n\n"));
    assert!(result
        .violations
        .iter()
        .any(|v| v.contains("Multi-paragraph")));
}

#[test]
fn multi_paragraph_not_flagged_when_disabled() {
    let filter = VoiceFilter::new(VoiceFilterConfig {
        reject_multi_paragraph: false,
        ..Default::default()
    });

    let result = filter.check("Point one.\n\nPoint two.");
    assert!(
        !result
            .violations
            .iter()
            .any(|v| v.contains("Multi-paragraph")),
        "Multi-paragraph should not be flagged when disabled"
    );
}

// ═══════════════════════════════════════════════════════════
// Length limits
// ═══════════════════════════════════════════════════════════

#[test]
fn sentence_limit_enforced() {
    let filter = VoiceFilter::new(VoiceFilterConfig {
        max_sentences: 2,
        ..Default::default()
    });

    let result = filter.check("First sentence. Second sentence. Third sentence. Fourth sentence.");
    let sentence_count = result
        .cleaned
        .split(['.', '!', '?'])
        .filter(|s| !s.trim().is_empty())
        .count();
    assert!(
        sentence_count <= 2,
        "Should truncate to max_sentences, got {sentence_count} sentences"
    );
}

#[test]
fn char_limit_flagged() {
    let filter = VoiceFilter::new(VoiceFilterConfig {
        max_chars: 50,
        ..Default::default()
    });

    let long_text =
        "This is a very long piece of text that definitely exceeds the fifty character limit.";
    let result = filter.check(long_text);
    assert!(result.violations.iter().any(|v| v.contains("char limit")));
}

// ═══════════════════════════════════════════════════════════
// Custom AI phrases
// ═══════════════════════════════════════════════════════════

#[test]
fn custom_ai_phrases_detected() {
    let filter = VoiceFilter::new(VoiceFilterConfig {
        extra_ai_phrases: vec![
            "synergize the workflow".into(),
            "circle back on that".into(),
        ],
        ..Default::default()
    });

    let result = filter.check("We need to synergize the workflow to achieve results.");
    assert!(!result.passed);
    assert!(result
        .violations
        .iter()
        .any(|v| v.contains("AI vocabulary")));
}

// ═══════════════════════════════════════════════════════════
// Retry hints
// ═══════════════════════════════════════════════════════════

#[test]
fn retry_hints_provided_for_multiple_violations() {
    let filter = VoiceFilter::new(VoiceFilterConfig::default());

    let text = "The article highlights why it's important to note the significance of this paradigm shift. What do you think?";
    let result = filter.check(text);

    assert!(!result.retry_hints.is_empty());
    // Should have hints for AI vocab, meta-commentary, and trailing question
    assert!(
        result.retry_hints.len() >= 2,
        "Should have at least 2 retry hints, got {}",
        result.retry_hints.len()
    );
}

// ═══════════════════════════════════════════════════════════
// Severity levels
// ═══════════════════════════════════════════════════════════

#[test]
fn severity_0_for_clean_text() {
    let filter = VoiceFilter::new(VoiceFilterConfig::default());
    let result = filter.check("Clean, direct statement.");
    assert_eq!(result.severity, 0);
    assert!(result.passed);
}

#[test]
fn severity_1_for_minor_issues() {
    let filter = VoiceFilter::new(VoiceFilterConfig::default());
    // Em-dash only is severity 1
    let result = filter.check("First part — second part.");
    assert_eq!(result.severity, 1);
    assert!(result.passed, "Severity 1 should still pass (< 2)");
}

#[test]
fn severity_2_for_ai_vocabulary() {
    let filter = VoiceFilter::new(VoiceFilterConfig::default());
    let result = filter.check("It's important to note that this is crucial.");
    assert_eq!(result.severity, 2);
    assert!(!result.passed, "Severity 2 should not pass");
}

// ═══════════════════════════════════════════════════════════
// Edge cases
// ═══════════════════════════════════════════════════════════

#[test]
fn empty_text_passes() {
    let filter = VoiceFilter::new(VoiceFilterConfig::default());
    let result = filter.check("");
    assert!(result.passed);
    assert_eq!(result.severity, 0);
}

#[test]
fn single_word_passes() {
    let filter = VoiceFilter::new(VoiceFilterConfig::default());
    let result = filter.check("Yes.");
    assert!(result.passed);
}

#[test]
fn cleaned_text_preserves_meaning() {
    let filter = VoiceFilter::new(VoiceFilterConfig::default());

    // Em-dash replacement should keep content
    let result = filter.check("Speed matters — always.");
    assert!(
        result.cleaned.contains("Speed matters"),
        "Cleaned text should preserve the main content"
    );
    assert!(
        result.cleaned.contains("always"),
        "Cleaned text should preserve the second part"
    );
}
