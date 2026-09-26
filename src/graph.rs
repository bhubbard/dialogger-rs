//! Dialogue graph container, validation, builder, and JSON serialization.

use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::schema::{ChoiceNode, ChoiceOption, ConditionNode, DialogueNode, EventNode, MutationOp, NodeId, SetVariableNode, SpeechNode, Value};

/// Graph validation or deserialization errors.
#[derive(Debug, Error, PartialEq)]
pub enum GraphError {
    /// Start node does not exist in graph.
    #[error("Start node '{0}' does not exist in graph")]
    MissingStartNode(NodeId),

    /// A node targets a non-existent destination node.
    #[error("Broken link: node '{from}' points to non-existent node '{to}'")]
    BrokenLink { from: NodeId, to: NodeId },

    /// Serialization or parsing error.
    #[error("JSON error: {0}")]
    JsonError(String),

    /// Missing required fields in Dialogger format.
    #[error("Malformed Dialogger JSON: {0}")]
    MalformedFormat(String),
}

/// A complete dialogue graph with nodes, start entrypoint, and optional metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogueGraph {
    /// Optional title or scene identifier.
    #[serde(default)]
    pub title: String,
    /// The entrypoint node ID where traversal begins.
    pub start_node: NodeId,
    /// Map of node IDs to dialogue nodes.
    pub nodes: HashMap<NodeId, DialogueNode>,
}

impl DialogueGraph {
    /// Create a new dialogue graph with a start node ID.
    pub fn new(start_node: impl Into<NodeId>) -> Self {
        Self {
            title: String::new(),
            start_node: start_node.into(),
            nodes: HashMap::new(),
        }
    }

    /// Set graph title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Add a node to the graph.
    pub fn add_node(&mut self, id: impl Into<NodeId>, node: DialogueNode) {
        self.nodes.insert(id.into(), node);
    }

    /// Get a reference to a node.
    pub fn get_node(&self, id: &str) -> Option<&DialogueNode> {
        self.nodes.get(id)
    }

    /// Get start node ID.
    pub fn start_node(&self) -> &str {
        &self.start_node
    }

    /// Validate the graph for integrity:
    /// - Checks start node exists
    /// - Checks all target/next links resolve to real nodes
    /// - Returns set of unreachable (orphan) nodes
    pub fn validate(&self) -> Result<HashSet<NodeId>, GraphError> {
        if !self.nodes.contains_key(&self.start_node) {
            return Err(GraphError::MissingStartNode(self.start_node.clone()));
        }

        for (id, node) in &self.nodes {
            for target in node.outgoing_targets() {
                if !self.nodes.contains_key(&target) {
                    return Err(GraphError::BrokenLink {
                        from: id.clone(),
                        to: target,
                    });
                }
            }
        }

        // Detect orphans (nodes unreachable from start)
        let mut visited = HashSet::new();
        let mut queue = vec![self.start_node.clone()];

        while let Some(current) = queue.pop() {
            if visited.insert(current.clone()) {
                if let Some(node) = self.nodes.get(&current) {
                    for target in node.outgoing_targets() {
                        if !visited.contains(&target) {
                            queue.push(target);
                        }
                    }
                }
            }
        }

        let all_keys: HashSet<NodeId> = self.nodes.keys().cloned().collect();
        let orphans: HashSet<NodeId> = all_keys.difference(&visited).cloned().collect();

        Ok(orphans)
    }

    /// Serialize graph to JSON string.
    pub fn to_json(&self) -> Result<String, GraphError> {
        serde_json::to_string_pretty(self).map_err(|e| GraphError::JsonError(e.to_string()))
    }

    /// Parse graph from modern JSON schema.
    pub fn from_json(json: &str) -> Result<Self, GraphError> {
        let graph: DialogueGraph =
            serde_json::from_str(json).map_err(|e| GraphError::JsonError(e.to_string()))?;
        Ok(graph)
    }

