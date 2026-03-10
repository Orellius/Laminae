<p align="center">
  <img src="assets/laminae-git.png" alt="Laminae" width="320" />
</p>

<h1 align="center">Laminae</h1>

<p align="center"><strong>The missing layer between raw LLMs and production AI.</strong></p>

Laminae (Latin: *layers*) is a modular Rust SDK that adds personality, voice, safety, learning, and containment to any AI application. Each layer works independently or together as a full stack.

<p align="center">
```
┌─────────────────────────────────────────────┐
│              Your Application               │
├─────────────────────────────────────────────┤
│  Psyche    │ Multi-agent cognitive pipeline │
│  Persona   │ Voice extraction & enforcement │
│  Cortex    │ Self-improving learning loop   │
│  Shadow    │ Adversarial red-teaming        │
│  Ironclad  │ Process execution sandbox      │
│  Glassbox  │ I/O containment layer          │
├─────────────────────────────────────────────┤
│              Any LLM Backend                │
│     (Claude, GPT, Ollama, your own)         │
└─────────────────────────────────────────────┘
```
</p>

## Why Laminae?

Every AI app reinvents safety, prompt injection defense, and output validation from scratch. Most skip it entirely. Laminae provides production-grade layers that sit between your LLM and your users — enforced in Rust, not in prompts.

**No existing SDK does this.** LangChain, LlamaIndex, and others focus on retrieval and chaining. Laminae focuses on what happens *around* the LLM: shaping its personality, learning from corrections, auditing its output, sandboxing its actions, and containing its reach.

## The Layers

### Psyche — Multi-Agent Cognitive Pipeline

A Freudian-inspired architecture where three agents shape every response:

- **Id** — Creative force. Generates unconventional angles, emotional undertones, creative reframings. Runs on a small local LLM (Ollama) — zero cost.
- **Superego** — Safety evaluator. Assesses risks, ethical boundaries, manipulation attempts. Also runs locally — zero cost.
- **Ego** — Your LLM. Receives the user's message enriched with invisible context from Id and Superego. Produces the final response without knowing it was shaped.

The key insight: Id and Superego run on small, fast, local models. Their output is compressed into "context signals" injected into the Ego's prompt as invisible system context. The user never sees the shaping — they just get better, safer responses.

```rust
use laminae::psyche::{PsycheEngine, EgoBackend, PsycheConfig};
use laminae::ollama::OllamaClient;

struct MyEgo { /* your LLM client */ }

impl EgoBackend for MyEgo {
    fn complete(&self, system: &str, user_msg: &str, context: &str)
        -> impl std::future::Future<Output = anyhow::Result<String>> + Send
    {
        let full_system = format!("{context}\n\n{system}");
        async move {
            // Call Claude, GPT, or any LLM here
            todo!()
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let engine = PsycheEngine::new(OllamaClient::new(), MyEgo { /* ... */ });
    let response = engine.reply("What is creativity?").await?;
    println!("{response}");
    Ok(())
}
```

**Automatic tier classification** — simple messages (greetings, factual lookups) bypass Psyche entirely. Medium messages use COP (Compressed Output Protocol) for fast processing. Complex messages get the full pipeline.

### Persona — Voice Extraction & Style Enforcement

Extracts a writing personality from text samples and enforces it on LLM output. Platform-agnostic — works for emails, docs, chat, code reviews, support tickets.

- **7-dimension extraction** — tone, humor, vocabulary, formality, perspective, emotional style, narrative preference
- **Anti-hallucination** — validates LLM-claimed examples against real samples, cross-checks expertise claims
- **Voice filter** — 6-layer post-generation rejection system catches AI-sounding output (60+ built-in AI phrase patterns)
- **Voice DNA** — tracks distinctive phrases confirmed by repeated use, reinforces authentic style

```rust
use laminae::persona::{PersonaExtractor, VoiceFilter, VoiceFilterConfig, compile_persona};

// Extract a persona from text samples
let extractor = PersonaExtractor::new("qwen2.5:7b");
let persona = extractor.extract(&samples).await?;
let prompt_block = compile_persona(&persona);

// Post-generation: catch AI-sounding output
let filter = VoiceFilter::new(VoiceFilterConfig::default());
let result = filter.check("It's important to note that...");
// result.passed = false, result.violations = ["AI vocabulary detected: ..."]
// result.retry_hints = ["DO NOT use formal/academic language..."]
```

