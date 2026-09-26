//! Runtime execution engine, variable store, and expression evaluation for dialogue graphs.

use std::collections::HashMap;
use thiserror::Error;

use crate::schema::{DialogueNode, EventNode, MutationOp, NodeId, SpeechNode, Value};
use crate::graph::DialogueGraph;

/// Errors that can occur during runtime traversal or expression evaluation.
#[derive(Debug, Error, PartialEq)]
pub enum RuntimeError {
    /// Referenced node does not exist in graph.
    #[error("Node not found: {0}")]
    NodeNotFound(NodeId),

    /// Choice index out of bounds or choice option ID invalid.
    #[error("Invalid choice selection: {0}")]
    InvalidChoice(String),

    /// Graph runner is already at the end of conversation.
    #[error("Conversation has ended")]
    ConversationEnded,

    /// Syntax or semantic error while evaluating condition expression.
    #[error("Expression evaluation error: {0}")]
    ExpressionError(String),

    /// Type mismatch during variable mutation or comparison.
    #[error("Type error: {0}")]
    TypeError(String),
}

/// Dynamic game variable store.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VariableStore {
    variables: HashMap<String, Value>,
}

use serde::{Deserialize, Serialize};

impl VariableStore {
    /// Create a new empty variable store.
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    /// Retrieve reference to a variable value.
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.variables.get(name)
    }

    /// Set a variable value.
    pub fn set<T: Into<Value>>(&mut self, name: impl Into<String>, val: T) {
        self.variables.insert(name.into(), val.into());
    }

    /// Check if a variable exists.
    pub fn contains(&self, name: &str) -> bool {
        self.variables.contains_key(name)
    }

    /// Remove a variable.
    pub fn remove(&mut self, name: &str) -> Option<Value> {
        self.variables.remove(name)
    }

    /// Get all variables as a map.
    pub fn all(&self) -> &HashMap<String, Value> {
        &self.variables
    }

    /// Mutate a variable according to a MutationOp.
    pub fn mutate(&mut self, name: &str, op: MutationOp, operand: &Value) -> Result<(), RuntimeError> {
        match op {
            MutationOp::Assign => {
                self.variables.insert(name.to_string(), operand.clone());
                Ok(())
            }
            MutationOp::Toggle => {
                let current = self.variables.get(name).map(|v| v.is_truthy()).unwrap_or(false);
                self.variables.insert(name.to_string(), Value::Bool(!current));
                Ok(())
            }
            MutationOp::Add => {
                let current = self.variables.get(name).cloned().unwrap_or(Value::Int(0));
                let new_val = match (current, operand) {
                    (Value::Int(a), Value::Int(b)) => Value::Int(a + b),
                    (Value::Float(a), Value::Float(b)) => Value::Float(a + b),
                    (Value::Int(a), Value::Float(b)) => Value::Float(a as f64 + b),
                    (Value::Float(a), Value::Int(b)) => Value::Float(a + *b as f64),
                    (Value::String(a), Value::String(b)) => Value::String(format!("{a}{b}")),
                    (Value::String(a), other) => Value::String(format!("{a}{}", other.as_str_repr())),
                    _ => {
                        return Err(RuntimeError::TypeError(format!(
                            "Cannot add {operand:?} to variable '{name}'"
                        )))
                    }
                };
                self.variables.insert(name.to_string(), new_val);
                Ok(())
            }
            MutationOp::Subtract => {
                let current = self.variables.get(name).cloned().unwrap_or(Value::Int(0));
                let new_val = match (current, operand) {
                    (Value::Int(a), Value::Int(b)) => Value::Int(a - b),
                    (Value::Float(a), Value::Float(b)) => Value::Float(a - b),
                    (Value::Int(a), Value::Float(b)) => Value::Float(a as f64 - b),
                    (Value::Float(a), Value::Int(b)) => Value::Float(a - *b as f64),
                    _ => {
                        return Err(RuntimeError::TypeError(format!(
                            "Cannot subtract {operand:?} from variable '{name}'"
                        )))
                    }
                };
                self.variables.insert(name.to_string(), new_val);
                Ok(())
            }
        }
    }
}

/// A choice option presented to the player during runtime.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluatedChoice {
    /// Original option ID.
    pub id: String,
    /// Player display text.
    pub text: String,
    /// Destination node ID.
    pub target: NodeId,
    /// True if the choice meets condition filter (or has no condition).
    pub available: bool,
}

/// Current state yield from advancing the `DialogueRunner`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DialogueStep {
    /// Active character speech line.
    Speech {
        node_id: NodeId,
        speech: SpeechNode,
    },
    /// Player choice selection point.
    Choice {
        node_id: NodeId,
        prompt: Option<String>,
        options: Vec<EvaluatedChoice>,
    },
    /// Event triggered in the game world.
    Event {
        node_id: NodeId,
        event: EventNode,
    },
    /// Conversation reached an end node.
    End,
}

