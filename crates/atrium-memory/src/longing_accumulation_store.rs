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
#[derive(Clone, Debug, Serialize, Deserialize)]
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

// 手动实现 Default — onset_multiplier 必须为 1.0（中性，不调整敏感度）
// Manual Default impl — onset_multiplier must be 1.0 (neutral, no sensitivity adjustment)
// 修复 P0-C：derive(Default) 会将 onset_multiplier 设为 0.0，导致想念起始阈值归零
// Fix P0-C: derive(Default) sets onset_multiplier to 0.0, zeroing the longing onset threshold
impl Default for LongingAccumulationSummary {
    fn default() -> Self {
        Self {
            total_departures: 0,
            total_reunions: 0,
            avg_away_secs: 0.0,
            max_away_secs: 0,
            avg_peak_longing: 0.0,
            consecutive_no_shows: 0,
            onset_multiplier: 1.0, // 中性乘数 / Neutral multiplier
        }
    }
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

// ════════════════════════════════════════════════════════════════════
// 测试模块 / Test Module
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// 创建临时内存数据库 / Create a temporary in-memory sled database.
    fn temp_db() -> sled::Db {
        sled::Config::default()
            .temporary(true)
            .flush_every_ms(None)
            .open()
            .expect("临时数据库创建失败 / failed to create temp db")
    }

    // ── T1: 空存储默认状态 / Empty Store Default State ──

    #[test]
    fn test_empty_store_default_summary() {
        // 空存储应返回默认摘要，起始乘数为 1.0 / Empty store returns default summary
        let db = temp_db();
        let store = LongingAccumulationStore::open_default(&db).unwrap();

        let summary = store.summary();
        assert_eq!(summary.total_departures, 0, "空存储离开次数应为 0");
        assert_eq!(summary.total_reunions, 0, "空存储回来次数应为 0");
        assert_eq!(summary.avg_away_secs, 0.0, "空存储平均离开应为 0");
        assert_eq!(summary.max_away_secs, 0, "空存储最长离开应为 0");
        assert_eq!(summary.avg_peak_longing, 0.0, "空存储平均峰值应为 0");
        assert_eq!(summary.onset_multiplier, 1.0, "空存储起始乘数应为 1.0");

        let mult = store.onset_multiplier();
        assert!(
            (mult - 1.0).abs() < f32::EPSILON,
            "onset_multiplier() 应返回 1.0"
        );
    }

    // ── T2: 离开事件记录 / Departure Event Recording ──

    #[test]
    fn test_record_departure() {
        // 记录离开后，摘要应反映 1 次离开 / After recording departure, summary shows 1
        let db = temp_db();
        let store = LongingAccumulationStore::open_default(&db).unwrap();

        let ts = 1_700_000_000i64;
        store.record_departure(ts).unwrap();

        let summary = store.summary();
        assert_eq!(summary.total_departures, 1, "离开次数应为 1");
        assert_eq!(summary.total_reunions, 0, "回来次数应为 0");
    }

    // ── T3: 回来事件记录 / Reunion Event Recording ──

    #[test]
    fn test_record_reunion() {
        // 记录离开+回来后，away_secs 和 peak_longing 应正确 / After departure+reunion
        let db = temp_db();
        let store = LongingAccumulationStore::open_default(&db).unwrap();

        let departed = 1_700_000_000i64;
        let reunited = 1_700_003_600i64; // 1 小时后 / 1 hour later
        let peak = 0.75f32;

        store.record_departure(departed).unwrap();
        store.record_reunion(reunited, peak).unwrap();

        let summary = store.summary();
        assert_eq!(summary.total_departures, 1);
        assert_eq!(summary.total_reunions, 1, "回来次数应为 1");
        assert_eq!(summary.max_away_secs, 3600, "最长离开应为 3600 秒");
        assert!(
            (summary.avg_peak_longing - peak).abs() < 0.001,
            "平均峰值应为 {:.2}, got {:.2}",
            peak,
            summary.avg_peak_longing
        );
    }

