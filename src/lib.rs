//! # dialogger-rs
//!
//! A pure Rust implementation of branching dialogue graph schema, traversal runtime,
//! expression evaluator, and NPC bark engine based on Evan Todd's Dialogger.
//!
//! ## Features
//!
//! - **Dialogue Graph Schema**: `SpeechNode`, `ChoiceNode`, `ConditionNode`, `SetVariableNode`, and `EventNode`.
//! - **Runtime Traversal**: `DialogueRunner` with history tracking, step execution, and branching choices.
//! - **Variable Store & Expressions**: Type-safe dynamic variables (`Bool`, `Int`, `Float`, `String`) and condition evaluator (`==`, `!=`, `<`, `>`, `<=`, `>=`, `&&`, `||`, `!`).
//! - **Dialogger Compatibility**: Parser for Evan Todd's classic Dialogger JSON format as well as modern serialized schema.
//! - **Contextual NPC Bark Engine**: Ambient bark lines (`CombatAlert`, `Fleeing`, `Insult`, `IdleChatter`, `CrimeWitness`), priority gating, cooldown management, and repeat-avoidance weighted selection.

pub mod bark;
pub mod graph;
pub mod runtime;
pub mod schema;

// Re-export primary types for convenience
pub use bark::{Bark, BarkCategory, BarkManager};
pub use graph::{DialogueGraph, GraphBuilder, GraphError};
pub use runtime::{
    eval_expression, DialogueRunner, DialogueStep, EvaluatedChoice, RuntimeError, VariableStore,
};
pub use schema::{
    ChoiceNode, ChoiceOption, ConditionNode, DialogueNode, EventNode, MutationOp, NodeId,
    SetVariableNode, SpeechNode, Value,
};