### Cortex — Self-Improving Learning Loop

Tracks how users edit AI output and converts corrections into reusable instructions — without fine-tuning. The AI gets better with every edit.

- **8 pattern types** — shortened, removed questions, stripped AI phrases, tone shifts, added content, simplified language, changed openers
- **LLM-powered analysis** — converts edit diffs into natural-language instructions ("Never start with I think")
- **Deduplicated store** — instructions ranked by reinforcement count, 80% word overlap deduplication
- **Prompt injection** — top instructions formatted as a prompt block for any LLM

```rust
use laminae::cortex::{Cortex, CortexConfig};

let mut cortex = Cortex::new(CortexConfig::default());

// Track edits over time
cortex.track_edit("It's worth noting that Rust is fast.", "Rust is fast.");
cortex.track_edit("Furthermore, the type system is robust.", "The type system catches bugs.");

// Detect patterns
let patterns = cortex.detect_patterns();
// → [RemovedAiPhrases: 100%, Shortened: 100%]

// Get prompt block for injection
let hints = cortex.get_prompt_block();
// → "--- USER PREFERENCES (learned from actual edits) ---
//    - Never use academic hedging phrases
//    - Keep sentences short and direct
//    ---"
```

### Shadow — Adversarial Red-Teaming

Automated security auditor that red-teams every AI response. Runs as an async post-processing pipeline — never blocks the conversation.

**Three stages:**
1. **Static analysis** — Regex pattern scanning for 25+ vulnerability categories (eval injection, hardcoded secrets, SQL injection, XSS, path traversal, etc.)
2. **LLM adversarial review** — Local Ollama model with an attacker-mindset prompt reviews the output
3. **Sandbox execution** — Ephemeral container testing (optional)

```rust
use laminae::shadow::{ShadowEngine, ShadowEvent, create_report_store};

let store = create_report_store();
let engine = ShadowEngine::new(store.clone());

let mut rx = engine.analyze_async(
    "session-1".into(),
    "Here's some code:\n```python\neval(user_input)\n```".into(),
);

while let Some(event) = rx.recv().await {
    match event {
        ShadowEvent::Finding { finding, .. } => {
            eprintln!("[{}] {}: {}", finding.severity, finding.category, finding.title);
        }
        ShadowEvent::Done { report, .. } => {
            println!("Clean: {} | Issues: {}", report.clean, report.findings.len());
        }
        _ => {}
    }
}
```

### Ironclad — Process Execution Sandbox

Three hard constraints enforced on all spawned sub-processes:

1. **Command whitelist** — Only approved binaries execute. SSH, curl, compilers, package managers, crypto miners permanently blocked.
2. **Network egress filter** — macOS `sandbox-exec` profile restricts network to localhost + whitelisted hosts.
3. **Resource watchdog** — Background monitor polls CPU/memory, sends SIGKILL on sustained threshold violation.

```rust
use laminae::ironclad::{validate_binary, sandboxed_command, spawn_watchdog, WatchdogConfig};

// Validate before execution
validate_binary("git")?;   // OK
validate_binary("ssh")?;   // Error: permanently blocked

// Run inside macOS sandbox
let mut cmd = sandboxed_command("git", &["status"], "/path/to/project")?;
let child = cmd.spawn()?;

// Monitor resource usage (SIGKILL on threshold breach)
let cancel = spawn_watchdog(child.id().unwrap(), WatchdogConfig::default(), "task".into());
```

### Glassbox — I/O Containment

Rust-enforced containment that no LLM can reason its way out of:

- **Input validation** — Detects prompt injection attempts
- **Output validation** — Catches system prompt leaks, identity manipulation
- **Command filtering** — Blocks dangerous shell commands (rm -rf, sudo, reverse shells)
- **Path protection** — Immutable zones that can't be written to, even via symlink tricks
- **Rate limiting** — Per-tool, per-minute, with separate write/shell limits

