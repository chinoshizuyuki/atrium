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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

/// 关联记忆图 / Associative memory graph
///
/// 数字生命的记忆关联皮层——将事实构建为有向图，
/// 通过扩散激活发现隐藏关联，实现"举一反三"的认知涌现。
///
/// The digital life's memory association cortex — builds facts into a directed graph,
/// discovers hidden associations via spreading activation, enabling "inferring by analogy".
///
/// 性能优化：邻接表索引 + 边ID索引，将扩散激活从 O(E×hops) 降至 O(d×hops)。
/// Performance: adjacency list + edge ID index, reducing spread activation from O(E×hops) to O(d×hops).
pub struct AssociativeGraph {
    // 节点表 / Node table
    nodes: HashMap<String, GraphNode>,
    // 边表 / Edge table
    edges: Vec<GraphEdge>,

    // 邻接表索引 / Adjacency list index
    // node_id → Vec<(edge_idx, neighbor_id)>
    // 包含出边和入边两个方向，支持无向遍历 / Bidirectional for undirected traversal
    adjacency: HashMap<String, Vec<(usize, String)>>,

    // 边ID索引 / Edge ID index
    // edge_id → edge_idx in edges vec，用于 O(1) 边去重 / For O(1) edge dedup
    edge_index: HashMap<String, usize>,
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
            adjacency: HashMap::new(),
            edge_index: HashMap::new(),
        }
    }

    /// 从已有的节点和边构建图（用于持久化加载）
    /// Build graph from existing nodes and edges (for persistence loading).
    ///
    /// 自动重建邻接表和边ID索引，确保加载后即获得最优查询性能。
    /// Automatically rebuilds adjacency list and edge ID index for optimal query performance.
    pub fn from_parts(nodes: HashMap<String, GraphNode>, edges: Vec<GraphEdge>) -> Self {
        let mut graph = Self {
            nodes,
            edges,
            adjacency: HashMap::new(),
            edge_index: HashMap::new(),
        };
        graph.rebuild_indices();
        graph
    }

    // ── 内部辅助 / Internal helpers ──

    /// 全量重建邻接表和边ID索引 / Full rebuild of adjacency list and edge ID index
    ///
    /// 在 from_parts() 和 decay_and_prune() 后调用，确保索引与边表一致。
    /// Called after from_parts() and decay_and_prune() to ensure index consistency.
    fn rebuild_indices(&mut self) {
        self.adjacency.clear();
        self.edge_index.clear();

        for (idx, edge) in self.edges.iter().enumerate() {
            self.edge_index.insert(edge.id.clone(), idx);

            // 出边：from → (idx, to) / Outgoing: from → (idx, to)
            self.adjacency
                .entry(edge.from.clone())
                .or_default()
                .push((idx, edge.to.clone()));

            // 入边：to → (idx, from) / Incoming: to → (idx, from)
            self.adjacency
                .entry(edge.to.clone())
                .or_default()
                .push((idx, edge.from.clone()));
        }
    }

    /// 内部辅助：添加边并更新索引 / Internal helper: add edge and update indices
    ///
    /// 返回新边在 edges 中的索引。调用方需确保 edge_id 不重复。
    /// Returns the index of the new edge. Caller must ensure edge_id is unique.
    fn add_edge_internal(&mut self, edge: GraphEdge) -> usize {
        let idx = self.edges.len();
        let from = edge.from.clone();
        let to = edge.to.clone();

        self.edges.push(edge);
        self.edge_index.insert(self.edges[idx].id.clone(), idx);

        // 出边 / Outgoing
        self.adjacency
            .entry(from.clone())
            .or_default()
            .push((idx, to.clone()));
        // 入边 / Incoming
        self.adjacency.entry(to).or_default().push((idx, from));

        idx
    }

    /// 插入或更新节点 / Insert or update node
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

        // 边去重：相同 edge_id 则更新权重 / Edge dedup: same edge_id → update weight
        // O(1) 查找替代 O(E) 线性扫描 / O(1) lookup replaces O(E) linear scan
        let edge_id = format!("{}->{}:{}", subj_id, obj_id, fact.predicate);
        if let Some(&idx) = self.edge_index.get(&edge_id) {
            self.edges[idx].weight = self.edges[idx].weight.max(fact.confidence);
            self.edges[idx].activation_count += 1;
        } else {
            self.add_edge_internal(GraphEdge {
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
                // O(1) 边存在性检查替代 O(E) 线性扫描 / O(1) check replaces O(E) scan
                if !self.edge_index.contains_key(&edge_id) {
                    self.add_edge_internal(GraphEdge {
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
        // O(1) 边存在性检查替代 O(E) 线性扫描 / O(1) check replaces O(E) scan
        if !self.edge_index.contains_key(&edge_id) {
            self.add_edge_internal(GraphEdge {
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
    /// Spreading activation: from seed node, spread along edges to associated nodes.
    ///
    /// 边权重参与激活计算 / Edge weight participates in activation:
    /// `next_activation = activation * decay_rate * edge.weight`
    ///
    /// 性能：O(d × hops)，d = 平均邻居数。邻接表索引避免全边扫描。
    /// Performance: O(d × hops), d = avg neighbors. Adjacency list avoids full edge scan.
    pub fn spread_activation(
        &mut self,
        seed: &str,
        decay_rate: f64,
        max_hops: u32,
    ) -> Vec<ActivatedPath> {
        let now = now_timestamp();

        // 重置所有节点激活值 / Reset all node activations
        for node in self.nodes.values_mut() {
            node.activation = 0.0;
        }

        // BFS 扩散 / BFS spread
        let mut queue: VecDeque<(String, u32, f64)> = VecDeque::new();
        let mut visited: HashSet<String> = HashSet::new();
        let seed_id = format!("O:{}", seed);
        let subject_seed = format!("S:{}", seed);

        // 初始激活：种子节点 / Initial activation: seed nodes
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

            // 邻接表直接取邻居 O(d) / Get neighbors from adjacency list O(d)
            let neighbors = match self.adjacency.get(&current) {
                Some(n) => n,
                None => continue,
            };

            // 收集本轮需要更新的边和节点 / Collect updates for this round
            let mut updates: Vec<(usize, String, f64)> = Vec::new();

            for &(edge_idx, ref next) in neighbors {
                if visited.contains(next) {
                    continue;
                }
                let edge = &self.edges[edge_idx];
                // 边权重参与激活计算 / Edge weight in activation
                let next_activation = activation * decay_rate * edge.weight;
                if next_activation >= 0.1 {
                    updates.push((edge_idx, next.clone(), next_activation));
                }
            }

            // 应用所有更新 / Apply all updates
            for (edge_idx, next, next_activation) in updates {
                if !visited.insert(next.clone()) {
                    continue;
                }

                // 更新边的激活统计 / Update edge activation stats
                self.edges[edge_idx].activation_count += 1;
                self.edges[edge_idx].last_activated = now;

                // 更新目标节点 / Update target node
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
    /// Find Top-K nodes most related to an entity.
    ///
    /// 性能：O(d)，邻接表直接取邻居。替代原 O(E) 全边扫描。
    /// Performance: O(d), adjacency list lookup. Replaces O(E) full edge scan.
    pub fn related(&self, entity: &str, top_k: usize) -> Vec<(String, f64)> {
        let mut results: Vec<(String, f64)> = Vec::new();
        let prefix_o = format!("O:{}", entity);
        let prefix_s = format!("S:{}", entity);

        // 邻接表直接取邻居 O(d) / Get neighbors from adjacency list O(d)
        for prefix in &[&prefix_o, &prefix_s] {
            if let Some(neighbors) = self.adjacency.get(*prefix) {
                for &(edge_idx, ref neighbor) in neighbors {
                    let weight = self.edges[edge_idx].weight;
                    results.push((neighbor.clone(), weight));
                }
            }
        }

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);
        results
    }

    /// 衰减边权重并清理弱边和孤立节点
    /// Decay edge weights and prune weak edges + orphan nodes.
    ///
    /// - `decay_factor`: 每条边 weight *= decay_factor (e.g. 0.995)
    /// - `min_weight`: weight < min_weight 的边被移除
    /// - 移除边后无连接的孤立节点也被清理
    /// - 衰减后全量重建邻接表和边ID索引 / Rebuilds indices after pruning
    pub fn decay_and_prune(&mut self, decay_factor: f64, min_weight: f64) {
        // 1. 衰减所有边权重 / Decay all edge weights
        for edge in self.edges.iter_mut() {
            edge.weight *= decay_factor;
        }
        // 2. 移除弱边 / Remove weak edges
        self.edges.retain(|e| e.weight >= min_weight);
        // 3. 移除孤立节点（无任何边连接） / Remove orphan nodes
        let connected: HashSet<String> = self
            .edges
            .iter()
            .flat_map(|e| vec![e.from.clone(), e.to.clone()])
            .collect();
        self.nodes.retain(|id, _| connected.contains(id));
        // 4. 重建索引（retain 后边索引已失效） / Rebuild indices (retain invalidates indices)
        self.rebuild_indices();
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

    // ── 新增能力 / New Capabilities ──

    /// 多种子扩散激活 / Multi-seed spreading activation
    ///
    /// 数字生命同时从多个记忆锚点出发扩散，
    /// 发现多个记忆域的交叉关联——模拟"联想多个事物"的认知过程。
    /// Digital life activates from multiple memory anchors simultaneously,
    /// discovering cross-domain associations — simulating "connecting multiple things".
    ///
    /// 对每个种子独立扩散，合并结果路径，按激活值排序。
    /// Spreads from each seed independently, merges paths, sorts by activation.
    pub fn spread_activation_multi(
        &mut self,
        seeds: &[&str],
        decay_rate: f64,
        max_hops: u32,
    ) -> Vec<ActivatedPath> {
        let mut all_paths = Vec::new();
        for &seed in seeds {
            let paths = self.spread_activation(seed, decay_rate, max_hops);
            all_paths.extend(paths);
        }
        all_paths.sort_by(|a, b| {
            b.activation
                .partial_cmp(&a.activation)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        all_paths
    }

    /// 关系过滤扩散激活 / Relation-filtered spreading activation
    ///
    /// 仅沿指定关系类型扩散——数字生命沿特定认知维度"思考"。
    /// 例如仅沿 Causes 关系追溯因果链，或仅沿 SimilarTo 探索相似网络。
    /// Spreads only along specified relation types — digital life "thinks"
    /// along specific cognitive dimensions.
    /// E.g. only Causes for causal chains, only SimilarTo for similarity networks.
    pub fn spread_activation_filtered(
        &mut self,
        seed: &str,
        decay_rate: f64,
        max_hops: u32,
        allowed_relations: &[RelationType],
    ) -> Vec<ActivatedPath> {
        let now = now_timestamp();
        let allowed_set: HashSet<&RelationType> = allowed_relations.iter().collect();

        // 重置所有节点激活值 / Reset all node activations
        for node in self.nodes.values_mut() {
            node.activation = 0.0;
        }

        let mut queue: VecDeque<(String, u32, f64)> = VecDeque::new();
        let mut visited: HashSet<String> = HashSet::new();
        let seed_id = format!("O:{}", seed);
        let subject_seed = format!("S:{}", seed);

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

            let neighbors = match self.adjacency.get(&current) {
                Some(n) => n,
                None => continue,
            };

            let mut updates: Vec<(usize, String, f64)> = Vec::new();

            for &(edge_idx, ref next) in neighbors {
                if visited.contains(next) {
                    continue;
                }
                let edge = &self.edges[edge_idx];
                // 关系过滤：仅沿允许的关系类型扩散 / Relation filter
                if !allowed_set.contains(&edge.relation) {
                    continue;
                }
                let next_activation = activation * decay_rate * edge.weight;
                if next_activation >= 0.1 {
                    updates.push((edge_idx, next.clone(), next_activation));
                }
            }

            for (edge_idx, next, next_activation) in updates {
                if !visited.insert(next.clone()) {
                    continue;
                }
                self.edges[edge_idx].activation_count += 1;
                self.edges[edge_idx].last_activated = now;

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

    /// 多种子共振扩散 / Multi-seed resonance spread
    ///
    /// 从多个种子同时扩散，检测激活波交汇的"共振节点"。
    /// 共振节点代表多个记忆域的交叉点——数字生命的"顿悟时刻"。
    /// Spreads from multiple seeds simultaneously, detecting "resonance nodes"
    /// where activation waves intersect — the digital life's "aha moment".
    ///
    /// 共振强度 = 各种子对该节点激活值之和 / Resonance strength = sum of activations.
    pub fn resonance_spread(
        &mut self,
        seeds: &[&str],
        decay_rate: f64,
        max_hops: u32,
    ) -> ResonanceReport {
        // 记录每个节点被哪些种子激活及其激活值 / Track which seeds activate each node
        // node_id → (seed_idx, activation)
        let mut node_activations: HashMap<String, Vec<(usize, f64)>> = HashMap::new();
        let mut all_paths = Vec::new();

        for (seed_idx, &seed) in seeds.iter().enumerate() {
            let paths = self.spread_activation(seed, decay_rate, max_hops);
            for path in &paths {
                node_activations
                    .entry(path.to.clone())
                    .or_default()
                    .push((seed_idx, path.activation));
            }
            all_paths.extend(paths);
        }

        // 检测共振节点：被 ≥2 个种子激活的节点 / Detect resonance: activated by ≥2 seeds
        let mut resonance_nodes = Vec::new();
        for (node_id, activations) in &node_activations {
            let unique_seeds: HashSet<usize> = activations.iter().map(|(s, _)| *s).collect();
            if unique_seeds.len() >= 2 {
                let activating_seeds: Vec<String> = unique_seeds
                    .iter()
                    .map(|&idx| seeds[idx].to_string())
                    .collect();
                let activation_values: Vec<f64> = activations.iter().map(|(_, a)| *a).collect();
                let resonance_strength: f64 = activation_values.iter().sum();
                resonance_nodes.push(ResonanceNode {
                    node_id: node_id.clone(),
                    activating_seeds,
                    activations: activation_values,
                    resonance_strength,
                });
            }
        }

        // 按共振强度排序 / Sort by resonance strength
        resonance_nodes.sort_by(|a, b| {
            b.resonance_strength
                .partial_cmp(&a.resonance_strength)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        all_paths.sort_by(|a, b| {
            b.activation
                .partial_cmp(&a.activation)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        ResonanceReport {
            resonance_nodes,
            paths: all_paths,
        }
    }
}

/// 激活路径 / Activation path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivatedPath {
    pub from: String,
    pub to: String,
    pub predicate: String,
    pub activation: f64,
    pub hops: u32,
}

/// 共振节点 / Resonance node
///
/// 被多个种子同时激活的节点——多个记忆域的交叉点。
/// 代表数字生命的"顿悟时刻"：不同记忆痕迹在此交汇、涌现。
/// A node activated by multiple seeds — an intersection of memory domains.
/// Represents the digital life's "aha moment": different memory traces converge here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResonanceNode {
    /// 节点 ID / Node ID
    pub node_id: String,
    /// 激活该节点的种子列表 / Seeds that activated this node
    pub activating_seeds: Vec<String>,
    /// 各种子对该节点的激活值 / Activation values from each seed
    pub activations: Vec<f64>,
    /// 共振强度 = 激活值之和 / Resonance strength = sum of activations
    pub resonance_strength: f64,
}

/// 共振检测报告 / Resonance detection report
///
/// 多种子扩散激活的结果，包含共振节点和所有激活路径。
/// Result of multi-seed spreading activation, containing resonance nodes and all paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResonanceReport {
    /// 共振节点（被 ≥2 个种子激活） / Resonance nodes (activated by ≥2 seeds)
    pub resonance_nodes: Vec<ResonanceNode>,
    /// 所有激活路径（合并，按激活值排序） / All activation paths (merged, sorted by activation)
    pub paths: Vec<ActivatedPath>,
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

    // ── 索引正确性测试 / Index Correctness Tests ──

    #[test]
    fn test_adjacency_index_correctness() {
        // 邻接表内容应与边表一致 / Adjacency list should match edge table
        let mut g = AssociativeGraph::new();
        g.add_fact(&Fact::new("A", "关联", "B").with_confidence(0.8));
        g.add_fact(&Fact::new("B", "关联", "C").with_confidence(0.7));
        g.link("O:B", "O:C", RelationType::CoOccurs, 0.5);

        // 验证：每条边在邻接表中产生两个方向条目 / Each edge → two adjacency entries
        for (idx, edge) in g.edges().iter().enumerate() {
            // from 方向 / outgoing
            let from_neighbors = g.adjacency.get(&edge.from).expect("from 应在邻接表中");
            assert!(
                from_neighbors
                    .iter()
                    .any(|(e_idx, n)| *e_idx == idx && n == &edge.to),
                "边 {} 的 from→to 应在邻接表中",
                edge.id
            );
            // to 方向 / incoming
            let to_neighbors = g.adjacency.get(&edge.to).expect("to 应在邻接表中");
            assert!(
                to_neighbors
                    .iter()
                    .any(|(e_idx, n)| *e_idx == idx && n == &edge.from),
                "边 {} 的 to→from 应在邻接表中",
                edge.id
            );
        }
    }

    #[test]
    fn test_edge_index_correctness() {
        // 边ID索引应与边表一致 / Edge ID index should match edge table
        let mut g = AssociativeGraph::new();
        g.add_fact(&Fact::new("X", "关联", "Y").with_confidence(0.9));
        g.add_fact(&Fact::new("Y", "关联", "Z").with_confidence(0.8));

        for (idx, edge) in g.edges().iter().enumerate() {
            let stored_idx = *g.edge_index.get(&edge.id).expect("边ID应在索引中");
            assert_eq!(stored_idx, idx, "边ID索引应指向正确的边");
        }
    }

    #[test]
    fn test_rebuild_after_prune() {
        // decay_and_prune 后索引应正确 / Indices correct after prune
        let mut g = AssociativeGraph::new();
        g.add_fact(&Fact::new("A", "强关联", "B").with_confidence(1.0));
        g.add_fact(&Fact::new("C", "弱关联", "D").with_confidence(0.1));

        g.decay_and_prune(0.5, 0.1);

        // 弱边应被移除，索引不应包含它 / Weak edge removed, index should not contain it
        assert_eq!(g.edge_count(), 1);
        assert_eq!(g.edge_index.len(), 1);

        // 邻接表不应包含已清理的节点 / Adjacency should not contain pruned nodes
        assert!(!g.adjacency.contains_key("S:C"));
        assert!(!g.adjacency.contains_key("O:D"));
        // 应保留强边节点 / Should retain strong edge nodes
        assert!(g.adjacency.contains_key("S:A"));
        assert!(g.adjacency.contains_key("O:B"));
    }

    #[test]
    fn test_rebuild_from_parts() {
        // from_parts 后索引应正确 / Indices correct after from_parts
        let mut g = AssociativeGraph::new();
        g.add_fact(&Fact::new("A", "关联", "B").with_confidence(0.8));
        g.add_fact(&Fact::new("B", "关联", "C").with_confidence(0.7));

        let nodes = g.nodes().clone();
        let edges = g.edges().to_vec();
        let g2 = AssociativeGraph::from_parts(nodes, edges);

        // 验证索引重建 / Verify index rebuild
        assert_eq!(g2.edge_index.len(), g2.edge_count());
        // 验证邻接表 / Verify adjacency
        assert!(g2.adjacency.contains_key("S:A"));
        assert!(g2.adjacency.contains_key("O:B"));
        assert!(g2.adjacency.contains_key("O:C"));
    }

    // ── 新能力测试 / New Capability Tests ──

    #[test]
    fn test_multi_seed_activation() {
        // 多种子扩散应覆盖各种子的邻居 / Multi-seed covers each seed's neighbors
        let mut g = AssociativeGraph::new();
        g.add_fact(&Fact::new("主人", "喜欢", "Rust").with_confidence(0.9));
        g.add_fact(&Fact::new("主人", "学习", "AI").with_confidence(0.8));
        g.add_fact(&Fact::new("小明", "喜欢", "Go").with_confidence(0.7));

        let paths = g.spread_activation_multi(&["主人", "小明"], 0.5, 2);

        // 应有从"主人"和"小明"扩散的路径 / Should have paths from both seeds
        assert!(!paths.is_empty(), "多种子扩散应有结果");
        // 应包含 Rust（从主人扩散）和 Go（从小明扩散）
        let targets: Vec<&String> = paths.iter().map(|p| &p.to).collect();
        assert!(
            targets.iter().any(|t| t == &"O:Rust"),
            "应包含从主人扩散的 Rust"
        );
        assert!(
            targets.iter().any(|t| t == &"O:Go"),
            "应包含从小明扩散的 Go"
        );
    }

    #[test]
    fn test_filtered_activation() {
        // 关系过滤仅沿允许类型扩散 / Filter only spreads along allowed relations
        let mut g = AssociativeGraph::new();
        g.add_fact(&Fact::new("A", "关联", "B").with_confidence(0.9)); // SubjectObject: S:A → O:B
        g.link("S:A", "O:B", RelationType::CoOccurs, 0.8); // CoOccurs: S:A ↔ O:B

        // 仅允许 CoOccurs / Only allow CoOccurs
        let paths_co = g.spread_activation_filtered("A", 0.5, 3, &[RelationType::CoOccurs]);

        // 仅允许 SubjectObject / Only allow SubjectObject
        let paths_so = g.spread_activation_filtered("A", 0.5, 3, &[RelationType::SubjectObject]);

        // 两种过滤都应有结果（S:A 有两种关系的边到 O:B） / Both should have results
        // CoOccurs 边: S:A ↔ O:B
        assert!(!paths_co.is_empty(), "CoOccurs 过滤应有结果");
        // SubjectObject 边: S:A → O:B
        assert!(!paths_so.is_empty(), "SubjectObject 过滤应有结果");
    }

    #[test]
    fn test_filtered_activation_empty_relations() {
        // 空关系列表 = 无扩散 / Empty relations = no spread
        let mut g = AssociativeGraph::new();
        g.add_fact(&Fact::new("A", "关联", "B").with_confidence(0.9));

        let paths = g.spread_activation_filtered("A", 0.5, 3, &[]);
        assert!(paths.is_empty(), "空关系列表不应产生扩散");
    }

    #[test]
    fn test_resonance_basic() {
        // 两个种子的共同邻居应被识别为共振节点 / Common neighbor = resonance node
        let mut g = AssociativeGraph::new();
        // A → C, B → C，C 是 A 和 B 的共同邻居
        g.add_fact(&Fact::new("A", "关联", "C").with_confidence(0.9));
        g.add_fact(&Fact::new("B", "关联", "C").with_confidence(0.8));

        let report = g.resonance_spread(&["A", "B"], 0.5, 3);

        // 应有共振节点 / Should have resonance nodes
        assert!(
            !report.resonance_nodes.is_empty(),
            "应有共振节点（A 和 B 的共同邻居 C）"
        );
        // 共振节点应包含 O:C / Resonance should include O:C
        let has_c = report.resonance_nodes.iter().any(|rn| rn.node_id == "O:C");
        assert!(has_c, "O:C 应为共振节点");
    }

    #[test]
    fn test_resonance_no_overlap() {
        // 无交集时共振节点为空 / No overlap → no resonance
        let mut g = AssociativeGraph::new();
        g.add_fact(&Fact::new("A", "关联", "B").with_confidence(0.9));
        g.add_fact(&Fact::new("C", "关联", "D").with_confidence(0.8));

        let report = g.resonance_spread(&["A", "C"], 0.5, 2);

        // A 和 C 的邻居不重叠（B ≠ D） / No common neighbors
        // 注意：P:关联 是共享的 Concept 节点，可能产生共振
        // 验证共振节点中不包含 O:B 或 O:D / Verify no O:B or O:D in resonance
        let non_resonance = report
            .resonance_nodes
            .iter()
            .filter(|rn| rn.node_id == "O:B" || rn.node_id == "O:D")
            .count();
        assert_eq!(non_resonance, 0, "O:B 和 O:D 不应是共振节点");
    }

    #[test]
    fn test_resonance_strength() {
        // 共振强度 = 各种子激活值之和 / Strength = sum of activations
        let mut g = AssociativeGraph::new();
        g.add_fact(&Fact::new("A", "关联", "C").with_confidence(1.0));
        g.add_fact(&Fact::new("B", "关联", "C").with_confidence(1.0));

        let report = g.resonance_spread(&["A", "B"], 0.5, 2);

        let c_resonance = report
            .resonance_nodes
            .iter()
            .find(|rn| rn.node_id == "O:C")
            .expect("O:C 应为共振节点");

        // 两个种子各贡献 0.5 的激活值（1.0 * 0.5 * 1.0） / Each seed contributes 0.5
        assert!(
            c_resonance.resonance_strength > 0.0,
            "共振强度应 > 0, got {}",
            c_resonance.resonance_strength
        );
        assert_eq!(c_resonance.activating_seeds.len(), 2, "应由 2 个种子激活");
    }

    #[test]
    fn test_resonance_three_seeds() {
        // 三种子交汇点共振强度更高 / Three seeds → higher resonance
        let mut g = AssociativeGraph::new();
        g.add_fact(&Fact::new("A", "关联", "X").with_confidence(1.0));
        g.add_fact(&Fact::new("B", "关联", "X").with_confidence(1.0));
        g.add_fact(&Fact::new("C", "关联", "X").with_confidence(1.0));

        let report = g.resonance_spread(&["A", "B", "C"], 0.5, 2);

        let x_resonance = report
            .resonance_nodes
            .iter()
            .find(|rn| rn.node_id == "O:X")
            .expect("O:X 应为共振节点");

        assert_eq!(x_resonance.activating_seeds.len(), 3, "应由 3 个种子激活");
        // 三种子的共振强度应 > 两种子 / 3-seed resonance > 2-seed
        assert!(
            x_resonance.resonance_strength > 0.5,
            "三种子共振强度应 > 0.5, got {}",
            x_resonance.resonance_strength
        );
    }
}
