//! Dialogue graph schema and node models.

use serde::{Deserialize, Serialize};

/// Unique identifier for a node in a dialogue graph.
pub type NodeId = String;

/// Dynamic value representation for variable store and event payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    /// Boolean value.
    Bool(bool),
    /// Integer value (64-bit).
    Int(i64),
    /// Floating point value (64-bit).
    Float(f64),
    /// String value.
    String(String),
}

impl Value {
    /// Return true if the value is truthy.
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0 && !f.is_nan(),
            Value::String(s) => !s.is_empty() && s != "false" && s != "0",
        }
    }

    /// Try to get value as integer.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            Value::Float(f) => Some(*f as i64),
            Value::Bool(b) => Some(if *b { 1 } else { 0 }),
            Value::String(s) => s.parse().ok(),
        }
    }

    /// Try to get value as float.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Int(i) => Some(*i as f64),
            Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            Value::String(s) => s.parse().ok(),
        }
    }

    /// Try to get value as boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            Value::Int(i) => Some(*i != 0),
            Value::Float(f) => Some(*f != 0.0),
            Value::String(s) => match s.to_lowercase().as_str() {
                "true" | "1" | "yes" => Some(true),
                "false" | "0" | "no" => Some(false),
                _ => None,
            },
        }
    }

    /// Get value as string representation.
    pub fn as_str_repr(&self) -> String {
        match self {
            Value::Bool(b) => b.to_string(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::String(s) => s.clone(),
        }
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Int(v)
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::Int(v as i64)
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Float(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::String(v.to_string())
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::String(v)
    }
}

/// A dialogue speech line delivered by a character.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpeechNode {
    /// Name of the speaking character.
    pub speaker: String,
    /// Dialogue text content.
    pub text: String,
    /// Optional voice asset audio identifier or path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice_asset: Option<String>,
    /// Optional character facial/emotional expression (e.g. "angry", "happy", "smirk").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
    /// Next node ID in the graph. None indicates dialogue conclusion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<NodeId>,
}

/// A single choice option within a ChoiceNode.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChoiceOption {
    /// Unique identifier for this choice option.
    pub id: String,
    /// Player-facing text displayed for this choice.
    pub text: String,
    /// Destination node ID triggered when this choice is selected.
    pub target: NodeId,
    /// Optional condition expression required for this option to be available/visible.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
}

/// A branching decision point with one or more selectable choices.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChoiceNode {
    /// Optional prompt or question introducing the choices.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// List of available choices.
    pub choices: Vec<ChoiceOption>,
}

/// A logic gate that routes traversal based on a variable condition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConditionNode {
    /// Boolean or comparative expression evaluated against variable store.
    pub condition: String,
    /// Target node ID if the condition evaluates to true.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub then_node: Option<NodeId>,
    /// Target node ID if the condition evaluates to false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub else_node: Option<NodeId>,
}

/// Operations for mutating variables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationOp {
    /// Assign a new value directly: `var = val`.
    Assign,
    /// Add numeric value: `var += val`.
    Add,
    /// Subtract numeric value: `var -= val`.
    Subtract,
    /// Invert boolean value: `var = !var`.
    Toggle,
}

/// A node that modifies the game variable store during traversal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetVariableNode {
    /// Target variable name.
    pub variable: String,
    /// Mutation operator.
    pub op: MutationOp,
    /// Value operand.
    #[serde(default = "default_set_var_value")]
    pub value: Value,
    /// Next node ID after variable assignment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<NodeId>,
}

fn default_set_var_value() -> Value {
    Value::Bool(true)
}

/// A node that triggers an external game event or audio/quest cue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventNode {
    /// Event name or trigger identifier.
    pub event_name: String,
    /// Optional parameter or payload dictionary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
    /// Next node ID following the event trigger.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<NodeId>,
}

/// Enumeration of all node variants supported in a dialogue graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DialogueNode {
    /// Character speech dialogue line.
    Speech(SpeechNode),
    /// Branching player choices.
    Choice(ChoiceNode),
    /// Conditional branch.
    Condition(ConditionNode),
    /// State / variable mutation.
    SetVariable(SetVariableNode),
    /// Game world event trigger.
    Event(EventNode),
}

impl DialogueNode {
    /// Returns the type name of this node.
    pub fn type_name(&self) -> &'static str {
        match self {
            DialogueNode::Speech(_) => "speech",
            DialogueNode::Choice(_) => "choice",
            DialogueNode::Condition(_) => "condition",
            DialogueNode::SetVariable(_) => "set_variable",
            DialogueNode::Event(_) => "event",
        }
    }

    /// Returns all outgoing node target IDs referenced by this node.
    pub fn outgoing_targets(&self) -> Vec<NodeId> {
        match self {
            DialogueNode::Speech(s) => s.next.clone().into_iter().collect(),
            DialogueNode::Choice(c) => c.choices.iter().map(|ch| ch.target.clone()).collect(),
            DialogueNode::Condition(c) => {
                let mut v = Vec::new();
                if let Some(ref t) = c.then_node {
                    v.push(t.clone());
                }
                if let Some(ref e) = c.else_node {
                    v.push(e.clone());
                }
                v
            }
            DialogueNode::SetVariable(s) => s.next.clone().into_iter().collect(),
            DialogueNode::Event(e) => e.next.clone().into_iter().collect(),
        }
    }
}
