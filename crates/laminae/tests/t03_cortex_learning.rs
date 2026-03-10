//! Integration tests for the Cortex learning loop.
//!
//! Tests the full cycle: track edits -> detect patterns -> manage instructions
//! -> export/import -> prompt generation.

mod common;

use laminae::cortex::{Cortex, CortexConfig, LearnedInstruction, PatternType};

// ═══════════════════════════════════════════════════════════
// Track edits and detect patterns
// ═══════════════════════════════════════════════════════════

#[test]
fn track_edits_detects_shortening_pattern() {
    let mut cortex = Cortex::new(CortexConfig {
        min_edits_for_detection: 3,
        min_pattern_frequency: 10.0,
        ..Default::default()
    });

    // User consistently shortens AI output
    for _ in 0..5 {
        cortex.track_edit(
            "It's important to note that this is a very long sentence with many unnecessary words that could be shortened significantly for clarity.",
            "This could be shorter.",
        );
    }

    let patterns = cortex.detect_patterns();
    assert!(!patterns.is_empty(), "Should detect patterns");
    assert!(
        patterns
            .iter()
            .any(|p| p.pattern_type == PatternType::Shortened),
        "Should detect Shortened pattern"
    );
}

#[test]
fn track_edits_detects_removed_questions() {
    let mut cortex = Cortex::new(CortexConfig {
        min_edits_for_detection: 3,
        min_pattern_frequency: 10.0,
        ..Default::default()
    });

    cortex.track_edit("Good point. What do you think?", "Good point.");
    cortex.track_edit(
        "Interesting take. How will this play out?",
        "Interesting take.",
    );
    cortex.track_edit("Strong argument. Don't you agree?", "Strong argument.");

    let patterns = cortex.detect_patterns();
    assert!(
        patterns
            .iter()
            .any(|p| p.pattern_type == PatternType::RemovedQuestion),
        "Should detect RemovedQuestion pattern"
    );
}

#[test]
fn track_edits_detects_removed_ai_phrases() {
    let mut cortex = Cortex::new(CortexConfig {
        min_edits_for_detection: 3,
        min_pattern_frequency: 10.0,
        ..Default::default()
    });

    cortex.track_edit("It's worth noting that Rust is fast.", "Rust is fast.");
    cortex.track_edit("Moving forward, we should use Rust.", "We should use Rust.");
    cortex.track_edit("At the end of the day, types matter.", "Types matter.");

    let patterns = cortex.detect_patterns();
    assert!(
        patterns
            .iter()
            .any(|p| p.pattern_type == PatternType::RemovedAiPhrases),
        "Should detect RemovedAiPhrases pattern"
    );
}

#[test]
fn track_edits_detects_tone_shift_stronger() {
    let mut cortex = Cortex::new(CortexConfig {
        min_edits_for_detection: 3,
        min_pattern_frequency: 10.0,
        ..Default::default()
    });

    cortex.track_edit("This could perhaps work.", "This definitely works.");
    cortex.track_edit("It might be useful.", "It is absolutely essential.");
    cortex.track_edit("Maybe consider this approach.", "Always use this approach.");

    let patterns = cortex.detect_patterns();
    assert!(
        patterns
            .iter()
            .any(|p| p.pattern_type == PatternType::ChangedToneStronger),
        "Should detect ChangedToneStronger pattern"
    );
}

#[test]
fn track_edits_detects_added_content() {
    let mut cortex = Cortex::new(CortexConfig {
        min_edits_for_detection: 2,
        min_pattern_frequency: 10.0,
        ..Default::default()
    });

    cortex.track_edit(
        "Short.",
        "Short. But actually there's way more to say about this topic and here's my full take on why it matters for everyone.",
    );
    cortex.track_edit(
        "Brief.",
        "Brief. However I want to expand significantly on this point because it deserves a thorough explanation.",
    );

    let patterns = cortex.detect_patterns();
    assert!(
        patterns
            .iter()
            .any(|p| p.pattern_type == PatternType::AddedContent),
        "Should detect AddedContent pattern"
    );
}

// ═══════════════════════════════════════════════════════════
// Instruction deduplication
// ═══════════════════════════════════════════════════════════

#[test]
fn instruction_dedup_merges_similar() {
    let _cortex = Cortex::new(CortexConfig::default());

    // Add two near-identical instructions
    let inst1 = LearnedInstruction {
        text: "Never start with I think".into(),
        source_count: 1,
        added: chrono::Utc::now(),
    };
    let inst2 = LearnedInstruction {
        text: "Never start with I think please".into(),
        source_count: 1,
        added: chrono::Utc::now(),
    };

    // Need to rebuild cortex with instructions added
    let cortex = Cortex::new(CortexConfig::default()).with_instructions(vec![inst1, inst2]);

    let all = cortex.export_instructions();
    assert_eq!(all.len(), 1, "Similar instructions should be deduplicated");
    assert_eq!(
        all[0].source_count, 2,
        "Deduped instruction should have combined source_count"
    );
}

