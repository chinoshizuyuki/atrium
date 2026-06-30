// SPDX-License-Identifier: MIT
//! Persona — 人格固化 + sled 持久化
//! Persona — Persona management + sled persistence.

use crate::evidence::{EvidenceConfig, EvidenceScorer};
use crate::fact_store::Fact;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// 人格特质
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trait {
    pub name: String,
    pub value: String,
    pub confidence: f64,
    pub source_count: u32,
    pub is_stable: bool,
}

/// 实体画像
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Persona {
    pub entity: String,
    pub traits: Vec<Trait>,
    pub last_updated: i64,
}

impl Persona {
    pub fn new(entity: &str) -> Self {
        Self {
            entity: entity.to_string(),
            traits: Vec::new(),
            last_updated: 0,
        }
    }

    pub fn has_trait(&self, name: &str) -> bool {
        self.traits.iter().any(|t| t.name == name)
    }

    pub fn stable_traits(&self) -> Vec<&Trait> {
        self.traits.iter().filter(|t| t.is_stable).collect()
    }

    pub fn suppress_unstable(&mut self, min_confidence: f64) {
        let before = self.traits.len();
        self.traits.retain(|t| t.confidence >= min_confidence);
        if self.traits.len() < before {
            self.last_updated = now_secs();
        }
    }
}

/// 人格管理器（多实体）— sled 持久化
pub struct PersonaManager {
    personas: HashMap<String, Persona>,
    scorer: EvidenceScorer,
    pub min_evidence_for_stable: u32,
    pub suppress_threshold: f64,
    db: Option<sled::Db>,
}

impl Default for PersonaManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PersonaManager {
    pub fn new() -> Self {
        Self {
            personas: HashMap::new(),
            scorer: EvidenceScorer::new(EvidenceConfig::default()),
            min_evidence_for_stable: 3,
            suppress_threshold: 0.3,
            db: None,
        }
    }

    pub fn open(db_path: &str) -> Self {
        let db = sled::open(db_path).ok();
        let mut personas = HashMap::new();
        if let Some(ref db) = db {
            for item in db.iter().flatten() {
                let (_, value) = item;
                if let Ok(p) = bincode::deserialize::<Persona>(&value) {
                    personas.insert(p.entity.clone(), p);
                }
            }
            tracing::info!("RuntimePersona: loaded {} personas", personas.len());
        }
        Self {
            personas,
            scorer: EvidenceScorer::new(EvidenceConfig::default()),
            min_evidence_for_stable: 3,
            suppress_threshold: 0.3,
            db,
        }
    }

    pub fn new_in_memory() -> Self {
        Self::new()
    }

    fn persist_all(&self) {
        if let Some(ref db) = self.db {
            for key in db.iter().keys().flatten() {
                let _ = db.remove(key);
            }
            for p in self.personas.values() {
                if let Ok(data) = bincode::serialize(p) {
                    let _ = db.insert(p.entity.as_bytes(), data);
                }
            }
            let _ = db.flush();
        }
    }

    pub fn get_or_create(&mut self, entity: &str) -> &mut Persona {
        self.personas
            .entry(entity.to_string())
            .or_insert_with(|| Persona::new(entity))
    }

    pub fn update_from_facts(&mut self, entity: &str, facts: &[Fact]) {
        let mut scored: Vec<(String, String, f64)> = Vec::new();
        let suppress_threshold = self.suppress_threshold;
        let min_evidence_for_stable = self.min_evidence_for_stable;

        for fact in facts {
            if fact.subject != entity {
                continue;
            }
            let source_type = crate::evidence::parse_source(&fact.source);
            let score = self.scorer.evaluate(fact, source_type, facts, 0.0).total;
            let evidence_based_conf = fact.confidence * score;

            if evidence_based_conf >= suppress_threshold {
                scored.push((
                    format!("{}_{}_{}", fact.subject, fact.predicate, fact.object),
                    fact.object.clone(),
                    evidence_based_conf,
                ));
            }
        }

        let persona = self.get_or_create(entity);
        let now = now_secs();

        for (trait_name, trait_value, conf) in scored {
            if let Some(existing) = persona.traits.iter_mut().find(|t| t.name == trait_name) {
                let total = (existing.source_count + 1) as f64;
                existing.confidence =
                    (existing.confidence * existing.source_count as f64 + conf) / total;
                existing.source_count += 1;
                existing.is_stable = existing.source_count >= min_evidence_for_stable;
            } else {
                persona.traits.push(Trait {
                    name: trait_name,
                    value: trait_value,
                    confidence: conf,
                    source_count: 1,
                    is_stable: false,
                });
            }
        }

        persona.last_updated = now;
        persona.suppress_unstable(suppress_threshold);
        self.persist_all();
    }

    pub fn entities(&self) -> Vec<&str> {
        self.personas.keys().map(|s| s.as_str()).collect()
    }

    pub fn get(&self, entity: &str) -> Option<&Persona> {
        self.personas.get(entity)
    }

    pub fn all_stable_traits(&self) -> Vec<(String, Vec<&Trait>)> {
        self.personas
            .values()
            .map(|p| (p.entity.clone(), p.stable_traits()))
            .filter(|(_, t)| !t.is_empty())
            .collect()
    }
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// 测试用例
#[cfg(test)]
mod tests {
    use super::*;
    use crate::fact_store::Fact;

    #[test]
    fn test_create_persona() {
        let mut mgr = PersonaManager::new();
        let facts = vec![
            Fact::new("主人", "喜欢", "Rust")
                .with_confidence(0.9)
                .with_source("对话_2026-06-08"),
            Fact::new("主人", "擅长", "编程")
                .with_confidence(0.8)
                .with_source("对话_2026-06-08"),
        ];
        mgr.update_from_facts("主人", &facts);
        let persona = mgr.get("主人").unwrap();
        assert_eq!(persona.traits.len(), 2);
    }

    #[test]
    fn test_stable_trait() {
        let mut mgr = PersonaManager::new();
        mgr.min_evidence_for_stable = 2;
        mgr.suppress_threshold = 0.1;

        mgr.update_from_facts(
            "主人",
            &[Fact::new("主人", "喜欢", "Rust").with_confidence(0.9)],
        );
        let persona = mgr.get("主人").unwrap();
        assert_eq!(persona.traits.len(), 1);
    }

    #[test]
    fn test_suppress_low_confidence() {
        let mut mgr = PersonaManager::new();
        mgr.update_from_facts(
            "主人",
            &[
                Fact::new("主人", "喜欢", "Rust").with_confidence(0.9),
                Fact::new("系统", "推断", "噪音").with_confidence(0.1),
            ],
        );
        let persona = mgr.get("主人").unwrap();
        for t in &persona.traits {
            assert!(t.confidence >= mgr.suppress_threshold);
        }
    }

    #[test]
    fn test_multiple_entities() {
        let mut mgr = PersonaManager::new();
        mgr.update_from_facts(
            "主人",
            &[Fact::new("主人", "喜欢", "Rust").with_confidence(0.9)],
        );
        mgr.update_from_facts(
            "Atrium",
            &[Fact::new("Atrium", "身份", "AI助手").with_confidence(1.0)],
        );
        assert_eq!(mgr.entities().len(), 2);
    }
}
