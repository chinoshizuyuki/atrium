// SPDX-License-Identifier: MIT
//! 共享内存桥接层
//! Shared memory bridge layer.
//!
//! Rust 核心通过共享内存向渲染引擎持续写入实时状态数据。
//! 渲染引擎（Unity/Unreal/Live2D）通过同一块内存读取。
//! Rust core writes real-time state data to shared memory for rendering engines.
//! Rendering engines (Unity/Unreal/Live2D) read from the same memory region.
//!
//! ## 同步策略 / Synchronization Strategy
//!
//! - AtomicU32 version 字段做撕裂写检测 / AtomicU32 version field for tear detection
//! - 写入端：先写数据，最后原子递增 version / Writer: write data first, then atomically increment version
//! - 读取端：先读 version，再读数据，再读 version 校验 / Reader: read version, read data, re-read version to validate
//! - 音频数据：环形缓冲区 + 读写指针 / Audio data: ring buffer + read/write pointers

use crate::error::BridgeError;
use crate::protocol::EmotionState;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

// 共享内存数据结构

/// 渲染状态
///
/// Rust 核心持续写入，渲染引擎按帧读取。
/// #[repr(C)] 保证内存布局在语言间一致。
#[repr(C)]
pub struct RenderState {
    /// 版本号（撕裂写检测 + 更新通知）
    pub version: AtomicU32,
    /// 时间戳（Unix 毫秒）
    pub timestamp_ms: AtomicU64,

    // ── PAD 情感 ──
    pub pleasure: f32,
    pub arousal: f32,
    pub dominance: f32,

    // ── 口型同步 ──
    pub phoneme: [f32; 16],
    pub mouth_open: f32,

    // ── 表情 ──
    pub expression_id: u32,
    pub expression_blend: f32,

    // ── 头部和眼睛 ──
    pub head_rotation: [f32; 4],
    pub eye_direction: [f32; 2],
    pub blink_state: f32,

    // ── 体态（KinesicsMapper 输出）──
    /// 肩膀展开度 [0,1]
    pub shoulder_openness: f32,
    /// 身体前倾/后仰 [-0.5,0.5]
    pub body_lean: f32,
    /// 手势活跃度 [0,1]
    pub gesture_activity: f32,
    /// 呼吸频率因子
    pub breath_rate: f32,

    // ── 韵律（ProsodyMapper 输出）──
    /// 基频偏移（半音）
    pub pitch_offset: f32,
    /// 语速因子
    pub speech_rate: f32,
    /// 音色温暖度 [0,1]
    pub warmth: f32,

    // ── 音频 ──
    pub audio_write_pos: AtomicU32,
    pub audio_read_pos: AtomicU32,
    pub is_speaking: u8,

    // ── 事件通知 ──
    pub pending_events: AtomicU32,
}

/// 音频环形缓冲区
#[repr(C)]
pub struct AudioBuffer {
    pub data: [f32; 16384],
    pub write_pos: AtomicU32,
    pub read_pos: AtomicU32,
}

/// 输入事件槽位
#[repr(C)]
pub struct InputEventSlot {
    pub events: [u64; 32],
    pub count: AtomicU32,
}

/// 完整的共享内存区域
#[repr(C)]
pub struct SharedMemoryRegion {
    pub magic: u32,
    pub version: u32,
    pub render_state: RenderState,
    pub audio: AudioBuffer,
    pub input: InputEventSlot,
    /// — Self-Play 思考流状态
    pub thought_stream: ThoughtStreamState,
}

/// Self-Play 思考流
///
/// Rust 核心在 Round 进行中写入当前 Slot 的思考过程，
/// 渲染引擎按帧读取以展示"AI 正在思考"的状态。
#[repr(C)]
pub struct ThoughtStreamState {
    /// Round 是否正在进行中
    pub room_active: u8,
    /// 正在思考的 Slot 名称（最多 32 字节 UTF-8）
    pub slot_name: [u8; 32],
    /// 当前思考文本片段（最多 512 字节 UTF-8）
    pub thought_text: [u8; 512],
    /// 当前话题（最多 128 字节 UTF-8）
    pub topic: [u8; 128],
    /// 自增序号（前端/渲染引擎用于判断是否有新数据）
    pub sequence: u32,
    /// 填充对齐
    pub _pad: [u8; 3],
}

impl Default for ThoughtStreamState {
    fn default() -> Self {
        Self {
            room_active: 0,
            slot_name: [0u8; 32],
            thought_text: [0u8; 512],
            topic: [0u8; 128],
            sequence: 0,
            _pad: [0u8; 3],
        }
    }
}

impl ThoughtStreamState {
    /// 写入字节到固定数组（自动截断，确保 UTF-8 边界安全）
    fn write_bytes(dst: &mut [u8], src: &str) {
        let bytes = src.as_bytes();
        let len = bytes.len().min(dst.len());
        dst[..len].copy_from_slice(&bytes[..len]);
        // 剩余填 0
        for b in dst.iter_mut().skip(len) {
            *b = 0;
        }
    }