#[test]
fn instruction_dedup_keeps_different() {
    let cortex = Cortex::new(CortexConfig::default()).with_instructions(vec![
        LearnedInstruction {
            text: "Never start with I think".into(),
            source_count: 1,
            added: chrono::Utc::now(),
        },
        LearnedInstruction {
            text: "Be more direct and concise".into(),
            source_count: 1,
            added: chrono::Utc::now(),
        },
    ]);

    assert_eq!(
        cortex.export_instructions().len(),
        2,
        "Different instructions should both be kept"
    );
}

// ═══════════════════════════════════════════════════════════
// Export/import roundtrip
// ═══════════════════════════════════════════════════════════

#[test]
fn export_import_roundtrip_preserves_data() {
    let mut cortex = Cortex::new(CortexConfig::default());

    // Track some edits
    cortex.track_edit("Original AI output.", "User's version.");
    cortex.track_edit("Another output.", "Another edit.");

    // Add instructions
    let cortex = cortex.with_instructions(vec![
        LearnedInstruction {
            text: "Be concise".into(),
            source_count: 5,
            added: chrono::Utc::now(),
        },
        LearnedInstruction {
            text: "No trailing questions".into(),
            source_count: 3,
            added: chrono::Utc::now(),
        },
    ]);

    // Export
    let exported = cortex.export_instructions();
    assert_eq!(exported.len(), 2);

    // Serialize and deserialize (simulating persistence)
    let json = serde_json::to_string(&exported).unwrap();
    let imported: Vec<LearnedInstruction> = serde_json::from_str(&json).unwrap();

    // Import into new Cortex
    let cortex2 = Cortex::new(CortexConfig::default()).with_instructions(imported);

    let re_exported = cortex2.export_instructions();
    assert_eq!(re_exported.len(), 2);
    assert!(re_exported.iter().any(|i| i.text == "Be concise"));
    assert!(re_exported
        .iter()
        .any(|i| i.text == "No trailing questions"));

    // Source counts preserved
    let concise = re_exported.iter().find(|i| i.text == "Be concise").unwrap();
    assert_eq!(concise.source_count, 5);
}

// ═══════════════════════════════════════════════════════════
// Stats accuracy
// ═══════════════════════════════════════════════════════════

#[test]
fn stats_reflect_actual_data() {
    let mut cortex = Cortex::new(CortexConfig {
        min_edits_for_detection: 1,
        ..Default::default()
    });

    cortex.track_edit("AI output one", "User edited version one");
    cortex.track_edit("AI output two", "AI output two"); // not edited
    cortex.track_edit("AI output three", "Completely different");
    cortex.track_edit("AI output four", "AI output four"); // not edited

    let stats = cortex.stats();
    assert_eq!(stats.total_edits, 4);
    assert_eq!(stats.edited_count, 2);
    assert_eq!(stats.unedited_count, 2);
    assert!((stats.edit_rate - 0.5).abs() < 0.01);
}

#[test]
fn stats_edit_rate_zero_when_nothing_edited() {
    let mut cortex = Cortex::new(CortexConfig::default());

    cortex.track_edit("Same text", "Same text");
    cortex.track_edit("Also same", "Also same");

    let stats = cortex.stats();
    assert_eq!(stats.edited_count, 0);
    assert_eq!(stats.edit_rate, 0.0);
}

#[test]
fn stats_edit_rate_one_when_all_edited() {
    let mut cortex = Cortex::new(CortexConfig::default());

    cortex.track_edit("Original one", "Edited one");
    cortex.track_edit("Original two", "Edited two");

    let stats = cortex.stats();
    assert_eq!(stats.edited_count, 2);
    assert!((stats.edit_rate - 1.0).abs() < 0.01);
}

#[test]
fn stats_instruction_count_reflects_store() {
    let cortex = Cortex::new(CortexConfig::default()).with_instructions(vec![
        LearnedInstruction {
            text: "Rule one".into(),
            source_count: 1,
            added: chrono::Utc::now(),
        },
        LearnedInstruction {
            text: "Rule two".into(),
            source_count: 1,
            added: chrono::Utc::now(),
        },
        LearnedInstruction {
            text: "Rule three".into(),
            source_count: 1,
            added: chrono::Utc::now(),
        },
    ]);

    let stats = cortex.stats();
    assert_eq!(stats.instruction_count, 3);
}

