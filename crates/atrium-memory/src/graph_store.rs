// SPDX-License-Identifier: MIT
//! GraphStore — 关联记忆图持久化层
//! GraphStore — Associative graph persistence layer.
//!
//! 将 AssociativeGraph 的节点和边持久化到 sled，重启后自动恢复。
//! 遵循 FactStore / ReflectionEngine 的同一模式。
//! Persists AssociativeGraph nodes and edges to sled with auto-recovery on restart.
//! Follows the same pattern as FactStore / ReflectionEngine.
//!
//! 存储格式 / Storage format:
//! - 节点 key / Node key: `"node:{id}"` → bincode(GraphNode)
//! - 边 key / Edge key: `"edge:{edge_id}"` → bincode(GraphEdge)

use crate::associative::{AssociativeGraph, GraphEdge, GraphNode};
use std::collections::HashMap;

/// 关联记忆图持久化存储
pub struct GraphStore {
    db: Option<sled::Db>,
}

impl GraphStore {
    /// 创建持久化存储（sled 数据库路径）
    pub fn new(db_path: &str) -> anyhow::Result<Self> {
        let db = sled::open(db_path)?;
        Ok(Self { db: Some(db) })
    }

    /// 创建纯内存存储（不持久化，用于测试）
    pub fn new_in_memory() -> Self {
        Self { db: None }
    }

    /// 保存整张图（全量写入节点 + 边）
    pub fn save(&self, graph: &AssociativeGraph) -> anyhow::Result<()> {
        let db = match &self.db {
            Some(db) => db,
            None => return Ok(()),
        };

        // 清空旧数据
        db.clear()?;

        // 写入所有节点
        for (id, node) in graph.nodes() {
            let key = format!("node:{}", id);
            let value = bincode::serialize(node)?;
            db.insert(key.as_bytes(), value)?;
        }

        // 写入所有边
        for edge in graph.edges() {
            let key = format!("edge:{}", edge.id);
            let value = bincode::serialize(edge)?;
            db.insert(key.as_bytes(), value)?;
        }

        db.flush()?;
        Ok(())
    }

    /// 加载持久化的图（如果数据库为空返回 None）
    pub fn load(&self) -> anyhow::Result<Option<AssociativeGraph>> {
        let db = match &self.db {
            Some(db) => db,
            None => return Ok(None),
        };

        let mut nodes: HashMap<String, GraphNode> = HashMap::new();
        let mut edges: Vec<GraphEdge> = Vec::new();

        for item in db.iter() {
            let (key, value) = item?;
            let key_str = String::from_utf8_lossy(&key);

            if let Some(node_id) = key_str.strip_prefix("node:") {
                let node: GraphNode = bincode::deserialize(&value)?;
                nodes.insert(node_id.to_string(), node);
            } else if let Some(_edge_id) = key_str.strip_prefix("edge:") {
                let edge: GraphEdge = bincode::deserialize(&value)?;
                edges.push(edge);
            }
        }

        if nodes.is_empty() && edges.is_empty() {
            Ok(None)
        } else {
            Ok(Some(AssociativeGraph::from_parts(nodes, edges)))
        }
    }

    /// 增量保存单个节点
    pub fn save_node(&self, node: &GraphNode) -> anyhow::Result<()> {
        let db = match &self.db {
            Some(db) => db,
            None => return Ok(()),
        };
        let key = format!("node:{}", node.id);
        let value = bincode::serialize(node)?;
        db.insert(key.as_bytes(), value)?;
        Ok(())
    }

    /// 增量保存单条边
    pub fn save_edge(&self, edge: &GraphEdge) -> anyhow::Result<()> {
        let db = match &self.db {
            Some(db) => db,
            None => return Ok(()),
        };
        let key = format!("edge:{}", edge.id);
        let value = bincode::serialize(edge)?;
        db.insert(key.as_bytes(), value)?;
        Ok(())
    }

    /// 删除节点及其关联边
    pub fn remove_node(&self, node_id: &str) -> anyhow::Result<()> {
        let db = match &self.db {
            Some(db) => db,
            None => return Ok(()),
        };

        // 删除节点
        let node_key = format!("node:{}", node_id);
        db.remove(node_key.as_bytes())?;

        // 删除所有关联的边（扫描所有边，找出 from 或 to 包含 node_id 的）
        let mut to_remove: Vec<Vec<u8>> = Vec::new();
        for item in db.scan_prefix(b"edge:") {
            let (key, value) = item?;
            let edge: GraphEdge = bincode::deserialize(&value)?;
            if edge.from == node_id || edge.to == node_id {
                to_remove.push(key.to_vec());
            }
        }
        for key in to_remove {
            db.remove(key)?;
        }

        Ok(())
    }

