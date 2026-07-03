// SPDX-License-Identifier: MIT
//! 想念累积存储 — sled 命名 Tree 持久化的跨会话想念模式
//! Longing accumulation store — sled named tree-backed cross-session longing patterns.
//!
//! G4: 跨会话想念累积——记录用户离开/回来模式，为想念引擎提供历史先验。
//! G4: Cross-session longing accumulation — records user departure/reunion patterns,
//! providing historical priors for the longing engine.
//!
//! 归属认知域：关系海马体（Relational）。
//! Cognitive domain: Relational.

use serde::{Deserialize, Serialize};

// ════════════════════════════════════════════════════════════════════
// DeparturePattern — 离开模式 / Departure Pattern
// ════════════════════════════════════════════════════════════════════

/// 离开模式 — 描述用户一次离开-回来循环 / Departure pattern
///
/// 数字生命通过累积离开模式，学习用户的"缺席节奏"——
/// 某些人习惯性离开2小时，某些人一走就是3天。
/// Digital life learns the user's "absence rhythm" by accumulating departure patterns —
/// some habitually leave for 2 hours, some disappear for 3 days.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeparturePattern {
    /// 离开时长（秒）/ Away duration in seconds
    pub away_secs: u64,
    /// 离开时间戳 / Departure timestamp
    pub departed_at: i64,
    /// 回来时间戳（0=尚未回来）/ Reunion timestamp (0 = not yet returned)
    pub reunited_at: i64,
    /// 此次想念峰值 / Peak longing intensity during this absence
    pub peak_longing: f32,
}

// ════════════════════════════════════════════════════════════════════
// LongingAccumulationSummary — 累积摘要 / Accumulation Summary
// ════════════════════════════════════════════════════════════════════

/// 想念累积摘要 — 跨会话统计 / Longing accumulation summary
///
/// 从历史离开模式中提取的统计摘要，供想念引擎使用。
/// Statistical summary extracted from historical departure patterns for the longing engine.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct LongingAccumulationSummary {
    /// 总离开次数 / Total departure count
    pub total_departures: u32,
    /// 总回来次数 / Total reunion count
    pub total_reunions: u32,
    /// 平均离开时长（秒）/ Average away duration in seconds
    pub avg_away_secs: f64,
    /// 最长离开时长（秒）/ Maximum away duration in seconds
    pub max_away_secs: u64,
    /// 平均想念峰值 / Average peak longing intensity
    pub avg_peak_longing: f32,
    /// 连续失约次数（离开但未在预期时间内回来）/ Consecutive no-shows
    pub consecutive_no_shows: u32,
    /// 起始乘数 — 基于历史模式调整想念起始阈值 / Onset multiplier
    /// \>1.0: 用户经常短暂离开，降低想念敏感度；\<1.0: 用户经常长时间离开，提高敏感度
    pub onset_multiplier: f32,
}

// ════════════════════════════════════════════════════════════════════
// LongingAccumulationStore — sled 持久化 / Longing Accumulation Store
// ════════════════════════════════════════════════════════════════════

/// 想念累积存储 — sled 命名 Tree + bincode 序列化
/// Longing accumulation store — sled named tree + bincode serialization.
///
/// # Tree 结构 / Tree Structure
///
/// | Tree 名 | 键 | 值 | 用途 |
/// |---------|-----|-----|------|
/// | `longing_patterns` | `pattern/{departed_at:020}` | DeparturePattern | 离开模式记录 |
/// | `longing_summary` | `summary` | LongingAccumulationSummary | 累积摘要 |
///
/// 最多保留最近 200 条离开模式（FIFO），超出时删除最旧的。
/// Keeps at most 200 recent departure patterns (FIFO), deleting oldest when exceeded.
pub struct LongingAccumulationStore {
    /// 离开模式 Tree / Departure pattern tree
    pattern_tree: sled::Tree,
    /// 累积摘要 Tree / Accumulation summary tree
    summary_tree: sled::Tree,
    /// 最大保留模式数 / Maximum retained pattern count
    max_patterns: usize,
}

impl LongingAccumulationStore {
    /// 从共享数据库打开想念累积存储 / Open longing accumulation store from shared database.
    ///
    /// # 参数 / Parameters
    ///
    /// - `db`: 共享 sled 数据库实例 / Shared sled database instance
    /// - `max_patterns`: 最大保留模式数（默认 200）/ Max retained patterns (default 200)
    pub fn open(db: &sled::Db, max_patterns: usize) -> Result<Self, sled::Error> {
        let pattern_tree = db.open_tree("longing_patterns")?;
        let summary_tree = db.open_tree("longing_summary")?;
        Ok(Self {
            pattern_tree,
            summary_tree,
            max_patterns,
        })
    }

    /// 使用默认参数打开 / Open with default parameters.
    pub fn open_default(db: &sled::Db) -> Result<Self, sled::Error> {
        Self::open(db, 200)
    }

    // ── 写入操作 / Write Operations ──

    /// 记录离开事件 / Record a departure event.
    ///
    /// O(1) 写入 + O(1) 摘要更新。
    pub fn record_departure(&self, departed_at: i64) -> Result<(), String> {
        let pattern = DeparturePattern {
            away_secs: 0,
            departed_at,
            reunited_at: 0,
            peak_longing: 0.0,
        };
        let key = format!("pattern/{:020}", departed_at);
        let val = bincode::serialize(&pattern).map_err(|e| e.to_string())?;
        self.pattern_tree
            .insert(key.as_bytes(), val.as_slice())
            .map_err(|e| e.to_string())?;
        // 更新摘要中的 total_departures / Update summary total_departures
        let mut summary = self.load_summary();
        summary.total_departures += 1;
        self.save_summary(&summary)?;
        // FIFO 淘汰 / FIFO eviction
        self.evict_if_needed()?;
        Ok(())
    }