/// Traversal runner for executing a dialogue graph.
#[derive(Debug, Clone)]
pub struct DialogueRunner<'a> {
    graph: &'a DialogueGraph,
    pub variables: VariableStore,
    current_node: Option<NodeId>,
    history: Vec<NodeId>,
    waiting_for_choice: bool,
    active_choices: Vec<EvaluatedChoice>,
}

impl<'a> DialogueRunner<'a> {
    /// Create a new runner starting at the graph's designated start node.
    pub fn new(graph: &'a DialogueGraph) -> Self {
        Self {
            graph,
            variables: VariableStore::new(),
            current_node: Some(graph.start_node().to_string()),
            history: Vec::new(),
            waiting_for_choice: false,
            active_choices: Vec::new(),
        }
    }

    /// Create a runner with an existing variable store.
    pub fn with_variables(graph: &'a DialogueGraph, variables: VariableStore) -> Self {
        Self {
            graph,
            variables,
            current_node: Some(graph.start_node().to_string()),
            history: Vec::new(),
            waiting_for_choice: false,
            active_choices: Vec::new(),
        }
    }

    /// Set current node directly (e.g. jumping to a checkpoint).
    pub fn set_current_node(&mut self, node_id: Option<NodeId>) {
        self.current_node = node_id;
        self.waiting_for_choice = false;
        self.active_choices.clear();
    }

    /// Get ID of the current node.
    pub fn current_node_id(&self) -> Option<&str> {
        self.current_node.as_deref()
    }

    /// Get list of visited node IDs in order.
    pub fn history(&self) -> &[NodeId] {
        &self.history
    }

    /// Check if currently awaiting a player choice.
    pub fn is_waiting_for_choice(&self) -> bool {
        self.waiting_for_choice
    }

    /// Get active choices currently available.
    pub fn active_choices(&self) -> &[EvaluatedChoice] {
        &self.active_choices
    }

    /// Select a choice by zero-based index among active options.
    pub fn select_choice_by_index(&mut self, index: usize) -> Result<(), RuntimeError> {
        if !self.waiting_for_choice {
            return Err(RuntimeError::InvalidChoice("Runner is not awaiting a choice".into()));
        }
        let choice = self
            .active_choices
            .get(index)
            .ok_or_else(|| RuntimeError::InvalidChoice(format!("Choice index {index} out of bounds")))?;

        if !choice.available {
            return Err(RuntimeError::InvalidChoice(format!(
                "Choice '{0}' is locked/unavailable",
                choice.text
            )));
        }

        let target = choice.target.clone();
        self.waiting_for_choice = false;
        self.active_choices.clear();
        self.current_node = Some(target);
        Ok(())
    }

    /// Select a choice by option ID.
    pub fn select_choice_by_id(&mut self, choice_id: &str) -> Result<(), RuntimeError> {
        let index = self
            .active_choices
            .iter()
            .position(|c| c.id == choice_id)
            .ok_or_else(|| RuntimeError::InvalidChoice(format!("Choice id '{choice_id}' not found")))?;
        self.select_choice_by_index(index)
    }