    /// 数据库中是否有数据
    pub fn has_data(&self) -> bool {
        match &self.db {
            Some(db) => !db.is_empty(),
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::associative::{NodeType, RelationType};
    use crate::fact_store::Fact;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn make_temp_store() -> GraphStore {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let path =
            std::env::temp_dir().join(format!("atrium_graph_test_{}_{}", std::process::id(), id));
        // 确保干净开始
        let _ = std::fs::remove_dir_all(&path);
        GraphStore::new(path.to_str().unwrap()).unwrap()
    }

    #[test]
    fn test_save_and_load() {
        let store = make_temp_store();

        let mut graph = AssociativeGraph::new();
        graph.add_fact(&Fact::new("主人", "喜欢", "Rust").with_confidence(0.9));
        graph.add_fact(&Fact::new("主人", "学习", "AI").with_confidence(0.8));

        let node_count = graph.node_count();
        let edge_count = graph.edge_count();

        store.save(&graph).unwrap();

        let loaded = store.load().unwrap().expect("应有数据");
        assert_eq!(loaded.node_count(), node_count);
        assert_eq!(loaded.edge_count(), edge_count);

        let rust_node = loaded.get_node("O:Rust").expect("应有 O:Rust");
        assert_eq!(rust_node.content, "Rust");
        assert_eq!(rust_node.node_type, NodeType::Fact);
    }

    #[test]
    fn test_incremental_edge() {
        let store = make_temp_store();

        let mut graph = AssociativeGraph::new();
        graph.add_fact(&Fact::new("A", "关联", "B").with_confidence(0.7));
        store.save(&graph).unwrap();

        let edge = GraphEdge {
            id: "S:A->O:C:likes".to_string(),
            from: "S:A".to_string(),
            to: "O:C".to_string(),
            relation: RelationType::CoOccurs,
            predicate: "likes".to_string(),
            weight: 0.6,
            activation_count: 0,
            created_at: 12345,
            last_activated: 0,
            pinned: false,
            emotional_salience: 0.0,
        };
        store.save_edge(&edge).unwrap();

        let loaded = store.load().unwrap().expect("应有数据");
        let found = loaded.edges().iter().any(|e| e.id == "S:A->O:C:likes");
        assert!(found, "增量保存的边应存在");
    }

    #[test]
    fn test_remove_node() {
        let store = make_temp_store();

        let mut graph = AssociativeGraph::new();
        graph.add_fact(&Fact::new("A", "喜欢", "B").with_confidence(0.9));
        graph.add_fact(&Fact::new("C", "知道", "D").with_confidence(0.8));
        store.save(&graph).unwrap();

        store.remove_node("S:A").unwrap();

        let loaded = store.load().unwrap().expect("应有数据");
        assert!(loaded.get_node("S:A").is_none(), "S:A 应被删除");
        let a_edges: Vec<_> = loaded
            .edges()
            .iter()
            .filter(|e| e.from == "S:A" || e.to == "S:A")
            .collect();
        assert!(a_edges.is_empty(), "与 S:A 关联的边应被清理");
        assert!(loaded.get_node("S:C").is_some());
    }

    #[test]
    fn test_persistence_across_reopen() {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let path =
            std::env::temp_dir().join(format!("atrium_graph_test_{}_{}", std::process::id(), id));
        let _ = std::fs::remove_dir_all(&path);
        let path_str = path.to_str().unwrap().to_string();

        // 第一次：创建并保存
        {
            let store = GraphStore::new(&path_str).unwrap();
            let mut graph = AssociativeGraph::new();
            graph.add_fact(&Fact::new("主人", "住在", "杭州").with_confidence(0.95));
            store.save(&graph).unwrap();
            drop(store); // 释放 sled 文件锁
        }

        // 第二次：重新打开并加载
        {
            let store = GraphStore::new(&path_str).unwrap();
            let loaded = store.load().unwrap().expect("应有数据");
            assert!(loaded.get_node("S:主人").is_some());
            assert!(loaded.get_node("O:杭州").is_some());
            assert_eq!(loaded.edge_count(), 1);
        }
    }

    #[test]
    fn test_empty_load() {
        let store = make_temp_store();
        let result = store.load().unwrap();
        assert!(result.is_none(), "空数据库应返回 None");
    }

    #[test]
    fn test_in_memory_no_persist() {
        let store = GraphStore::new_in_memory();

        let mut graph = AssociativeGraph::new();
        graph.add_fact(&Fact::new("A", "测试", "B").with_confidence(0.5));

        store.save(&graph).unwrap();

        let result = store.load().unwrap();
        assert!(result.is_none(), "内存模式加载应返回 None");
    }

    #[test]
    fn test_save_node_incremental() {
        let store = make_temp_store();

        let graph = AssociativeGraph::new();
        store.save(&graph).unwrap();

        let node = GraphNode {
            id: "I:test_insight".to_string(),
            node_type: NodeType::Insight,
            content: "test insight content".to_string(),
            activation: 0.0,
            created_at: 99999,
            access_count: 1,
            last_access: 99999,
            pinned: false,
        };
        store.save_node(&node).unwrap();

        let loaded = store.load().unwrap().expect("应有数据");
        let found = loaded.get_node("I:test_insight");
        assert!(found.is_some(), "增量保存的节点应存在");
        assert_eq!(found.unwrap().node_type, NodeType::Insight);
    }
}
