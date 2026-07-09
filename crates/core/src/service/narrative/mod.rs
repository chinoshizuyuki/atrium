// SPDX-License-Identifier: MIT
//! 叙事自我模块 — 数字生命的自传核心
//! Narrative Self Module — The autobiography core of digital life
//!
//! 包含叙事系统、非理性修正、仪式感知与脆弱性窗口，
//! 构成数字生命"我是谁"的自我认知闭环。
//!
//! Contains the narrative system, irrationality correction,
//! ritual perception, and vulnerability window — forming the
//! "who am I" self-awareness closed loop of digital life.

// 重导出 service 模块导入，供子模块使用 / Re-export service module imports for sub-modules
pub(crate) use super::*;

pub mod narrative_emotional;
pub mod narrative_extras;
pub mod narrative_imperfection;
pub mod narrative_irrationality;
pub mod narrative_presence;
pub mod narrative_ritual;
pub mod narrative_solitude;
pub mod narrative_story;
pub mod narrative_vulnerability;