    /// Import from Evan Todd's classic Dialogger JSON format.
    /// In classic Dialogger, a file is an array of node objects or `{ "nodes": [...] }`,
    /// where nodes have `id` (often numeric), `character`, `text`, and `choices: [{"text": ..., "node": ...}]`.
    pub fn from_dialogger_json(json: &str) -> Result<Self, GraphError> {
        #[derive(Deserialize)]
        struct DialoggerChoice {
            text: String,
            node: serde_json::Value,
            condition: Option<String>,
        }

        #[derive(Deserialize)]
        struct DialoggerNode {
            id: serde_json::Value,
            #[serde(default)]
            character: Option<String>,
            #[serde(default)]
            text: Option<String>,
            #[serde(default)]
            voice: Option<String>,
            #[serde(default)]
            expression: Option<String>,
            #[serde(default)]
            choices: Vec<DialoggerChoice>,
            #[serde(default)]
            next: Option<serde_json::Value>,
            #[serde(default)]
            event: Option<String>,
            #[serde(default)]
            set_var: Option<String>,
            #[serde(default)]
            var_val: Option<serde_json::Value>,
        }

        fn json_val_to_node_id(v: &serde_json::Value) -> String {
            match v {
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            }
        }

        let raw_nodes: Vec<DialoggerNode> = if let Ok(list) = serde_json::from_str::<Vec<DialoggerNode>>(json) {
            list
        } else {
            #[derive(Deserialize)]
            struct Container {
                nodes: Vec<DialoggerNode>,
            }
            let c: Container = serde_json::from_str(json).map_err(|e| GraphError::MalformedFormat(e.to_string()))?;
            c.nodes
        };

        if raw_nodes.is_empty() {
            return Err(GraphError::MalformedFormat("No nodes in Dialogger file".into()));
        }

        let start_id = json_val_to_node_id(&raw_nodes[0].id);
        let mut graph = DialogueGraph::new(start_id);

        for d_node in raw_nodes {
            let id = json_val_to_node_id(&d_node.id);

            // If node has event
            if let Some(event_name) = d_node.event {
                let next = d_node.next.as_ref().map(json_val_to_node_id);
                graph.add_node(
                    id,
                    DialogueNode::Event(EventNode {
                        event_name,
                        payload: None,
                        next,
                    }),
                );
                continue;
            }

            // If node has set_var
            if let Some(var_name) = d_node.set_var {
                let next = d_node.next.as_ref().map(json_val_to_node_id);
                let val = match d_node.var_val {
                    Some(serde_json::Value::Bool(b)) => Value::Bool(b),
                    Some(serde_json::Value::Number(n)) => {
                        if let Some(i) = n.as_i64() {
                            Value::Int(i)
                        } else {
                            Value::Float(n.as_f64().unwrap_or(0.0))
                        }
                    }
                    Some(serde_json::Value::String(s)) => Value::String(s),
                    _ => Value::Bool(true),
                };
                graph.add_node(
                    id,
                    DialogueNode::SetVariable(SetVariableNode {
                        variable: var_name,
                        op: MutationOp::Assign,
                        value: val,
                        next,
                    }),
                );
                continue;
            }

            // If choices are present
            if !d_node.choices.is_empty() {
                // If there's also speech in this node, split or store as Speech leading to Choice
                if let Some(speaker) = d_node.character {
                    let speech_id = id.clone();
                    let choice_id = format!("{id}_choices");
                    let text = d_node.text.unwrap_or_default();

                    graph.add_node(
                        speech_id,
                        DialogueNode::Speech(SpeechNode {
                            speaker,
                            text,
                            voice_asset: d_node.voice,
                            expression: d_node.expression,
                            next: Some(choice_id.clone()),
                        }),
                    );

                    let choices = d_node
                        .choices
                        .into_iter()
                        .enumerate()
                        .map(|(idx, ch)| ChoiceOption {
                            id: format!("opt_{idx}"),
                            text: ch.text,
                            target: json_val_to_node_id(&ch.node),
                            condition: ch.condition,
                        })
                        .collect();

                    graph.add_node(
                        choice_id,
                        DialogueNode::Choice(ChoiceNode {
                            prompt: None,
                            choices,
                        }),
                    );
                    continue;
                } else {
                    let choices = d_node
                        .choices
                        .into_iter()
                        .enumerate()
                        .map(|(idx, ch)| ChoiceOption {
                            id: format!("opt_{idx}"),
                            text: ch.text,
                            target: json_val_to_node_id(&ch.node),
                            condition: ch.condition,
                        })
                        .collect();

                    graph.add_node(
                        id,
                        DialogueNode::Choice(ChoiceNode {
                            prompt: d_node.text,
                            choices,
                        }),
                    );
                    continue;
                }
            }

            // Normal Speech node
            let speaker = d_node.character.unwrap_or_else(|| "Narrator".into());
            let text = d_node.text.unwrap_or_default();
            let next = d_node.next.as_ref().map(json_val_to_node_id);

            graph.add_node(
                id,
                DialogueNode::Speech(SpeechNode {
                    speaker,
                    text,
                    voice_asset: d_node.voice,
                    expression: d_node.expression,
                    next,
                }),
            );
        }

        Ok(graph)
    }
}