// ═══════════════════════════════════════════════════════════
// Below-threshold doesn't trigger false positives
// ═══════════════════════════════════════════════════════════

#[test]
fn below_min_edits_threshold_returns_no_patterns() {
    let cortex = Cortex::new(CortexConfig {
        min_edits_for_detection: 10,
        ..Default::default()
    });

    // 0 edits tracked
    assert!(
        cortex.detect_patterns().is_empty(),
        "No edits should mean no patterns"
    );
}

#[test]
fn below_frequency_threshold_filters_patterns() {
    let mut cortex = Cortex::new(CortexConfig {
        min_edits_for_detection: 1,
        min_pattern_frequency: 80.0, // require 80% frequency
        ..Default::default()
    });

    // Cortex filters to edited-only records, then frequency = pattern_count / edited_count.
    // 1 shortened + 4 other edits (not shortened) => 1/5 = 20% < 80% threshold.
    cortex.track_edit(
        "Long verbose text that should be shorter for clarity.",
        "Short.",
    );
    cortex.track_edit("Original A.", "Changed A slightly.");
    cortex.track_edit("Original B.", "Changed B slightly.");
    cortex.track_edit("Original C.", "Changed C slightly.");
    cortex.track_edit("Original D.", "Changed D slightly.");

    let patterns = cortex.detect_patterns();
    // With 80% threshold and only 1 out of 5 edited records being shortened (20%),
    // the shortened pattern should NOT appear
    let shortened = patterns
        .iter()
        .find(|p| p.pattern_type == PatternType::Shortened);
    assert!(
        shortened.is_none(),
        "Pattern below frequency threshold should be filtered out"
    );
}

// ═══════════════════════════════════════════════════════════
// Prompt block generation
// ═══════════════════════════════════════════════════════════

#[test]
fn prompt_block_empty_when_no_instructions() {
    let cortex = Cortex::new(CortexConfig::default());
    assert!(cortex.get_prompt_block().is_empty());
}

#[test]
fn prompt_block_contains_instructions() {
    let cortex = Cortex::new(CortexConfig::default()).with_instructions(vec![
        LearnedInstruction {
            text: "Never start with I think".into(),
            source_count: 5,
            added: chrono::Utc::now(),
        },
        LearnedInstruction {
            text: "Keep under 2 sentences".into(),
            source_count: 3,
            added: chrono::Utc::now(),
        },
    ]);

    let block = cortex.get_prompt_block();
    assert!(block.contains("USER PREFERENCES"));
    assert!(block.contains("Never start with I think"));
    assert!(block.contains("Keep under 2 sentences"));
}

#[test]
fn prompt_block_respects_max_instructions() {
    let mut instructions = Vec::new();
    for i in 0..20 {
        instructions.push(LearnedInstruction {
            text: format!("Instruction number {i}"),
            source_count: 20 - i as u32,
            added: chrono::Utc::now(),
        });
    }

    let cortex = Cortex::new(CortexConfig {
        max_prompt_instructions: 3,
        ..Default::default()
    })
    .with_instructions(instructions);

    let block = cortex.get_prompt_block();
    let instruction_lines: Vec<&str> = block.lines().filter(|l| l.starts_with("- ")).collect();
    assert!(
        instruction_lines.len() <= 3,
        "Should respect max_prompt_instructions limit"
    );
}

// ═══════════════════════════════════════════════════════════
// Edit history management
// ═══════════════════════════════════════════════════════════

#[test]
fn with_edits_loads_history() {
    use laminae::cortex::EditRecord;

    let existing_edits = vec![
        EditRecord::new("AI said this", "User changed it"),
        EditRecord::new("Same thing", "Same thing"),
    ];

    let cortex = Cortex::new(CortexConfig::default()).with_edits(existing_edits);

    assert_eq!(cortex.edits().len(), 2);
    assert!(cortex.edits()[0].was_edited);
    assert!(!cortex.edits()[1].was_edited);
}

// ═══════════════════════════════════════════════════════════
// Max instruction capacity
// ═══════════════════════════════════════════════════════════

#[test]
fn instruction_store_respects_max_size() {
    let mut instructions = Vec::new();
    for i in 0..20 {
        instructions.push(LearnedInstruction {
            text: format!("Unique instruction number {i} with distinct content"),
            source_count: 1,
            added: chrono::Utc::now(),
        });
    }

    let cortex = Cortex::new(CortexConfig {
        max_instructions: 5,
        dedup_threshold: 1.0, // disable dedup so all are treated as unique
        ..Default::default()
    })
    .with_instructions(instructions);

    assert!(
        cortex.export_instructions().len() <= 5,
        "Should cap at max_instructions"
    );
}