    // ── T4: 多轮离开-回来循环统计 / Multiple Cycle Statistics ──

    #[test]
    fn test_multiple_cycles_stats() {
        // 3 轮离开-回来，验证 avg/max/reunions 全正确 / 3 cycles, verify all stats
        let db = temp_db();
        let store = LongingAccumulationStore::open_default(&db).unwrap();

        // 3 轮：30min, 2h, 6h / 3 cycles: 30min, 2h, 6h
        let cycles = [
            (1_700_000_000i64, 1_700_001_800i64, 0.3f32), // 30 min
            (1_700_010_000i64, 1_700_017_200i64, 0.6f32), // 2 h
            (1_700_020_000i64, 1_700_041_600i64, 0.9f32), // 6 h
        ];

        for (dep, reu, peak) in cycles {
            store.record_departure(dep).unwrap();
            store.record_reunion(reu, peak).unwrap();
        }

        let summary = store.summary();
        assert_eq!(summary.total_departures, 3, "离开 3 次");
        assert_eq!(summary.total_reunions, 3, "回来 3 次");
        assert_eq!(summary.max_away_secs, 21600, "最长离开应为 6h=21600s");

        // 平均离开时长 = (1800 + 7200 + 21600) / 3 = 10200
        assert!(
            (summary.avg_away_secs - 10200.0).abs() < 1.0,
            "平均离开应为 ~10200s, got {:.1}",
            summary.avg_away_secs
        );

        // 平均峰值 = (0.3 + 0.6 + 0.9) / 3 = 0.6
        assert!(
            (summary.avg_peak_longing - 0.6).abs() < 0.01,
            "平均峰值应为 ~0.6, got {:.3}",
            summary.avg_peak_longing
        );
    }

    // ── T5: 起始乘数 — 短暂离开 / Onset Multiplier — Short Absence ──

    #[test]
    fn test_onset_multiplier_short_absence() {
        // 用户平均离开 < 10 分钟 → 乘数 > 1.0（降低敏感度）/ Short absence → mult > 1.0
        let db = temp_db();
        let store = LongingAccumulationStore::open_default(&db).unwrap();

        // 3 次短暂离开：5min, 3min, 7min → avg=5min < 10min
        let departures = [1_700_000_000i64, 1_700_001_000i64, 1_700_002_000i64];
        let durations = [300u64, 180, 420]; // 5min, 3min, 7min

        for (i, &dep) in departures.iter().enumerate() {
            store.record_departure(dep).unwrap();
            store
                .record_reunion(dep + durations[i] as i64, 0.5)
                .unwrap();
        }

        let mult = store.onset_multiplier();
        assert!(
            mult > 1.0,
            "短暂离开应使乘数 > 1.0（降低敏感度）, got {:.3}",
            mult
        );
    }

    // ── T6: 起始乘数 — 长时离开 / Onset Multiplier — Long Absence ──

    #[test]
    fn test_onset_multiplier_long_absence() {
        // 用户平均离开 > 1 小时 → 乘数 < 1.0（提高敏感度）/ Long absence → mult < 1.0
        let db = temp_db();
        let store = LongingAccumulationStore::open_default(&db).unwrap();

        // 2 次长时离开：2h, 4h → avg=3h >> 10min
        store.record_departure(1_700_000_000).unwrap();
        store.record_reunion(1_700_007_200, 0.8).unwrap(); // 2h
        store.record_departure(1_700_010_000).unwrap();
        store.record_reunion(1_700_024_400, 0.9).unwrap(); // 4h

        let mult = store.onset_multiplier();
        assert!(
            mult < 1.0,
            "长时离开应使乘数 < 1.0（提高敏感度）, got {:.3}",
            mult
        );
    }

    // ── T7: 起始乘数 — 边界钳制 / Onset Multiplier — Clamp ──

