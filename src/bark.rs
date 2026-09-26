//! NPC Bark system for ambient contextual reactions, priority scoring, cooldowns, and weighted selection.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::runtime::{eval_expression, VariableStore};

/// Standard NPC ambient reaction bark categories.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BarkCategory {
    /// Alerted to combat / enemy sighted.
    CombatAlert,
    /// Retreating or fleeing danger.
    Fleeing,
    /// Taunt or insult directed at adversary.
    Insult,
    /// Ambient idle murmurs or world chatter.
    IdleChatter,
    /// Reaction to witnessed theft, murder, or crime.
    CrimeWitness,
    /// Custom game-specific category.
    Custom(String),
}

/// A single ambient dialogue line / bark.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bark {
    /// Unique identifier for this bark line.
    pub id: String,
    /// Speaker name or archetype (e.g. "CityGuard", "Bandit").
    pub speaker: String,
    /// Bark trigger category.
    pub category: BarkCategory,
    /// Spoken text.
    pub text: String,
    /// Audio asset identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice_asset: Option<String>,
    /// Priority level (higher priority interrupts or supersedes lower priority).
    #[serde(default = "default_priority")]
    pub priority: u32,
    /// Selection probability weight relative to other valid barks.
    #[serde(default = "default_weight")]
    pub weight: f64,
    /// Cooldown duration in seconds before this specific bark can be chosen again.
    #[serde(default = "default_cooldown")]
    pub cooldown: f64,
    /// Optional variable expression filter required to trigger.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
}

fn default_priority() -> u32 {
    10
}

fn default_weight() -> f64 {
    1.0
}

fn default_cooldown() -> f64 {
    15.0
}

/// Ambient Bark Manager handling registration, cooldowns, history, and weighted selection.
#[derive(Debug, Clone, Default)]
pub struct BarkManager {
    barks: Vec<Bark>,
    last_fired_bark: HashMap<String, f64>,
    last_fired_category: HashMap<BarkCategory, f64>,
    recent_history: Vec<String>,
    rng_state: u64,
}

impl BarkManager {
    /// Create a new bark manager.
    pub fn new() -> Self {
        Self {
            barks: Vec::new(),
            last_fired_bark: HashMap::new(),
            last_fired_category: HashMap::new(),
            recent_history: Vec::new(),
            rng_state: 0x853c49e6748fea9b,
        }
    }

    /// Seed the internal pseudo-random number generator for reproducible testing.
    pub fn seed(&mut self, seed: u64) {
        self.rng_state = if seed == 0 { 0x853c49e6748fea9b } else { seed };
    }

    /// Register a bark line.
    pub fn add_bark(&mut self, bark: Bark) {
        self.barks.push(bark);
    }

    /// Get all registered barks.
    pub fn barks(&self) -> &[Bark] {
        &self.barks
    }

    /// Fast linear congruential pseudo-random float in [0.0, 1.0).
    fn next_random_float(&mut self) -> f64 {
        // xorshift64*
        let mut x = self.rng_state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng_state = x;
        let val = x.wrapping_mul(0x2545F4914F6CDD1D);
        (val >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Query and trigger the most appropriate bark for a given category.
    ///
    /// Filtering rules:
    /// 1. Category match.
    /// 2. Condition evaluated against `VariableStore` must be true.
    /// 3. Must not be on individual cooldown at `current_time`.
    /// 4. Highest priority group is selected first.
    /// 5. Within the highest priority group, weighted random choice is made, penalizing or avoiding immediate repeats.
    pub fn trigger_bark(
        &mut self,
        category: &BarkCategory,
        variables: &VariableStore,
        current_time: f64,
    ) -> Option<Bark> {
        // Collect eligible bark indices
        let eligible_indices: Vec<usize> = self
            .barks
            .iter()
            .enumerate()
            .filter(|(_, b)| &b.category == category)
            .filter(|(_, b)| {
                if let Some(ref cond) = b.condition {
                    eval_expression(cond, variables).unwrap_or(false)
                } else {
                    true
                }
            })
            .filter(|(_, b)| {
                if let Some(&last_time) = self.last_fired_bark.get(&b.id) {
                    current_time >= last_time + b.cooldown
                } else {
                    true
                }
            })
            .map(|(idx, _)| idx)
            .collect();

        if eligible_indices.is_empty() {
            return None;
        }

        // Find max priority
        let max_priority = eligible_indices
            .iter()
            .map(|&i| self.barks[i].priority)
            .max()
            .unwrap_or(0);

        let mut top_candidate_indices: Vec<usize> = eligible_indices
            .into_iter()
            .filter(|&i| self.barks[i].priority == max_priority)
            .collect();

        if top_candidate_indices.is_empty() {
            return None;
        }

        // Avoid immediate repeat if alternative candidates exist
        if top_candidate_indices.len() > 1 {
            if let Some(last_id) = self.recent_history.last() {
                let filtered: Vec<usize> = top_candidate_indices
                    .iter()
                    .cloned()
                    .filter(|&i| &self.barks[i].id != last_id)
                    .collect();
                if !filtered.is_empty() {
                    top_candidate_indices = filtered;
                }
            }
        }

        // Weighted random selection
        let total_weight: f64 = top_candidate_indices
            .iter()
            .map(|&i| self.barks[i].weight.max(0.001))
            .sum();

        let rand_val = self.next_random_float();
        let target = rand_val * total_weight;

        let mut acc = 0.0;
        let mut chosen_idx = top_candidate_indices[0];
        for &idx in &top_candidate_indices {
            acc += self.barks[idx].weight.max(0.001);
            if acc >= target {
                chosen_idx = idx;
                break;
            }
        }

        let chosen = self.barks[chosen_idx].clone();

        // Record cooldown and history
        self.last_fired_bark.insert(chosen.id.clone(), current_time);
        self.last_fired_category.insert(chosen.category.clone(), current_time);
        self.recent_history.push(chosen.id.clone());
        if self.recent_history.len() > 20 {
            self.recent_history.remove(0);
        }

        Some(chosen)
    }

    /// Reset all cooldown timers and history.
    pub fn reset_cooldowns(&mut self) {
        self.last_fired_bark.clear();
        self.last_fired_category.clear();
        self.recent_history.clear();
    }
}