    /// Advance traversal to the next presentation step.
    /// Internal logic nodes (`SetVariable`, `Condition`) are resolved automatically.
    pub fn step(&mut self) -> Result<DialogueStep, RuntimeError> {
        if self.waiting_for_choice {
            return Err(RuntimeError::InvalidChoice(
                "Must select a choice before calling step()".into(),
            ));
        }

        loop {
            let node_id = match &self.current_node {
                Some(id) => id.clone(),
                None => return Ok(DialogueStep::End),
            };

            let node = self
                .graph
                .get_node(&node_id)
                .ok_or_else(|| RuntimeError::NodeNotFound(node_id.clone()))?;

            self.history.push(node_id.clone());

            match node {
                DialogueNode::Speech(speech) => {
                    self.current_node = speech.next.clone();
                    return Ok(DialogueStep::Speech {
                        node_id,
                        speech: speech.clone(),
                    });
                }
                DialogueNode::Event(event) => {
                    self.current_node = event.next.clone();
                    return Ok(DialogueStep::Event {
                        node_id,
                        event: event.clone(),
                    });
                }
                DialogueNode::Choice(choice) => {
                    let mut evaluated = Vec::new();
                    for opt in &choice.choices {
                        let available = if let Some(ref cond) = opt.condition {
                            eval_expression(cond, &self.variables).unwrap_or(false)
                        } else {
                            true
                        };
                        evaluated.push(EvaluatedChoice {
                            id: opt.id.clone(),
                            text: opt.text.clone(),
                            target: opt.target.clone(),
                            available,
                        });
                    }
                    self.waiting_for_choice = true;
                    self.active_choices = evaluated.clone();
                    return Ok(DialogueStep::Choice {
                        node_id,
                        prompt: choice.prompt.clone(),
                        options: evaluated,
                    });
                }
                DialogueNode::SetVariable(set_var) => {
                    self.variables.mutate(&set_var.variable, set_var.op, &set_var.value)?;
                    self.current_node = set_var.next.clone();
                    // Loop continues automatically to next node
                }
                DialogueNode::Condition(cond_node) => {
                    let cond_result = eval_expression(&cond_node.condition, &self.variables)?;
                    let next = if cond_result {
                        cond_node.then_node.clone()
                    } else {
                        cond_node.else_node.clone()
                    };
                    self.current_node = next;
                    // Loop continues automatically to next node
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Expression Evaluator
// ---------------------------------------------------------------------------

/// Evaluate a boolean expression string against the variable store.
pub fn eval_expression(expr: &str, vars: &VariableStore) -> Result<bool, RuntimeError> {
    let expr = expr.trim();
    if expr.is_empty() {
        return Ok(true);
    }
    let mut tokens = tokenize(expr)?;
    let val = parse_or_expr(&mut tokens, vars)?;
    Ok(val.is_truthy())
}

#[derive(Debug, PartialEq, Clone)]
enum Token {
    Ident(String),
    Number(f64),
    Str(String),
    Bool(bool),
    Op(String),
    LParen,
    RParen,
}

fn tokenize(input: &str) -> Result<Vec<Token>, RuntimeError> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }

        if c == '(' {
            tokens.push(Token::LParen);
            i += 1;
            continue;
        }
        if c == ')' {
            tokens.push(Token::RParen);
            i += 1;
            continue;
        }

        // Two-char operators
        if i + 1 < chars.len() {
            let two: String = chars[i..=i + 1].iter().collect();
            if matches!(two.as_str(), "==" | "!=" | "<=" | ">=" | "&&" | "||") {
                tokens.push(Token::Op(two));
                i += 2;
                continue;
            }
        }

        // Single-char operators
        if matches!(c, '<' | '>' | '!' | '+' | '-' | '*' | '/') {
            tokens.push(Token::Op(c.to_string()));
            i += 1;
            continue;
        }

        // Strings
        if c == '"' || c == '\'' {
            let quote = c;
            i += 1;
            let mut s = String::new();
            while i < chars.len() && chars[i] != quote {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 1;
                    s.push(chars[i]);
                } else {
                    s.push(chars[i]);
                }
                i += 1;
            }
            if i >= chars.len() {
                return Err(RuntimeError::ExpressionError("Unterminated string literal".into()));
            }
            i += 1; // skip closing quote
            tokens.push(Token::Str(s));
            continue;
        }

        // Numbers
        if c.is_ascii_digit() || (c == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit()) {
            let mut num_str = String::new();
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                num_str.push(chars[i]);
                i += 1;
            }
            let num: f64 = num_str
                .parse()
                .map_err(|e| RuntimeError::ExpressionError(format!("Invalid number '{num_str}': {e}")))?;
            tokens.push(Token::Number(num));
            continue;
        }

        // Identifiers / keywords
        if c.is_alphabetic() || c == '_' {
            let mut ident = String::new();
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '.') {
                ident.push(chars[i]);
                i += 1;
            }

            match ident.to_lowercase().as_str() {
                "true" => tokens.push(Token::Bool(true)),
                "false" => tokens.push(Token::Bool(false)),
                "and" => tokens.push(Token::Op("&&".into())),
                "or" => tokens.push(Token::Op("||".into())),
                "not" => tokens.push(Token::Op("!".into())),
                _ => tokens.push(Token::Ident(ident)),
            }
            continue;
        }

        return Err(RuntimeError::ExpressionError(format!(
            "Unexpected character in expression: '{c}'"
        )));
    }

    Ok(tokens)
}

fn parse_or_expr(tokens: &mut Vec<Token>, vars: &VariableStore) -> Result<Value, RuntimeError> {
    let mut left = parse_and_expr(tokens, vars)?;

    while !tokens.is_empty() {
        if let Some(Token::Op(op)) = tokens.first() {
            if op == "||" {
                tokens.remove(0);
                let right = parse_and_expr(tokens, vars)?;
                let res = left.is_truthy() || right.is_truthy();
                left = Value::Bool(res);
                continue;
            }
        }
        break;
    }

    Ok(left)
}

fn parse_and_expr(tokens: &mut Vec<Token>, vars: &VariableStore) -> Result<Value, RuntimeError> {
    let mut left = parse_comparison(tokens, vars)?;

    while !tokens.is_empty() {
        if let Some(Token::Op(op)) = tokens.first() {
            if op == "&&" {
                tokens.remove(0);
                let right = parse_comparison(tokens, vars)?;
                let res = left.is_truthy() && right.is_truthy();
                left = Value::Bool(res);
                continue;
            }
        }
        break;
    }

    Ok(left)
}