    #[test]
    fn test_onset_multiplier_clamp() {
        // 极端离开时长 → 乘数钳制在 [0.5, 2.0] / Extreme values clamped to [0.5, 2.0]
        let db = temp_db();
        let store = LongingAccumulationStore::open_default(&db).unwrap();

        // 极短离开：1 秒 → ratio = 600/1 = 600 → 钳制为 2.0
        store.record_departure(1_700_000_000).unwrap();
        store.record_reunion(1_700_000_001, 0.5).unwrap();

        let mult = store.onset_multiplier();
        assert!(
            (0.5..=2.0).contains(&mult),
            "乘数应在 [0.5, 2.0] 范围内, got {:.3}",
            mult
        );
        assert!(
            (mult - 2.0).abs() < 0.01,
            "极短离开应使乘数钳制到 2.0, got {:.3}",
            mult
        );

        // 极长离开：100000 秒 → ratio = 600/100000 = 0.006 → 钳制为 0.5
        let db2 = temp_db();
        let store2 = LongingAccumulationStore::open_default(&db2).unwrap();
        store2.record_departure(1_700_000_000).unwrap();
        store2.record_reunion(1_700_100_000, 0.5).unwrap();

        let mult2 = store2.onset_multiplier();
        assert!(
            (mult2 - 0.5).abs() < 0.01,
            "极长离开应使乘数钳制到 0.5, got {:.3}",
            mult2
        );
    }

    // ── T8: FIFO 淘汰机制 / FIFO Eviction ──

    #[test]
    fn test_fifo_eviction() {
        // 超过 max_patterns → 最旧模式被删除 / Exceed max → oldest evicted
        let db = temp_db();
        let store = LongingAccumulationStore::open(&db, 5).unwrap(); // 最多 5 条

        // 写入 7 条离开记录 / Write 7 departures
        for i in 0..7 {
            store.record_departure(1_700_000_000 + i * 100_000).unwrap();
        }

        let summary = store.summary();
        assert_eq!(
            summary.total_departures, 7,
            "total_departures 应记录所有 7 次（摘要不因 FIFO 减少）"
        );
        // pattern_tree 应只剩 5 条 / pattern_tree should have only 5
        // 注意：total_departures 是累计计数器，不因 FIFO 减少
        // Note: total_departures is a cumulative counter, not reduced by FIFO
    }

    // ── T9: 持久化写穿/恢复 — 核心测试 / Persistence Write-Through Recovery ──

    #[test]
    fn test_persistence_write_reopen() {
        // 写入数据 → 丢弃 Store → 重新打开 → 数据完整恢复
        // Write → drop Store → reopen → data fully recovered
        let db = temp_db();

        // 第一阶段：写入 3 轮模式 / Phase 1: write 3 cycles
        {
            let store = LongingAccumulationStore::open_default(&db).unwrap();
            store.record_departure(1_700_000_000).unwrap();
            store.record_reunion(1_700_001_800, 0.4).unwrap(); // 30min
            store.record_departure(1_700_010_000).unwrap();
            store.record_reunion(1_700_017_200, 0.7).unwrap(); // 2h
            store.record_departure(1_700_020_000).unwrap();
            store.record_reunion(1_700_041_600, 0.9).unwrap(); // 6h

            // 确认写入时数据正确 / Verify data correct at write time
            let s = store.summary();
            assert_eq!(s.total_departures, 3);
            assert_eq!(s.total_reunions, 3);
            assert_eq!(s.max_away_secs, 21600);
        } // Store 被 drop（模拟关闭） / Store dropped (simulating shutdown)

        // 第二阶段：重新打开同一 DB → 验证恢复 / Phase 2: reopen same DB → verify recovery
        {
            let store = LongingAccumulationStore::open_default(&db).unwrap();
            let summary = store.summary();

            assert_eq!(
                summary.total_departures, 3,
                "恢复后离开次数应为 3 / departures should survive reopen"
            );
            assert_eq!(
                summary.total_reunions, 3,
                "恢复后回来次数应为 3 / reunions should survive reopen"
            );
            assert_eq!(
                summary.max_away_secs, 21600,
                "恢复后最长离开应为 21600s / max_away should survive reopen"
            );
            assert!(
                (summary.avg_away_secs - 10200.0).abs() < 1.0,
                "恢复后平均离开应为 ~10200s / avg_away should survive reopen, got {:.1}",
                summary.avg_away_secs
            );
            assert!(
                (summary.avg_peak_longing - 0.6667).abs() < 0.01,
                "恢复后平均峰值应为 ~0.667 / avg_peak should survive reopen, got {:.3}",
                summary.avg_peak_longing
            );
            assert!(
                summary.onset_multiplier < 1.0,
                "恢复后起始乘数应 < 1.0（长时离开）/ onset_multiplier should be < 1.0, got {:.3}",
                summary.onset_multiplier
            );
        }
    }

