// SPDX-License-Identifier: MIT
//! 关联记忆图 — 扩散激活，举一反三
//! Associative Memory Graph — Spreading activation for implicit relation discovery.
//!
//! 将 FactStore 中的事实构建成有向图，通过扩散激活发现隐藏关联。
//! "主人喜欢Rust" + "Rust是系统语言" → 推断"主人可能对系统编程感兴趣"
//! Builds a directed graph from FactStore facts, discovers hidden associations
//! via spreading activation. E.g. "user likes Rust" + "Rust is a systems language"
//! → infers "user may be interested in systems programming".

use crate::fact_store::Fact;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

fn now_timestamp() -> i64 {
    chrono::Utc::now().timestamp()
}

// ── 枚举 ──

/// 记忆图节点语义类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeType {
    Fact,       // 事实（"主人住在杭州"）
    Behavior,   // 行为模式（"主人凌晨写代码"）
    Preference, // 偏好（"主人喜欢简短回复"）
    Experience, // 经验（"上次回答太长主人不满意"）
    Concept,    // 概念（"Rust 所有权"）
    Pattern,    // 抽象模式（"用户对过长回复的反感"）
    Insight,    // 洞察（来自 ReflectionEngine）
}

/// 节点间关系类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationType {
    SimilarTo,      // 语义相似（嵌入余弦 > 0.85）
    Causes,         // 因果
    CoOccurs,       // 共现（同一对话中出现）
    Contrast,       // 对比
    InstanceOf,     // 实例关系
    AbstractedFrom, // 抽象关系
    Triggered,      // 触发关系
    SubjectObject,  // 原始 S-P-O 关系（从 Fact 构建）
}

/// 兼容旧版枚举（保留用于向后兼容）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
    Subject,
    Object,
    Predicate,
}

// ── 数据结构 ──

/// 记忆图节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub node_type: NodeType,
    pub content: String,
    pub activation: f64, // 运行时值，每次 spread_activation 重置
    pub created_at: i64,
    pub access_count: u64,
    pub last_access: i64,
}

/// 记忆图边
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub relation: RelationType,
    pub predicate: String,
    pub weight: f64,
    pub activation_count: u32,
    pub created_at: i64,
    pub last_activated: i64,
}

/// 图统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub avg_weight: f64,
    pub max_activation: f64,
    pub node_type_distribution: HashMap<NodeType, usize>,
}

// ── 关联记忆图 ──

/// 关联记忆图
pub struct AssociativeGraph {
    nodes: HashMap<String, GraphNode>,
    edges: Vec<GraphEdge>,
}