    /// 更新思考流状态（由 Self-Play Room 写入）
    pub fn update_thought(
        &mut self,
        room_active: bool,
        slot_name: &str,
        thought_text: &str,
        topic: &str,
    ) {
        self.room_active = if room_active { 1 } else { 0 };
        Self::write_bytes(&mut self.slot_name, slot_name);
        Self::write_bytes(&mut self.thought_text, thought_text);
        Self::write_bytes(&mut self.topic, topic);
        self.sequence = self.sequence.wrapping_add(1);
    }

    /// 清空思考流（Round 结束）
    pub fn clear(&mut self) {
        self.room_active = 0;
        self.slot_name = [0u8; 32];
        self.thought_text = [0u8; 512];
    }
}

pub const SHM_MAGIC: u32 = 0x4154524D;
pub const SHM_DEFAULT_SIZE: usize = std::mem::size_of::<SharedMemoryRegion>();

// 方法

impl RenderState {
    pub fn update_from_emotion(&mut self, emotion: &EmotionState) {
        self.pleasure = emotion.pleasure;
        self.arousal = emotion.arousal;
        self.dominance = emotion.dominance;
        self.expression_id = if emotion.pleasure > 0.5 {
            1
        } else if emotion.pleasure < -0.3 {
            2
        } else if emotion.arousal > 0.5 && emotion.dominance > 0.3 {
            3
        } else if emotion.arousal > 0.5 {
            4
        } else if emotion.dominance < -0.3 {
            5
        } else {
            0
        };
        self.expression_blend =
            (emotion.pleasure.abs() + emotion.arousal.abs() + emotion.dominance.abs()) / 3.0;
    }

    /// 从韵律参数更新渲染状态 — 将 ProsodyMapper 输出同步到共享内存
    /// Update render state from prosody params — sync ProsodyMapper output to shared memory.
    ///
    /// 数字生命工程理念：韵律是声音的情感指纹。
    /// 每 200ms，Scheduler 将 ProsodyMapper 产出的韵律参数写入此处，
    /// 渲染引擎据此调整 TTS 引擎的基频、语速和音色，
    /// 让数字生命的声音随情感连续变化——同样的话，悲伤时低沉，喜悦时明亮。
    ///
    /// Digital life engineering: prosody is the emotional fingerprint of voice.
    /// Every 200ms, the Scheduler writes ProsodyMapper output here,
    /// and the render engine adjusts TTS pitch, rate, and warmth accordingly,
    /// making digital life's voice vary continuously with emotion —
    /// the same words sound low and slow when sad, bright and fast when joyful.
    ///
    /// @param prosody 韵律参数引用 / Reference to prosody parameters
    pub fn update_from_prosody(&mut self, prosody: &atrium_memory::prosody_mapper::ProsodyParams) {
        self.pitch_offset = prosody.pitch_offset;
        self.speech_rate = prosody.rate;
        self.warmth = prosody.warmth;
    }

    /// 设置说话状态 — 标记数字生命是否正在发声
    /// Set speaking state — mark whether digital life is currently speaking.
    ///
    /// 当 TTS 引擎开始合成时置为 1，合成结束或 barge-in 时置为 0。
    /// 渲染引擎据此控制口型同步动画和音频输出。
    ///
    /// Set to 1 when TTS engine starts synthesis, 0 on completion or barge-in.
    /// The render engine uses this to control lip-sync animation and audio output.
    ///
    /// @param speaking 是否正在说话 / Whether currently speaking
    pub fn set_speaking(&mut self, speaking: bool) {
        self.is_speaking = if speaking { 1 } else { 0 };
    }

    pub fn publish(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.timestamp_ms.store(now, Ordering::Release);
        self.version.fetch_add(1, Ordering::Release);
    }

    pub fn has_new_data(&self, last_version: u32) -> bool {
        self.version.load(Ordering::Acquire) != last_version
    }
}

impl SharedMemoryRegion {
    pub fn init(&mut self) {
        self.magic = SHM_MAGIC;
        self.version = 1;
    }

    pub fn validate(&self) -> Result<(), BridgeError> {
        if self.magic != SHM_MAGIC {
            return Err(BridgeError::Shm(format!(
                "魔数不匹配: 期望 0x{:08X}, 收到 0x{:08X}",
                SHM_MAGIC, self.magic
            )));
        }
        Ok(())
    }
}

// 共享内存管理器（shm feature 启用时）

#[cfg(feature = "shm")]
pub struct SharedMemory {
    shmem: shared_memory::Shmem,
    region: *mut SharedMemoryRegion,
}

#[cfg(feature = "shm")]
unsafe impl Send for SharedMemory {}
#[cfg(feature = "shm")]
unsafe impl Sync for SharedMemory {}