    /// 记录回来事件 / Record a reunion event.
    ///
    /// O(K) 扫描未回来模式（K 通常 <10）+ O(1) 摘要更新。
    pub fn record_reunion(&self, reunited_at: i64, peak_longing: f32) -> Result<(), String> {
        // 查找最近的未回来模式 / Find most recent unreunited pattern
        let mut latest_key: Option<Vec<u8>> = None;
        let mut latest_pattern: Option<DeparturePattern> = None;

        for item in self.pattern_tree.iter() {
            let (k, v) = item.map_err(|e| e.to_string())?;
            let pattern: DeparturePattern = bincode::deserialize(&v).map_err(|e| e.to_string())?;
            if pattern.reunited_at == 0 {
                latest_key = Some(k.to_vec());
                latest_pattern = Some(pattern);
            }
        }

        if let (Some(key), Some(mut pattern)) = (latest_key, latest_pattern) {
            pattern.reunited_at = reunited_at;
            pattern.away_secs = (reunited_at - pattern.departed_at).max(0) as u64;
            pattern.peak_longing = peak_longing;
            let val = bincode::serialize(&pattern).map_err(|e| e.to_string())?;
            self.pattern_tree
                .insert(&key, val.as_slice())
                .map_err(|e| e.to_string())?;

            // 更新摘要 / Update summary
            let mut summary = self.load_summary();
            summary.total_reunions += 1;
            summary.avg_peak_longing = if summary.total_reunions > 0 {
                (summary.avg_peak_longing * (summary.total_reunions - 1) as f32 + peak_longing)
                    / summary.total_reunions as f32
            } else {
                peak_longing
            };
            if pattern.away_secs > summary.max_away_secs {
                summary.max_away_secs = pattern.away_secs;
            }
            // 重新计算平均离开时长和起始乘数 / Recompute avg and onset multiplier
            self.recompute_stats(&mut summary)?;
            self.save_summary(&summary)?;
        }

        Ok(())
    }

    // ── 读取操作 / Read Operations ──

    /// 获取累积摘要 / Get accumulation summary.
    ///
    /// O(1) — 直接从摘要 Tree 读取。
    pub fn summary(&self) -> LongingAccumulationSummary {
        self.load_summary()
    }

    /// 获取起始乘数 / Get onset multiplier for longing engine.
    ///
    /// O(1) — 从摘要中直接读取。
    ///
    /// \>1.0: 用户经常短暂离开，降低想念敏感度
    /// \<1.0: 用户经常长时间离开，提高想念敏感度
    pub fn onset_multiplier(&self) -> f32 {
        self.load_summary().onset_multiplier
    }

    // ── 内部方法 / Internal Methods ──

    /// 加载摘要 / Load summary from tree.
    fn load_summary(&self) -> LongingAccumulationSummary {
        match self.summary_tree.get(b"summary") {
            Ok(Some(v)) => bincode::deserialize(&v).unwrap_or_default(),
            _ => LongingAccumulationSummary::default(),
        }
    }

    /// 保存摘要 / Save summary to tree.
    fn save_summary(&self, summary: &LongingAccumulationSummary) -> Result<(), String> {
        let val = bincode::serialize(summary).map_err(|e| e.to_string())?;
        self.summary_tree
            .insert(b"summary", val.as_slice())
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// FIFO 淘汰超出上限的旧模式 / Evict oldest patterns if exceeding max_patterns.
    fn evict_if_needed(&self) -> Result<(), String> {
        let count = self.pattern_tree.len();
        if count <= self.max_patterns {
            return Ok(());
        }
        // 删除最旧的 K 条 / Delete oldest K entries
        let to_remove = count - self.max_patterns;
        for (removed, item) in self.pattern_tree.iter().enumerate() {
            if removed >= to_remove {
                break;
            }
            let (k, _) = item.map_err(|e| e.to_string())?;
            self.pattern_tree.remove(&k).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// 重新计算统计量 / Recompute statistics from all patterns.
    ///
    /// O(P) — P=模式数，通常 <200。
    fn recompute_stats(&self, summary: &mut LongingAccumulationSummary) -> Result<(), String> {
        let mut total_away: f64 = 0.0;
        let mut count: u32 = 0;

        for item in self.pattern_tree.iter() {
            let (_, v) = item.map_err(|e| e.to_string())?;
            let pattern: DeparturePattern = bincode::deserialize(&v).map_err(|e| e.to_string())?;
            if pattern.reunited_at > 0 {
                total_away += pattern.away_secs as f64;
                count += 1;
            }
        }

        if count > 0 {
            summary.avg_away_secs = total_away / count as f64;
        }

        // 起始乘数计算 / Onset multiplier calculation
        // 基准：平均离开时长 vs 默认起始阈值（600秒=10分钟）
        // 如果用户平均离开 < 10分钟 → 乘数 > 1.0（降低敏感度，避免频繁想念）
        // 如果用户平均离开 > 1小时 → 乘数 < 1.0（提高敏感度，用户长时间离开更值得想念）
        if summary.avg_away_secs > 0.0 {
            let baseline_secs = 600.0; // 10 分钟基准 / 10-minute baseline
            let ratio = baseline_secs / summary.avg_away_secs;
            // 限制在 [0.5, 2.0] 范围内 / Clamp to [0.5, 2.0]
            summary.onset_multiplier = ratio.clamp(0.5, 2.0) as f32;
        } else {
            summary.onset_multiplier = 1.0;
        }

        Ok(())
    }
}
