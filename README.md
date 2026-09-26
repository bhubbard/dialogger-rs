# dialogger-rs

[![Deploy GitHub Pages](https://github.com/bhubbard/dialogger-rs/actions/workflows/pages.yml/badge.svg)](https://github.com/bhubbard/dialogger-rs/actions/workflows/pages.yml)
[![GitHub Pages](https://img.shields.io/badge/Live_Demo-GitHub_Pages-blue?style=flat&logo=github)](https://code.brandonhubbard.com/dialogger-rs/)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Rust: Edition 2024](https://img.shields.io/badge/Rust-2024_Edition-orange?logo=rust)](https://www.rust-lang.org/)

A pure Rust implementation of branching dialogue graph schema, traversal runtime, boolean/numeric expression evaluator, and contextual NPC bark engine based on Evan Todd's **Dialogger**.

🎮 **[Live Interactive Dialogue Player & Graph Visualizer](https://code.brandonhubbard.com/dialogger-rs/)** (or [GitHub Pages mirror](https://bhubbard.github.io/dialogger-rs/))

---

## Features

- **Dialogue Graph Schema (`src/schema.rs`)**:
  - `SpeechNode`: Speaker name, text content, voice asset ID, character expression.
  - `ChoiceNode`: Selectable choices with condition gating filters.
  - `ConditionNode`: Boolean/numeric expressions evaluated against game variables (`then_node` / `else_node`).
  - `SetVariableNode`: Variable mutation (`assign`, `add`, `subtract`, `toggle`).
  - `EventNode`: Game-world triggers (give items, quests, audio triggers).
- **Runtime Traversal Engine (`src/runtime.rs`)**:
  - `DialogueRunner`: Manages current node, history tracing, and stepping through dialogue turns.
  - Automatic resolution of intermediate logic gates (`SetVariable`, `Condition`).
  - Dynamic Variable Store: `Value::Bool`, `Value::Int`, `Value::Float`, `Value::String`.
  - Built-in recursive-descent expression evaluator supporting comparisons (`==`, `!=`, `<`, `>`, `<=`, `>=`), arithmetic (`+`, `-`, `*`, `/`), and boolean logic (`&&`, `||`, `!`, `AND`, `OR`, `NOT`).
- **Contextual NPC Bark Engine (`src/bark.rs`)**:
  - Ambient reaction barks: `CombatAlert`, `Fleeing`, `Insult`, `IdleChatter`, `CrimeWitness`, `Custom`.
  - Multi-tier priority scoring, cooldown timers per line and per category.
  - Weighted random selection with consecutive repeat avoidance.
- **Dialogger JSON Interop (`src/graph.rs`)**:
  - Direct import from Evan Todd's classic Dialogger JSON format.
  - Full modern JSON serialization / deserialization.
  - Validation checks: detects broken links and orphan nodes.

---

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
dialogger-rs = { git = "https://github.com/bhubbard/dialogger-rs" }
```

### 1. Constructing a Dialogue Graph

```rust
use dialogger_rs::{DialogueGraph, GraphBuilder, MutationOp, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let graph = GraphBuilder::new()
        .title("City Gate Confrontation")
        .speech("intro", "Captain Vane", "Halt! State your business.", Some("choices"))
        .choice(
            "choices",
            Some("How do you respond?"),
            vec![
                dialogger_rs::ChoiceOption {
                    id: "bribe".into(),
                    text: "Here is 50 gold. (-50 Gold)".into(),
                    target: "pay_bribe".into(),
                    condition: Some("gold >= 50".into()),
                },
                dialogger_rs::ChoiceOption {
                    id: "seal".into(),
                    text: "I carry the Royal Seal.".into(),
                    target: "check_seal".into(),
                    condition: Some("has_seal == true".into()),
                },
                dialogger_rs::ChoiceOption {
                    id: "threaten".into(),
                    text: "Stand aside or draw steel!".into(),
                    target: "combat".into(),
                    condition: None,
                },
            ],
        )
        .set_variable("pay_bribe", "gold", MutationOp::Subtract, Value::Int(50), Some("pass"))
        .speech("pass", "Captain Vane", "Proceed, citizen.", None)
        .speech("check_seal", "Captain Vane", "My apologies, Envoy! Enter.", None)
        .event("combat", "aggro_guards", None, Some("attack_line"))
        .speech("attack_line", "Captain Vane", "Treason! Cut them down!", None)
        .build()?;

    Ok(())
}
```

### 2. Stepping through Dialogue

```rust
use dialogger_rs::{DialogueRunner, DialogueStep, Value};

let mut runner = DialogueRunner::new(&graph);
runner.variables.set("gold", 100);
runner.variables.set("has_seal", false);

loop {
    match runner.step()? {
        DialogueStep::Speech { speech, .. } => {
            println!("{}: {}", speech.speaker, speech.text);
        }
        DialogueStep::Choice { prompt, options, .. } => {
            if let Some(p) = prompt { println!("{}", p); }
            for (i, opt) in options.iter().enumerate() {
                let status = if opt.available { "✓" } else { "🔒" };
                println!("[{}] {} {}", i, status, opt.text);
            }
            // Select choice by ID or index
            runner.select_choice_by_id("bribe")?;
        }
        DialogueStep::Event { event, .. } => {
            println!(">> Event fired: {}", event.event_name);
        }
        DialogueStep::End => {
            println!("Conversation complete.");
            break;
        }
    }
}
```

### 3. Ambient NPC Bark Engine

```rust
use dialogger_rs::{Bark, BarkCategory, BarkManager, VariableStore};

let mut barks = BarkManager::new();

barks.add_bark(Bark {
    id: "guard_alert".into(),
    speaker: "City Guard".into(),
    category: BarkCategory::CombatAlert,
    text: "Intruder! To arms!".into(),
    voice_asset: Some("snd/guard_alert.ogg".into()),
    priority: 20,
    weight: 1.0,
    cooldown: 8.0,
    condition: None,
});

let vars = VariableStore::new();
let current_time = 0.0;

if let Some(bark) = barks.trigger_bark(&BarkCategory::CombatAlert, &vars, current_time) {
    println!("{}: {}", bark.speaker, bark.text);
}
```

---

## Classic Dialogger JSON Compatibility

`dialogger-rs` parses files saved from Evan Todd's original open-source Dialogger tool:

```rust
let json_str = r#"[
    {
        "id": 1,
        "character": "Guard",
        "text": "Who goes there?",
        "choices": [
            { "text": "Friend.", "node": 2 },
            { "text": "Foe.", "node": 3 }
        ]
    },
    { "id": 2, "character": "Guard", "text": "Pass, friend.", "next": null },
    { "id": 3, "character": "Guard", "text": "Draw your weapon!", "next": null }
]"#;

let graph = DialogueGraph::from_dialogger_json(json_str)?;
```

---

## Interactive Demo

The included web visualizer in `docs/index.html` showcases:
- Real-time graph node highlighting as dialogue advances.
- Variable inspector with dynamic state changes.
- Branching player choices with live condition validation.
- NPC bark simulator with cooldown bars, priority sorting, and event logs.

Access it online at: **[https://bhubbard.github.io/dialogger-rs/](https://bhubbard.github.io/dialogger-rs/)**

---

## License

Dual-licensed under either:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
