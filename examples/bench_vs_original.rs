//! Benchmark comparing `dialogger-rs` (Rust) vs original Evan Todd Dialogger / Yarn Spinner (C#/JS).

use dialogger_rs::{
    eval_expression, Bark, BarkCategory, BarkManager, ChoiceOption, DialogueRunner, DialogueStep,
    GraphBuilder, MutationOp, Value, VariableStore,
};
use std::time::Instant;

fn main() {
    println!("============================================================");
    println!("   dialogger-rs (Rust) vs Dialogger / Yarn Spinner (C#/JS)  ");
    println!("============================================================");

    // 1. Dialogue Graph Traversal (Speech -> Mutation -> Condition -> Choice)
    println!("\n--- 1. Dialogue Graph Step & Traversal Engine ---");
    {
        let graph = GraphBuilder::new()
            .speech("node_1", "NPC", "Greetings traveler.", Some("node_2"))
            .set_variable("node_2", "visited_count", MutationOp::Add, Value::Int(1), Some("node_3"))
            .condition("node_3", "gold >= 50", Some("node_rich"), Some("node_poor"))
            .speech("node_rich", "NPC", "I see you have coin.", Some("node_choice"))
            .speech("node_poor", "NPC", "You look destitute.", Some("node_choice"))
            .choice(
                "node_choice",
                Some("What do you want?"),
                vec![
                    ChoiceOption {
                        id: "opt_buy".into(),
                        text: "I want to buy goods.".into(),
                        target: "node_end".into(),
                        condition: Some("gold >= 50".into()),
                    },
                    ChoiceOption {
                        id: "opt_quest".into(),
                        text: "Got any work?".into(),
                        target: "node_end".into(),
                        condition: None,
                    },
                ],
            )
            .speech("node_end", "NPC", "Farewell.", None)
            .build()
            .unwrap();

        let iterations = 200_000;
        let start = Instant::now();
        let mut total_steps = 0;

        for i in 0..iterations {
            let mut runner = DialogueRunner::new(&graph);
            runner.variables.set("gold", if i % 2 == 0 { 100 } else { 20 });
            runner.variables.set("visited_count", 0);

            // Traverse until completion
            while let Ok(step) = runner.step() {
                total_steps += 1;
                match step {
                    DialogueStep::Choice { options, .. } => {
                        // Choose first available option
                        if let Some((idx, _)) = options.iter().enumerate().find(|(_, o)| o.available) {
                            let _ = runner.select_choice_by_index(idx);
                        } else {
                            break;
                        }
                    }
                    DialogueStep::End => break,
                    _ => {}
                }
            }
        }

        std::hint::black_box(total_steps);
        let elapsed = start.elapsed();
        let ns_per_story = elapsed.as_nanos() as f64 / iterations as f64;
        let stories_per_sec = iterations as f64 / elapsed.as_secs_f64();
        let ns_per_step = elapsed.as_nanos() as f64 / total_steps as f64;

        println!(
            "Stories Traversed: {} | Total Steps: {} | Time: {:.2?} | Latency: {:.2} ns/step ({:.2} ns/story, {:>8.0} stories/s) | {:>10.0} steps/s",
            iterations, total_steps, elapsed, ns_per_step, ns_per_story, stories_per_sec, total_steps as f64 / elapsed.as_secs_f64()
        );
    }

    // 2. Dynamic Expression Evaluator Throughput
    println!("\n--- 2. Type-Safe Dynamic Expression Evaluator ---");
    {
        let mut vars = VariableStore::new();
        vars.set("gold", 150);
        vars.set("has_amulet", true);
        vars.set("reputation", 4.5);
        vars.set("faction", "guild");

        let exprs = [
            "gold >= 100",
            "has_amulet && gold > 100",
            "!has_amulet || gold == 150",
            "reputation > 4.0 && (gold == 150 || gold == 0)",
            "faction == 'guild'",
        ];

        let iterations = 1_000_000;
        let start = Instant::now();
        let mut true_evals = 0;

        for i in 0..iterations {
            let expr = exprs[i % exprs.len()];
            if eval_expression(expr, &vars).unwrap_or(false) {
                true_evals += 1;
            }
        }

        std::hint::black_box(true_evals);
        let elapsed = start.elapsed();
        let ns_per_eval = elapsed.as_nanos() as f64 / iterations as f64;
        let evals_per_sec = iterations as f64 / elapsed.as_secs_f64();

        println!(
            "Expressions Evaluated: {} | Time: {:.2?} | Latency: {:.2} ns/eval | {:>10.0} evals/s | Matches: {}",
            iterations, elapsed, ns_per_eval, evals_per_sec, true_evals
        );
    }

    // 3. NPC Contextual Bark Manager (Priority + Cooldown Filtering)
    println!("\n--- 3. NPC Contextual Bark Selection Engine ---");
    {
        let mut manager = BarkManager::new();
        manager.seed(1337);

        // Populate with 100 combat, insult, and ambient chatter barks
        for i in 0..100 {
            manager.add_bark(Bark {
                id: format!("bark_{}", i),
                speaker: format!("NPC_{}", i % 5),
                category: if i < 40 {
                    BarkCategory::CombatAlert
                } else if i < 70 {
                    BarkCategory::Insult
                } else {
                    BarkCategory::IdleChatter
                },
                text: format!("Bark line {} for category", i),
                voice_asset: None,
                priority: (i % 25) as u32,
                weight: 1.0 + (i % 3) as f64,
                cooldown: 5.0 + (i % 5) as f64,
                condition: if i % 4 == 0 {
                    Some("alert_level > 2".into())
                } else {
                    None
                },
            });
        }

        let mut vars = VariableStore::new();
        vars.set("alert_level", 3);

        let iterations = 500_000;
        let start = Instant::now();
        let mut triggered_count = 0;

        for i in 0..iterations {
            let time = (i as f64) * 0.05;
            let cat = match i % 3 {
                0 => BarkCategory::CombatAlert,
                1 => BarkCategory::Insult,
                _ => BarkCategory::IdleChatter,
            };

            if let Some(_bark) = manager.trigger_bark(&cat, &vars, time) {
                triggered_count += 1;
            }
        }

        std::hint::black_box(triggered_count);
        let elapsed = start.elapsed();
        let ns_per_bark = elapsed.as_nanos() as f64 / iterations as f64;
        let barks_per_sec = iterations as f64 / elapsed.as_secs_f64();

        println!(
            "Bark Triage Evals: {} (100 barks) | Time: {:.2?} | Latency: {:.2} ns/eval | {:>10.0} evals/s | Triggered: {}",
            iterations, elapsed, ns_per_bark, barks_per_sec, triggered_count
        );
    }

    println!("\n============================================================");
    println!("                      Benchmark Complete                    ");
    println!("============================================================");
}