fn parse_comparison(tokens: &mut Vec<Token>, vars: &VariableStore) -> Result<Value, RuntimeError> {
    let left = parse_additive(tokens, vars)?;

    if !tokens.is_empty() {
        if let Some(Token::Op(op)) = tokens.first() {
            if matches!(op.as_str(), "==" | "!=" | "<" | "<=" | ">" | ">=") {
                let op = op.clone();
                tokens.remove(0);
                let right = parse_additive(tokens, vars)?;
                let res = compare_values(&left, &op, &right);
                return Ok(Value::Bool(res));
            }
        }
    }

    Ok(left)
}

fn parse_additive(tokens: &mut Vec<Token>, vars: &VariableStore) -> Result<Value, RuntimeError> {
    let mut left = parse_multiplicative(tokens, vars)?;

    while !tokens.is_empty() {
        if let Some(Token::Op(op)) = tokens.first() {
            if op == "+" || op == "-" {
                let op = op.clone();
                tokens.remove(0);
                let right = parse_multiplicative(tokens, vars)?;
                let a = left.as_float().unwrap_or(0.0);
                let b = right.as_float().unwrap_or(0.0);
                let res = if op == "+" { a + b } else { a - b };
                left = Value::Float(res);
                continue;
            }
        }
        break;
    }

    Ok(left)
}

fn parse_multiplicative(tokens: &mut Vec<Token>, vars: &VariableStore) -> Result<Value, RuntimeError> {
    let mut left = parse_unary(tokens, vars)?;

    while !tokens.is_empty() {
        if let Some(Token::Op(op)) = tokens.first() {
            if op == "*" || op == "/" {
                let op = op.clone();
                tokens.remove(0);
                let right = parse_unary(tokens, vars)?;
                let a = left.as_float().unwrap_or(0.0);
                let b = right.as_float().unwrap_or(1.0);
                let res = if op == "*" {
                    a * b
                } else if b != 0.0 {
                    a / b
                } else {
                    0.0
                };
                left = Value::Float(res);
                continue;
            }
        }
        break;
    }

    Ok(left)
}

fn parse_unary(tokens: &mut Vec<Token>, vars: &VariableStore) -> Result<Value, RuntimeError> {
    if !tokens.is_empty() {
        if let Some(Token::Op(op)) = tokens.first() {
            if op == "!" {
                tokens.remove(0);
                let inner = parse_unary(tokens, vars)?;
                return Ok(Value::Bool(!inner.is_truthy()));
            } else if op == "-" {
                tokens.remove(0);
                let inner = parse_unary(tokens, vars)?;
                let num = inner.as_float().unwrap_or(0.0);
                return Ok(Value::Float(-num));
            }
        }
    }
    parse_primary(tokens, vars)
}

fn parse_primary(tokens: &mut Vec<Token>, vars: &VariableStore) -> Result<Value, RuntimeError> {
    if tokens.is_empty() {
        return Err(RuntimeError::ExpressionError("Unexpected end of expression".into()));
    }

    match tokens.remove(0) {
        Token::Number(n) => Ok(Value::Float(n)),
        Token::Str(s) => Ok(Value::String(s)),
        Token::Bool(b) => Ok(Value::Bool(b)),
        Token::Ident(id) => {
            let val = vars.get(&id).cloned().unwrap_or(Value::Bool(false));
            Ok(val)
        }
        Token::LParen => {
            let val = parse_or_expr(tokens, vars)?;
            if tokens.is_empty() || tokens.remove(0) != Token::RParen {
                return Err(RuntimeError::ExpressionError("Missing closing parenthesis".into()));
            }
            Ok(val)
        }
        Token::Op(op) => Err(RuntimeError::ExpressionError(format!(
            "Unexpected operator in primary position: '{op}'"
        ))),
        Token::RParen => Err(RuntimeError::ExpressionError("Unexpected closing parenthesis".into())),
    }
}

fn compare_values(left: &Value, op: &str, right: &Value) -> bool {
    // If both can be compared as floats/numbers
    if let (Some(a), Some(b)) = (left.as_float(), right.as_float()) {
        return match op {
            "==" => (a - b).abs() < f64::EPSILON,
            "!=" => (a - b).abs() >= f64::EPSILON,
            "<" => a < b,
            "<=" => a <= b,
            ">" => a > b,
            ">=" => a >= b,
            _ => false,
        };
    }

    // String comparison
    let a_str = left.as_str_repr();
    let b_str = right.as_str_repr();
    match op {
        "==" => a_str == b_str,
        "!=" => a_str != b_str,
        "<" => a_str < b_str,
        "<=" => a_str <= b_str,
        ">" => a_str > b_str,
        ">=" => a_str >= b_str,
        _ => false,
    }
}