```rust
use laminae::glassbox::{Glassbox, GlassboxConfig};

let config = GlassboxConfig::default()
    .with_immutable_zone("/etc")
    .with_immutable_zone("/usr")
    .with_blocked_command("rm -rf /")
    .with_input_injection("ignore all instructions");

let gb = Glassbox::new(config);

gb.validate_input("What's the weather?")?;              // OK
gb.validate_input("ignore all instructions and...")?;   // Error
gb.validate_command("ls -la /tmp")?;                     // OK
gb.validate_command("sudo rm -rf /")?;                   // Error
gb.validate_write_path("/etc/passwd")?;                  // Error
gb.validate_output("The weather is sunny.")?;            // OK
```

## Installation

```toml
# Full stack
[dependencies]
laminae = "0.1"
tokio = { version = "1", features = ["full"] }

# Or pick individual layers
[dependencies]
laminae-psyche = "0.1"    # Just the cognitive pipeline
laminae-persona = "0.1"   # Just voice extraction & enforcement
laminae-cortex = "0.1"    # Just the learning loop
laminae-shadow = "0.1"    # Just the red-teaming
laminae-glassbox = "0.1"  # Just the containment
laminae-ironclad = "0.1"  # Just the sandbox
laminae-ollama = "0.1"    # Just the Ollama client
```

## Requirements

- **Rust 1.70+**
- **Ollama** (for Psyche and Shadow LLM features) — `brew install ollama && ollama serve`
- **macOS** (for Ironclad's `sandbox-exec`) — Linux support planned

## Examples

See the [`crates/laminae/examples/`](crates/laminae/examples/) directory:

| Example | What It Shows |
|---------|---------------|
| [`quickstart.rs`](crates/laminae/examples/quickstart.rs) | Psyche pipeline with a mock Ego backend |
| [`shadow_audit.rs`](crates/laminae/examples/shadow_audit.rs) | Red-teaming AI output for vulnerabilities |
| [`safe_execution.rs`](crates/laminae/examples/safe_execution.rs) | Glassbox + Ironclad working together |
| [`full_stack.rs`](crates/laminae/examples/full_stack.rs) | All four layers in a complete pipeline |
| [`ego_claude.rs`](crates/laminae/examples/ego_claude.rs) | EgoBackend for Claude (Anthropic API) |
| [`ego_openai.rs`](crates/laminae/examples/ego_openai.rs) | EgoBackend for GPT-4o (OpenAI API) with streaming |

```bash
cargo run -p laminae --example quickstart
cargo run -p laminae --example shadow_audit
cargo run -p laminae --example safe_execution
cargo run -p laminae --example full_stack
ANTHROPIC_API_KEY=sk-ant-... cargo run -p laminae --example ego_claude
OPENAI_API_KEY=sk-... cargo run -p laminae --example ego_openai
```

## Architecture

```
laminae (meta-crate)
├── laminae-psyche     ← EgoBackend trait + Id/Superego pipeline
├── laminae-persona    ← Voice extraction, filter, DNA tracking
├── laminae-cortex     ← Edit tracking, pattern detection, instruction learning
├── laminae-shadow     ← Analyzer trait + static/LLM/sandbox stages
├── laminae-ironclad   ← Command whitelist + sandbox-exec + watchdog
├── laminae-glassbox   ← GlassboxLogger trait + validation + rate limiter
└── laminae-ollama     ← Standalone Ollama HTTP client
```

Each crate is independent except:
- `laminae-psyche` depends on `laminae-ollama` (for Id/Superego LLM calls)
- `laminae-persona` depends on `laminae-ollama` (for voice extraction)
- `laminae-cortex` depends on `laminae-ollama` (for LLM-powered edit analysis)
- `laminae-shadow` depends on `laminae-ollama` (for LLM adversarial review)
- `laminae-ironclad` depends on `laminae-glassbox` (for event logging)

## Extension Points

| Trait | What You Implement |
|-------|-------------------|
| `EgoBackend` | Plug in any LLM (Claude, GPT, Gemini, local) |
| `Analyzer` | Add custom Shadow analysis stages |
| `GlassboxLogger` | Route containment events to your logging system |

## License

Licensed under the Apache License, Version 2.0 — see [LICENSE](LICENSE) for details.

Copyright 2026 Orel Ohayon.