impl Default for AssociativeGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl AssociativeGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
        }
    }

    /// 从已有的节点和边构建图（用于持久化加载）
    pub fn from_parts(nodes: HashMap<String, GraphNode>, edges: Vec<GraphEdge>) -> Self {
        Self { nodes, edges }
    }

    // ── 内部辅助 ──

    /// 插入或更新节点
    fn insert_node(&mut self, id: String, node_type: NodeType, content: String, _confidence: f64) {
        let now = now_timestamp();
        let entry = self.nodes.entry(id.clone()).or_insert(GraphNode {
            id: id.clone(),
            node_type: node_type.clone(),
            content: content.clone(),
            activation: 0.0,
            created_at: now,
            access_count: 0,
            last_access: 0,
        });
        entry.id = id;
        entry.node_type = node_type;
        entry.content = content;
        entry.access_count += 1;
        entry.last_access = now;
    }

    // ── 公开 API ──

    /// 从 Fact 列表构建图（向后兼容，委托到 add_fact）
    pub fn build(&mut self, facts: &[Fact]) {
        for fact in facts {
            self.add_fact(fact);
        }
    }

    /// 增量添加一条事实到图中
    ///
    /// 不重建整张图，只添加新的节点和边。
    /// 如果节点已存在，更新内容并增加访问计数。
    /// 如果边已存在（相同 edge_id），更新权重为 max。
    pub fn add_fact(&mut self, fact: &Fact) {
        let subj_id = format!("S:{}", fact.subject);
        let obj_id = format!("O:{}", fact.object);
        let pred_id = format!("P:{}", fact.predicate);

        // 插入/更新节点
        self.insert_node(
            subj_id.clone(),
            NodeType::Fact,
            fact.subject.clone(),
            fact.confidence,
        );
        self.insert_node(
            obj_id.clone(),
            NodeType::Fact,
            fact.object.clone(),
            fact.confidence,
        );
        self.insert_node(
            pred_id.clone(),
            NodeType::Concept,
            fact.predicate.clone(),
            fact.confidence,
        );

        // 边去重：相同 edge_id 则更新权重
        let edge_id = format!("{}->{}:{}", subj_id, obj_id, fact.predicate);
        if let Some(edge) = self.edges.iter_mut().find(|e| e.id == edge_id) {
            edge.weight = edge.weight.max(fact.confidence);
            edge.activation_count += 1;
        } else {
            self.edges.push(GraphEdge {
                id: edge_id,
                from: subj_id,
                to: obj_id,
                relation: RelationType::SubjectObject,
                predicate: fact.predicate.clone(),
                weight: fact.confidence,
                activation_count: 0,
                created_at: now_timestamp(),
                last_activated: 0,
            });
        }
    }

    /// 从 ReflectionEngine 的 Insight 添加节点
    ///
    /// 创建 Insight 类型节点，并关联到所有 supporting facts 对应的节点。
    pub fn add_insight(&mut self, summary: &str, supporting_facts: &[String], confidence: f64) {
        let insight_id = format!("I:{}", summary);
        let now = now_timestamp();

        self.nodes.insert(
            insight_id.clone(),
            GraphNode {
                id: insight_id.clone(),
                node_type: NodeType::Insight,
                content: summary.to_string(),
                activation: 0.0,
                created_at: now,
                access_count: 1,
                last_access: now,
            },
        );

        // 关联到 supporting facts 对应的节点
        for fact_ref in supporting_facts {
            // 查找包含该 fact 文本的节点
            let matching: Vec<String> = self
                .nodes
                .values()
                .filter(|n| n.content == *fact_ref && n.id != insight_id)
                .map(|n| n.id.clone())
                .collect();

            for node_id in matching {
                let edge_id = format!("{}->{}:insight", insight_id, node_id);
                if !self.edges.iter().any(|e| e.id == edge_id) {
                    self.edges.push(GraphEdge {
                        id: edge_id,
                        from: insight_id.clone(),
                        to: node_id,
                        relation: RelationType::AbstractedFrom,
                        predicate: "insight".to_string(),
                        weight: confidence,
                        activation_count: 0,
                        created_at: now,
                        last_activated: 0,
                    });
                }
            }
        }
    }

    /// 在两个已有节点间建立关联边
    pub fn link(&mut self, from: &str, to: &str, relation: RelationType, weight: f64) {
        if !self.nodes.contains_key(from) || !self.nodes.contains_key(to) {
            return;
        }
        let now = now_timestamp();
        let edge_id = format!("{}->{}:{:?}", from, to, relation);
        if !self.edges.iter().any(|e| e.id == edge_id) {
            self.edges.push(GraphEdge {
                id: edge_id,
                from: from.to_string(),
                to: to.to_string(),
                relation,
                predicate: String::new(),
                weight: weight.clamp(0.0, 1.0),
                activation_count: 0,
                created_at: now,
                last_activated: 0,
            });
        }
    }

    /// 在同轮对话提取的两个 Fact 之间建立共现边
    ///
    /// 在两个 Fact 的 Object 节点间建立 CoOccurs 关系边。
    /// 权重取两个 Fact 置信度的平均值。
    pub fn link_co_occurs(&mut self, fact_a: &Fact, fact_b: &Fact) {
        let obj_a = format!("O:{}", fact_a.object);
        let obj_b = format!("O:{}", fact_b.object);
        if obj_a == obj_b {
            return; // 相同对象不需要共现边
        }
        let weight = (fact_a.confidence + fact_b.confidence) / 2.0;
        self.link(&obj_a, &obj_b, RelationType::CoOccurs, weight);
    }

    /// 在两个矛盾的 Fact 之间建立对比边
    ///
    /// 在两个 Fact 的 Subject 节点间建立 Contrast 关系边。
    pub fn link_contrast(&mut self, fact_a: &Fact, fact_b: &Fact) {
        let subj_a = format!("S:{}", fact_a.subject);
        let subj_b = format!("S:{}", fact_b.subject);
        let weight = ((fact_a.confidence + fact_b.confidence) / 2.0).min(0.8);
        // 对比边建在主体节点间（同一主体的不同偏好形成对比）
        if subj_a == subj_b {
            // 同一主体的矛盾：Subject 节点自环无意义，改为 Object 间
            let obj_a = format!("O:{}", fact_a.object);
            let obj_b = format!("O:{}", fact_b.object);
            self.link(&obj_a, &obj_b, RelationType::Contrast, weight);
        } else {
            self.link(&subj_a, &subj_b, RelationType::Contrast, weight);
        }
    }

    /// 按 ID 获取节点引用
    pub fn get_node(&self, id: &str) -> Option<&GraphNode> {
        self.nodes.get(id)
    }

    /// 获取所有节点的不可变引用
    pub fn nodes(&self) -> &HashMap<String, GraphNode> {
        &self.nodes
    }

    /// 获取所有边的不可变引用
    pub fn edges(&self) -> &[GraphEdge] {
        &self.edges
    }

    /// 扩散激活：从种子节点出发，沿边扩散到关联节点
    ///
    /// ：边权重参与激活计算
    /// `next_activation = activation * decay_rate * edge.weight`
    pub fn spread_activation(
        &mut self,
        seed: &str,
        decay_rate: f64,
        max_hops: u32,
    ) -> Vec<ActivatedPath> {
        let now = now_timestamp();

        // 重置所有节点激活值
        for node in self.nodes.values_mut() {
            node.activation = 0.0;
        }

        // BFS 扩散
        let mut queue: VecDeque<(String, u32, f64)> = VecDeque::new();
        let mut visited: HashSet<String> = HashSet::new();
        let seed_id = format!("O:{}", seed);
        let subject_seed = format!("S:{}", seed);

        // 初始激活：种子节点
        if let Some(node) = self.nodes.get_mut(&seed_id) {
            node.activation = 1.0;
            queue.push_back((seed_id.clone(), 0, 1.0));
            visited.insert(seed_id);
        }
        if let Some(node) = self.nodes.get_mut(&subject_seed) {
            node.activation = 1.0;
            queue.push_back((subject_seed.clone(), 0, 1.0));
            visited.insert(subject_seed);
        }

        let mut paths = Vec::new();

        while let Some((current, hops, activation)) = queue.pop_front() {
            if hops >= max_hops {
                continue;
            }

            // 收集本轮需要更新的边和节点（避免同时借用 nodes 和 edges）
            let mut updates: Vec<(usize, String, f64)> = Vec::new();

            for (i, edge) in self.edges.iter().enumerate() {
                let next = if edge.from == current {
                    &edge.to
                } else if edge.to == current {
                    &edge.from
                } else {
                    continue;
                };

                if !visited.contains(next) {
                    // ：边权重参与激活计算
                    let next_activation = activation * decay_rate * edge.weight;
                    if next_activation >= 0.1 {
                        updates.push((i, next.clone(), next_activation));
                    }
                }
            }

            // 应用所有更新
            for (edge_idx, next, next_activation) in updates {
                if !visited.insert(next.clone()) {
                    continue;
                }

                // 更新边的激活统计
                self.edges[edge_idx].activation_count += 1;
                self.edges[edge_idx].last_activated = now;

                // 更新目标节点
                if let Some(node) = self.nodes.get_mut(&next) {
                    node.activation = node.activation.max(next_activation);
                    node.access_count += 1;
                    node.last_access = now;
                }

                queue.push_back((next.clone(), hops + 1, next_activation));
                paths.push(ActivatedPath {
                    from: current.clone(),
                    to: next,
                    predicate: self.edges[edge_idx].predicate.clone(),
                    activation: next_activation,
                    hops: hops + 1,
                });
            }
        }

        paths.sort_by(|a, b| {
            b.activation
                .partial_cmp(&a.activation)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        paths
    }

    /// 查询与某实体最相关的 Top-K 节点
    pub fn related(&self, entity: &str, top_k: usize) -> Vec<(String, f64)> {
        let mut results: Vec<(String, f64)> = Vec::new();
        let prefix_o = format!("O:{}", entity);
        let prefix_s = format!("S:{}", entity);

        for edge in &self.edges {
            if edge.from == prefix_o || edge.from == prefix_s {
                results.push((edge.to.clone(), edge.weight));
            }
            if edge.to == prefix_o || edge.to == prefix_s {
                results.push((edge.from.clone(), edge.weight));
            }
        }

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);
        results
    }

    /// 衰减边权重并清理弱边和孤立节点
    ///
    /// - `decay_factor`: 每条边 weight *= decay_factor (e.g. 0.995)
    /// - `min_weight`: weight < min_weight 的边被移除
    /// - 移除边后无连接的孤立节点也被清理
    pub fn decay_and_prune(&mut self, decay_factor: f64, min_weight: f64) {
        // 1. 衰减所有边权重
        for edge in self.edges.iter_mut() {
            edge.weight *= decay_factor;
        }
        // 2. 移除弱边
        self.edges.retain(|e| e.weight >= min_weight);
        // 3. 移除孤立节点（无任何边连接）
        let connected: HashSet<String> = self
            .edges
            .iter()
            .flat_map(|e| vec![e.from.clone(), e.to.clone()])
            .collect();
        self.nodes.retain(|id, _| connected.contains(id));
    }

    /// 获取图统计信息
    pub fn stats(&self) -> GraphStats {
        let mut dist: HashMap<NodeType, usize> = HashMap::new();
        for node in self.nodes.values() {
            *dist.entry(node.node_type.clone()).or_insert(0) += 1;
        }
        let avg_weight = if self.edges.is_empty() {
            0.0
        } else {
            self.edges.iter().map(|e| e.weight).sum::<f64>() / self.edges.len() as f64
        };
        let max_activation = self
            .nodes
            .values()
            .map(|n| n.activation)
            .fold(0.0_f64, f64::max);
        GraphStats {
            node_count: self.nodes.len(),
            edge_count: self.edges.len(),
            avg_weight,
            max_activation,
            node_type_distribution: dist,
        }
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

/// 激活路径
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivatedPath {
    pub from: String,
    pub to: String,
    pub predicate: String,
    pub activation: f64,
    pub hops: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── 原有测试（保持兼容） ──

    #[test]
    fn test_build_graph() {
        let facts = vec![
            Fact::new("主人", "喜欢", "Rust").with_confidence(0.9),
            Fact::new("主人", "喜欢", "AI").with_confidence(0.8),
        ];
        let mut g = AssociativeGraph::new();
        g.build(&facts);
        assert!(g.node_count() >= 4); // S:主人 + O:Rust + O:AI + P:喜欢
        assert_eq!(g.edge_count(), 2);
    }

    #[test]
    fn test_spread_activation() {
        let facts = vec![
            Fact::new("主人", "喜欢", "Rust").with_confidence(0.9),
            Fact::new("主人", "喜欢", "编程").with_confidence(0.85),
            Fact::new("主人", "知道", "Rust").with_confidence(0.7),
        ];
        let mut g = AssociativeGraph::new();
        g.build(&facts);
        let paths = g.spread_activation("Rust", 0.5, 3);
        assert!(!paths.is_empty(), "应从Rust扩散到关联节点");
    }

    #[test]
    fn test_related() {
        let facts = vec![
            Fact::new("主人", "喜欢", "Rust").with_confidence(0.9),
            Fact::new("主人", "喜欢", "Go").with_confidence(0.6),
            Fact::new("主人", "在", "杭州").with_confidence(0.95),
        ];
        let mut g = AssociativeGraph::new();
        g.build(&facts);
        let related = g.related("主人", 5);
        assert!(!related.is_empty());
    }

    // ── 测试 ──

    #[test]
    fn test_node_type_classification() {
        let facts = vec![Fact::new("主人", "喜欢", "Rust").with_confidence(0.9)];
        let mut g = AssociativeGraph::new();
        g.build(&facts);

        let subj = g.get_node("S:主人").expect("应有 S:主人 节点");
        assert_eq!(subj.node_type, NodeType::Fact);
        assert_eq!(subj.content, "主人");

        let obj = g.get_node("O:Rust").expect("应有 O:Rust 节点");
        assert_eq!(obj.node_type, NodeType::Fact);
        assert_eq!(obj.content, "Rust");

        let pred = g.get_node("P:喜欢").expect("应有 P:喜欢 节点");
        assert_eq!(pred.node_type, NodeType::Concept);
        assert_eq!(pred.content, "喜欢");
    }

    #[test]
    fn test_relation_type_from_fact() {
        let facts = vec![Fact::new("主人", "喜欢", "Rust").with_confidence(0.9)];
        let mut g = AssociativeGraph::new();
        g.build(&facts);

        assert_eq!(g.edges().len(), 1);
        assert_eq!(g.edges()[0].relation, RelationType::SubjectObject);
        assert_eq!(g.edges()[0].predicate, "喜欢");
        assert_eq!(g.edges()[0].from, "S:主人");
        assert_eq!(g.edges()[0].to, "O:Rust");
        assert!(!g.edges()[0].id.is_empty());
    }

    #[test]
    fn test_add_fact_incremental() {
        let mut g = AssociativeGraph::new();

        let f1 = Fact::new("主人", "喜欢", "Rust").with_confidence(0.9);
        g.add_fact(&f1);
        let count_after_first = g.node_count();
        assert!(count_after_first >= 3);
        assert_eq!(g.edge_count(), 1);

        let f2 = Fact::new("主人", "学习", "AI").with_confidence(0.8);
        g.add_fact(&f2);
        assert!(g.node_count() > count_after_first, "增量添加应增加新节点");
        assert_eq!(g.edge_count(), 2);

        // 验证第一个事实的节点仍在
        assert!(g.get_node("O:Rust").is_some());
        assert!(g.get_node("O:AI").is_some());
    }

    #[test]
    fn test_add_insight() {
        let mut g = AssociativeGraph::new();

        // 先添加一些事实
        g.add_fact(&Fact::new("主人", "喜欢", "Rust").with_confidence(0.9));
        g.add_fact(&Fact::new("主人", "学习", "AI").with_confidence(0.8));

        let count_before = g.node_count();
        g.add_insight(
            "主人对技术充满热情",
            &["Rust".to_string(), "AI".to_string()],
            0.85,
        );

        assert_eq!(g.node_count(), count_before + 1, "应新增一个 Insight 节点");

        let insight_node = g
            .get_node("I:主人对技术充满热情")
            .expect("应有 Insight 节点");
        assert_eq!(insight_node.node_type, NodeType::Insight);

        // 应有边连接到 supporting facts
        let insight_edges: Vec<&GraphEdge> = g
            .edges()
            .iter()
            .filter(|e| e.from == "I:主人对技术充满热情")
            .collect();
        assert!(
            insight_edges.len() >= 2,
            "应连接到至少 2 个 supporting facts, got {}",
            insight_edges.len()
        );
        assert!(insight_edges
            .iter()
            .all(|e| e.relation == RelationType::AbstractedFrom));
    }

    #[test]
    fn test_link_nodes() {
        let mut g = AssociativeGraph::new();
        g.add_fact(&Fact::new("主人", "喜欢", "Rust").with_confidence(0.9));
        g.add_fact(&Fact::new("主人", "学习", "Go").with_confidence(0.8));

        let edges_before = g.edge_count();
        g.link("O:Rust", "O:Go", RelationType::CoOccurs, 0.7);
        assert_eq!(g.edge_count(), edges_before + 1);

        let link_edge = g
            .edges()
            .iter()
            .find(|e| e.relation == RelationType::CoOccurs)
            .expect("应找到 CoOccurs 边");
        assert_eq!(link_edge.from, "O:Rust");
        assert_eq!(link_edge.to, "O:Go");
        assert!((link_edge.weight - 0.7).abs() < 0.01);

        // link 到不存在的节点应静默失败
        let edges_before = g.edge_count();
        g.link("O:不存在", "O:Rust", RelationType::SimilarTo, 0.5);
        assert_eq!(g.edge_count(), edges_before, "不存在的节点不应建边");
    }

    #[test]
    fn test_weight_in_activation() {
        let mut g = AssociativeGraph::new();
        // 两条路径从"主人"出发，权重不同
        g.add_fact(&Fact::new("主人", "强关联", "Rust").with_confidence(1.0));
        g.add_fact(&Fact::new("主人", "弱关联", "Java").with_confidence(0.3));

        let paths = g.spread_activation("主人", 0.5, 2);

        let rust_path = paths.iter().find(|p| p.to == "O:Rust");
        let java_path = paths.iter().find(|p| p.to == "O:Java");

        assert!(rust_path.is_some(), "应扩散到 Rust");
        assert!(java_path.is_some(), "应扩散到 Java");

        // 权重参与计算：高权重路径激活值更高
        let rust_act = rust_path.unwrap().activation;
        let java_act = java_path.unwrap().activation;
        assert!(
            rust_act > java_act,
            "高权重边(1.0)激活值({})应 > 低权重边(0.3)激活值({})",
            rust_act,
            java_act
        );

        // 验证具体数值：activation = 1.0 * decay(0.5) * weight
        assert!(
            (rust_act - 0.5).abs() < 0.01,
            "Rust 激活值应为 0.5, got {}",
            rust_act
        );
        assert!(
            (java_act - 0.15).abs() < 0.01,
            "Java 激活值应为 0.15, got {}",
            java_act
        );
    }

    #[test]
    fn test_decay_and_prune() {
        let mut g = AssociativeGraph::new();
        g.add_fact(&Fact::new("A", "强关联", "B").with_confidence(1.0));
        g.add_fact(&Fact::new("C", "弱关联", "D").with_confidence(0.1));
        assert_eq!(g.edge_count(), 2);
        assert!(g.get_node("S:A").is_some());
        assert!(g.get_node("S:C").is_some());

        // 衰减 0.5 + 阈值 0.1
        // 强边: 1.0 * 0.5 = 0.5 (保留)
        // 弱边: 0.1 * 0.5 = 0.05 (移除)
        g.decay_and_prune(0.5, 0.1);

        assert_eq!(g.edge_count(), 1, "弱边应被清理");
        assert_eq!(g.edges()[0].from, "S:A", "保留的应是强边");

        // C/D 节点应被清理（孤立）
        assert!(g.get_node("S:C").is_none(), "孤立节点 S:C 应被清理");
        assert!(g.get_node("O:D").is_none(), "孤立节点 O:D 应被清理");

        // A/B 节点应仍在
        assert!(g.get_node("S:A").is_some());
        assert!(g.get_node("O:B").is_some());
    }

    #[test]
    fn test_serialize_deserialize() {
        // GraphNode 序列化往返
        let node = GraphNode {
            id: "S:test".to_string(),
            node_type: NodeType::Fact,
            content: "test content".to_string(),
            activation: 0.5,
            created_at: 1234567890,
            access_count: 42,
            last_access: 1234567900,
        };
        let encoded = bincode::serialize(&node).unwrap();
        let decoded: GraphNode = bincode::deserialize(&encoded).unwrap();
        assert_eq!(decoded.id, node.id);
        assert_eq!(decoded.node_type, NodeType::Fact);
        assert_eq!(decoded.content, "test content");
        assert_eq!(decoded.access_count, 42);

        // GraphEdge 序列化往返
        let edge = GraphEdge {
            id: "S:a->O:b:test".to_string(),
            from: "S:a".to_string(),
            to: "O:b".to_string(),
            relation: RelationType::CoOccurs,
            predicate: "test".to_string(),
            weight: 0.85,
            activation_count: 7,
            created_at: 1234567890,
            last_activated: 1234567900,
        };
        let encoded = bincode::serialize(&edge).unwrap();
        let decoded: GraphEdge = bincode::deserialize(&encoded).unwrap();
        assert_eq!(decoded.id, edge.id);
        assert_eq!(decoded.relation, RelationType::CoOccurs);
        assert!((decoded.weight - 0.85).abs() < f64::EPSILON);
        assert_eq!(decoded.activation_count, 7);

        // GraphStats 序列化往返
        let stats = GraphStats {
            node_count: 10,
            edge_count: 5,
            avg_weight: 0.6,
            max_activation: 0.9,
            node_type_distribution: {
                let mut m = HashMap::new();
                m.insert(NodeType::Fact, 6);
                m.insert(NodeType::Concept, 4);
                m
            },
        };
        let encoded = bincode::serialize(&stats).unwrap();
        let decoded: GraphStats = bincode::deserialize(&encoded).unwrap();
        assert_eq!(decoded.node_count, 10);
        assert_eq!(decoded.node_type_distribution[&NodeType::Fact], 6);
    }

    #[test]
    fn test_stats() {
        let mut g = AssociativeGraph::new();
        g.add_fact(&Fact::new("A", "喜欢", "B").with_confidence(0.8));
        g.add_fact(&Fact::new("C", "知道", "D").with_confidence(0.6));

        let s = g.stats();
        assert_eq!(s.node_count, g.node_count());
        assert_eq!(s.edge_count, 2);
        assert!((s.avg_weight - 0.7).abs() < 0.01); // (0.8+0.6)/2
        assert!(s.node_type_distribution.contains_key(&NodeType::Fact));
        assert!(s.node_type_distribution.contains_key(&NodeType::Concept));
    }

    // ── 高级关联测试 / Advanced Association Tests ──

    #[test]
    fn test_link_co_occurs() {
        let mut g = AssociativeGraph::new();
        let fact_a = Fact::new("主人", "喜欢", "Rust").with_confidence(0.9);
        let fact_b = Fact::new("主人", "学习", "AI").with_confidence(0.8);
        g.add_fact(&fact_a);
        g.add_fact(&fact_b);

        let edges_before = g.edge_count();
        g.link_co_occurs(&fact_a, &fact_b);

        assert_eq!(g.edge_count(), edges_before + 1, "应新增一条共现边");
        let co_edge = g
            .edges()
            .iter()
            .find(|e| e.relation == RelationType::CoOccurs)
            .expect("应有 CoOccurs 边");
        assert_eq!(co_edge.from, "O:Rust");
        assert_eq!(co_edge.to, "O:AI");
        assert!((co_edge.weight - 0.85).abs() < 0.01); // (0.9+0.8)/2
    }

    #[test]
    fn test_link_co_occurs_same_object_skipped() {
        let mut g = AssociativeGraph::new();
        let fact_a = Fact::new("主人", "喜欢", "Rust").with_confidence(0.9);
        let fact_b = Fact::new("AI", "使用", "Rust").with_confidence(0.8);
        g.add_fact(&fact_a);
        g.add_fact(&fact_b);

        let edges_before = g.edge_count();
        g.link_co_occurs(&fact_a, &fact_b);
        assert_eq!(g.edge_count(), edges_before, "相同对象不应产生共现边");
    }

    #[test]
    fn test_link_contrast_same_subject() {
        let mut g = AssociativeGraph::new();
        let fact_a = Fact::new("主人", "喜欢", "Rust").with_confidence(0.9);
        let fact_b = Fact::new("主人", "讨厌", "Java").with_confidence(0.7);
        g.add_fact(&fact_a);
        g.add_fact(&fact_b);

        g.link_contrast(&fact_a, &fact_b);

        let contrast_edge = g
            .edges()
            .iter()
            .find(|e| e.relation == RelationType::Contrast)
            .expect("应有 Contrast 边");
        // 同一主体时，对比边建在 Object 之间
        assert_eq!(contrast_edge.from, "O:Rust");
        assert_eq!(contrast_edge.to, "O:Java");
        assert!(contrast_edge.weight <= 0.8, "对比权重应 capped at 0.8");
    }

    #[test]
    fn test_link_contrast_different_subjects() {
        let mut g = AssociativeGraph::new();
        let fact_a = Fact::new("主人", "喜欢", "Rust").with_confidence(0.9);
        let fact_b = Fact::new("小明", "讨厌", "Rust").with_confidence(0.8);
        g.add_fact(&fact_a);
        g.add_fact(&fact_b);

        g.link_contrast(&fact_a, &fact_b);

        let contrast_edge = g
            .edges()
            .iter()
            .find(|e| e.relation == RelationType::Contrast)
            .expect("应有 Contrast 边");
        // 不同主体时，对比边建在 Subject 之间
        assert_eq!(contrast_edge.from, "S:主人");
        assert_eq!(contrast_edge.to, "S:小明");
    }
}