/// Fluent builder for constructing dialogue graphs programmatically.
#[derive(Debug, Default)]
pub struct GraphBuilder {
    title: String,
    start_node: Option<NodeId>,
    nodes: HashMap<NodeId, DialogueNode>,
}

impl GraphBuilder {
    /// Create a new empty graph builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set graph title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set start node ID.
    pub fn start_node(mut self, id: impl Into<NodeId>) -> Self {
        self.start_node = Some(id.into());
        self
    }

    /// Add a speech line.
    pub fn speech(
        mut self,
        id: impl Into<NodeId>,
        speaker: impl Into<String>,
        text: impl Into<String>,
        next: Option<&str>,
    ) -> Self {
        let id_s = id.into();
        if self.start_node.is_none() {
            self.start_node = Some(id_s.clone());
        }
        self.nodes.insert(
            id_s,
            DialogueNode::Speech(SpeechNode {
                speaker: speaker.into(),
                text: text.into(),
                voice_asset: None,
                expression: None,
                next: next.map(ToString::to_string),
            }),
        );
        self
    }

    /// Add a speech line with expression and voice asset.
    pub fn speech_rich(
        mut self,
        id: impl Into<NodeId>,
        speaker: impl Into<String>,
        text: impl Into<String>,
        expression: Option<&str>,
        voice_asset: Option<&str>,
        next: Option<&str>,
    ) -> Self {
        let id_s = id.into();
        if self.start_node.is_none() {
            self.start_node = Some(id_s.clone());
        }
        self.nodes.insert(
            id_s,
            DialogueNode::Speech(SpeechNode {
                speaker: speaker.into(),
                text: text.into(),
                expression: expression.map(ToString::to_string),
                voice_asset: voice_asset.map(ToString::to_string),
                next: next.map(ToString::to_string),
            }),
        );
        self
    }

    /// Add a choice node.
    pub fn choice(
        mut self,
        id: impl Into<NodeId>,
        prompt: Option<impl Into<String>>,
        choices: Vec<ChoiceOption>,
    ) -> Self {
        let id_s = id.into();
        if self.start_node.is_none() {
            self.start_node = Some(id_s.clone());
        }
        self.nodes.insert(
            id_s,
            DialogueNode::Choice(ChoiceNode {
                prompt: prompt.map(Into::into),
                choices,
            }),
        );
        self
    }

    /// Add a conditional branch node.
    pub fn condition(
        mut self,
        id: impl Into<NodeId>,
        expr: impl Into<String>,
        then_node: Option<&str>,
        else_node: Option<&str>,
    ) -> Self {
        let id_s = id.into();
        if self.start_node.is_none() {
            self.start_node = Some(id_s.clone());
        }
        self.nodes.insert(
            id_s,
            DialogueNode::Condition(ConditionNode {
                condition: expr.into(),
                then_node: then_node.map(ToString::to_string),
                else_node: else_node.map(ToString::to_string),
            }),
        );
        self
    }

    /// Add a set variable node.
    pub fn set_variable(
        mut self,
        id: impl Into<NodeId>,
        var: impl Into<String>,
        op: MutationOp,
        value: impl Into<Value>,
        next: Option<&str>,
    ) -> Self {
        let id_s = id.into();
        if self.start_node.is_none() {
            self.start_node = Some(id_s.clone());
        }
        self.nodes.insert(
            id_s,
            DialogueNode::SetVariable(SetVariableNode {
                variable: var.into(),
                op,
                value: value.into(),
                next: next.map(ToString::to_string),
            }),
        );
        self
    }

    /// Add an event node.
    pub fn event(
        mut self,
        id: impl Into<NodeId>,
        event_name: impl Into<String>,
        payload: Option<Value>,
        next: Option<&str>,
    ) -> Self {
        let id_s = id.into();
        if self.start_node.is_none() {
            self.start_node = Some(id_s.clone());
        }
        self.nodes.insert(
            id_s,
            DialogueNode::Event(EventNode {
                event_name: event_name.into(),
                payload,
                next: next.map(ToString::to_string),
            }),
        );
        self
    }

    /// Build the dialogue graph.
    pub fn build(self) -> Result<DialogueGraph, GraphError> {
        let start_node = self
            .start_node
            .ok_or_else(|| GraphError::MissingStartNode("None specified".into()))?;

        let graph = DialogueGraph {
            title: self.title,
            start_node,
            nodes: self.nodes,
        };

        graph.validate()?;
        Ok(graph)
    }
}