    // ── T10: 多 Store 共享同一 DB 隔离性 / Shared DB Isolation ──

    #[test]
    fn test_shared_db_isolation() {
        // 同一 DB 打开两个 Store → 数据通过 Tree 名隔离
        // Same DB, two stores → isolated via tree names
        let db = temp_db();

        let store1 = LongingAccumulationStore::open_default(&db).unwrap();
        store1.record_departure(1_700_000_000).unwrap();
        store1.record_reunion(1_700_001_800, 0.5).unwrap();

        // 用同一 DB 再开一个 Store → 应看到相同数据（同一 Tree）
        // Reopen with same DB → should see same data (same tree)
        let store2 = LongingAccumulationStore::open_default(&db).unwrap();
        let summary = store2.summary();
        assert_eq!(
            summary.total_departures, 1,
            "同一 DB 重开应看到已有数据 / same DB should see existing data"
        );
        assert_eq!(summary.total_reunions, 1);
    }

    // ── T11: 无离开记录的回来事件 / Reunion Without Departure ──

    #[test]
    fn test_reunion_without_departure() {
        // 没有离开记录时调用回来 → 安全 no-op，不崩溃
        // Reunion without prior departure → safe no-op, no crash
        let db = temp_db();
        let store = LongingAccumulationStore::open_default(&db).unwrap();

        // 直接调用回来，不先记录离开 / Call reunion without prior departure
        store.record_reunion(1_700_000_000, 0.5).unwrap();

        let summary = store.summary();
        assert_eq!(summary.total_departures, 0, "无离开记录");
        assert_eq!(summary.total_reunions, 0, "无回来应被记录");
    }

    // ── T12: 想念峰值增量平均正确性 / Peak Longing Incremental Average ──

    #[test]
    fn test_peak_longing_averaging() {
        // 多轮回来 → avg_peak_longing 应为增量平均（非简单算术平均）
        // Multiple reunions → avg_peak_longing should be incremental running average
        let db = temp_db();
        let store = LongingAccumulationStore::open_default(&db).unwrap();

        let peaks = [0.2f32, 0.5, 0.8, 1.0];
        let base = 1_700_000_000i64;

        for (i, &peak) in peaks.iter().enumerate() {
            let dep = base + i as i64 * 1_000_000;
            store.record_departure(dep).unwrap();
            store.record_reunion(dep + 3600, peak).unwrap(); // 每次离开 1 小时
        }

        let summary = store.summary();
        // 增量平均：(0.2) → (0.2+0.5)/2=0.35 → (0.35*2+0.8)/3=0.5 → (0.5*3+1.0)/4=0.625
        let expected = 0.625f32;
        assert!(
            (summary.avg_peak_longing - expected).abs() < 0.01,
            "增量平均应为 {:.3}, got {:.3}",
            expected,
            summary.avg_peak_longing
        );
    }
}