#[cfg(feature = "shm")]
impl SharedMemory {
    pub fn create_or_open(name: &str) -> Result<Self, BridgeError> {
        let size = SHM_DEFAULT_SIZE;
        let (shmem, is_new) = match shared_memory::ShmemConf::new()
            .size(size)
            .os_id(name)
            .create()
        {
            Ok(m) => {
                tracing::info!("创建共享内存: {} ({} 字节)", name, size);
                (m, true)
            }
            Err(shared_memory::ShmemError::MappingIdExists) => {
                let m = shared_memory::ShmemConf::new()
                    .os_id(name)
                    .open()
                    .map_err(|e| BridgeError::Shm(format!("打开共享内存失败: {}", e)))?;
                tracing::info!("打开已有共享内存: {}", name);
                (m, false)
            }
            Err(e) => return Err(BridgeError::Shm(format!("共享内存操作失败: {}", e))),
        };
        let ptr = shmem.as_ptr() as *mut SharedMemoryRegion;
        let region = unsafe { &mut *ptr };
        if is_new {
            region.init();
        }
        region.validate()?;
        Ok(Self { shmem, region: ptr })
    }

    pub fn region(&self) -> &SharedMemoryRegion {
        unsafe { &*self.region }
    }
    pub fn region_mut(&mut self) -> &mut SharedMemoryRegion {
        unsafe { &mut *self.region }
    }
    pub fn render_state(&self) -> &RenderState {
        &self.region().render_state
    }
    pub fn render_state_mut(&mut self) -> &mut RenderState {
        &mut self.region_mut().render_state
    }
    pub fn audio_buffer(&self) -> &AudioBuffer {
        &self.region().audio
    }
    pub fn audio_buffer_mut(&mut self) -> &mut AudioBuffer {
        &mut self.region_mut().audio
    }
}

#[cfg(feature = "shm")]
impl Drop for SharedMemory {
    fn drop(&mut self) {
        tracing::debug!("共享内存管理器释放");
    }
}

// 共享内存管理器（shm feature 未启用时 - 占位）
// Shared memory manager placeholder when shm feature is disabled.
//
// 仅保留 create_or_open（始终返回 Err），其余方法编译期消除。
// Only create_or_open is kept (always returns Err); other methods are eliminated at compile time.
// 理由：create_or_open 返回 Err 意味着无法构造 SharedMemory 实例，
// 因此 region/render_state/audio_buffer 等方法不可达，保留 panic! 是运行时隐患。
// Rationale: create_or_open returning Err means no SharedMemory can be constructed,
// so region/render_state/audio_buffer etc. are unreachable — keeping panic! is a runtime hazard.

#[cfg(not(feature = "shm"))]
pub struct SharedMemory;

#[cfg(not(feature = "shm"))]
impl SharedMemory {
    /// 占位构造 — 始终返回错误 / Placeholder constructor — always returns Err.
    pub fn create_or_open(_name: &str) -> Result<Self, BridgeError> {
        tracing::warn!("共享内存功能未启用 (编译时未添加 feature=shm)");
        Err(BridgeError::Shm("feature=shm 未启用".into()))
    }
}

// 测试用例

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_state_size() {
        let size = std::mem::size_of::<RenderState>();
        let align = std::mem::align_of::<RenderState>();
        println!("RenderState 大小={} 对齐={}", size, align);
        assert!(size >= 140, "RenderState 太小，可能漏了字段");
    }

    #[test]
    fn test_emotion_to_expression() {
        let mut state: RenderState = unsafe { std::mem::zeroed() };
        state.update_from_emotion(&EmotionState::new(0.8, 0.3, 0.2));
        assert_eq!(state.expression_id, 1);
        state.update_from_emotion(&EmotionState::new(-0.5, -0.2, -0.3));
        assert_eq!(state.expression_id, 2);
    }

    #[test]
    fn test_version_barrier() {
        let mut region: SharedMemoryRegion = unsafe { std::mem::zeroed() };
        region.init();
        let initial = region.render_state.version.load(Ordering::Acquire);
        region.render_state.publish();
        let after = region.render_state.version.load(Ordering::Acquire);
        assert!(after > initial);
    }

    #[test]
    fn test_region_validation() {
        let mut region: SharedMemoryRegion = unsafe { std::mem::zeroed() };
        assert!(region.validate().is_err());
        region.init();
        assert!(region.validate().is_ok());
    }

    #[test]
    fn test_update_from_prosody() {
        // 韵律参数写入正确性 / Prosody params write correctness
        let mut state: RenderState = unsafe { std::mem::zeroed() };
        let prosody = atrium_memory::prosody_mapper::ProsodyParams {
            pitch_offset: 2.5,
            pitch_range: 7.0,
            rate: 1.3,
            energy: 1.1,
            pause_duration_ms: 300.0,
            intra_pause_prob: 0.2,
            warmth: 0.8,
            breathiness: 0.15,
        };
        state.update_from_prosody(&prosody);
        assert_eq!(state.pitch_offset, 2.5);
        assert_eq!(state.speech_rate, 1.3);
        assert_eq!(state.warmth, 0.8);
    }

    #[test]
    fn test_set_speaking() {
        // 说话状态切换 / Speaking state toggle
        let mut state: RenderState = unsafe { std::mem::zeroed() };
        assert_eq!(state.is_speaking, 0);
        state.set_speaking(true);
        assert_eq!(state.is_speaking, 1);
        state.set_speaking(false);
        assert_eq!(state.is_speaking, 0);
    }
}
