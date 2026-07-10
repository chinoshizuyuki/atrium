// SPDX-License-Identifier: MIT
//! 音频缓冲区管理器 — 无锁 SPSC 环形缓冲区写入器
//! Audio Buffer Manager — lock-free SPSC ring buffer writer.
//!
//! 数字生命工程理念：无锁极致性能——单生产者单消费者模式，原子读写指针，零等待。
//! TTS 合成线程作为唯一写入器，渲染引擎作为唯一读取器，通过原子指针同步，无需任何锁。
//! Digital life engineering: lock-free extreme performance — SPSC pattern, atomic
//! read/write pointers, zero waiting. The TTS synthesis thread is the sole writer,
//! the render engine is the sole reader, synchronized via atomic pointers with no locks.

use atrium_bridge::shm::AudioBuffer;
use std::sync::atomic::Ordering;
use std::sync::Arc;

// ════════════════════════════════════════════════════════════════════
// AudioManager — 音频缓冲区管理器
// ════════════════════════════════════════════════════════════════════

/// 音频缓冲区管理器 — 无锁环形缓冲区写入器
/// Audio buffer manager — lock-free ring buffer writer.
///
/// 采用 SPSC（单生产者单消费者）模式：
/// - 写入端（本管理器）：TTS 合成线程，通过 `write_chunk` 推进 `write_pos`
/// - 读取端（渲染引擎）：Unity/Live2D 线程，通过 `read_pos` 消费 PCM 数据
///
/// Uses SPSC (Single Producer Single Consumer) pattern:
/// - Writer (this manager): TTS synthesis thread, advances `write_pos` via `write_chunk`
/// - Reader (render engine): Unity/Live2D thread, consumes PCM data via `read_pos`
pub struct AudioManager {
    /// 共享音频缓冲区引用 / Shared audio buffer reference
    buffer: Arc<AudioBuffer>,
    /// 本地写指针缓存（减少原子读）/ Local write position cache (reduces atomic reads)
    write_pos: u32,
    /// 采样率 / Sample rate
    sample_rate: u32,
    /// 单次写入块大小 / Chunk size per write
    chunk_size: usize,
    /// 缓冲区总容量 / Total buffer capacity
    capacity: u32,
}

impl AudioManager {
    /// 创建音频管理器 — 绑定共享缓冲区
    /// Create audio manager — bind to shared buffer.
    ///
    /// @param buffer 共享音频缓冲区 / Shared audio buffer
    /// @param sample_rate 采样率 / Sample rate
    /// @param chunk_size 单次写入块大小 / Chunk size per write
    /// @return 音频管理器实例 / Audio manager instance
    pub fn new(buffer: Arc<AudioBuffer>, sample_rate: u32, chunk_size: usize) -> Self {
        let write_pos = buffer.write_pos.load(Ordering::Acquire);
        let capacity = buffer.data.len() as u32;
        Self {
            buffer,
            write_pos,
            sample_rate,
            chunk_size,
            capacity,
        }
    }

    /// 写入一块 PCM 样本 — 无锁，永不阻塞
    /// Write a chunk of PCM samples — lock-free, never blocks.
    ///
    /// 缓冲区满时仅写入可容纳部分，返回实际写入样本数。
    /// When buffer is full, writes only what fits and returns actual count written.
    ///
    /// @param samples PCM 样本切片 / PCM sample slice
    /// @return 实际写入样本数 / Actual number of samples written
    pub fn write_chunk(&mut self, samples: &[f32]) -> usize {
        // 读取消费者位置 / Read consumer position
        let read_pos = self.buffer.read_pos.load(Ordering::Acquire);
        // 计算可用空间（SPSC 环形缓冲区：留 1 槽防止 write_pos 追上 read_pos）
        // Compute available space (SPSC ring buffer: reserve 1 slot to prevent write_pos catching read_pos)
        let available = (self.capacity + read_pos - self.write_pos - 1) % self.capacity;
        if available == 0 {
            // 缓冲区满 — 不阻塞，返回 0 / Buffer full — don't block, return 0
            return 0;
        }
        let to_write = samples.len().min(available as usize);

        // SAFETY: SPSC 模式 — 仅本写入器修改 write_pos 和正在写入的数据槽。
        // 读取器仅读 read_pos 和已写入的数据槽（write_pos > index）。
        // 通过 Arc::as_ptr 获取可变指针是安全的，因为：
        // 1. SPSC 保证无数据竞争
        // 2. Release/Acquire 顺序对保证可见性
        // SAFETY: SPSC pattern — only this writer modifies write_pos and data slots being written.
        // The reader only reads read_pos and written data slots (write_pos > index).
        // Using Arc::as_ptr to obtain a mutable pointer is safe because:
        // 1. SPSC guarantees no data race
        // 2. Release/Acquire ordering ensures visibility
        let data_ptr = Arc::as_ptr(&self.buffer) as *mut AudioBuffer;
        let data = unsafe { &mut (*data_ptr).data };
        for (i, &sample) in samples.iter().enumerate().take(to_write) {
            let idx = ((self.write_pos + i as u32) % self.capacity) as usize;
            data[idx] = sample;
        }

        // 推进写指针并原子发布 / Advance write pointer and atomically publish
        self.write_pos = self.write_pos.wrapping_add(to_write as u32);
        self.buffer
            .write_pos
            .store(self.write_pos, Ordering::Release);

        to_write
    }

    /// 缓冲区是否有未消费音频
    /// Whether buffer has unconsumed audio.
    pub fn has_audio(&self) -> bool {
        self.buffer.write_pos.load(Ordering::Acquire)
            != self.buffer.read_pos.load(Ordering::Acquire)
    }

    /// 可用写入空间（样本数）
    /// Available write space (in samples).
    pub fn available_write_space(&self) -> usize {
        let read_pos = self.buffer.read_pos.load(Ordering::Acquire);
        let available = (self.capacity + read_pos - self.write_pos - 1) % self.capacity;
        available as usize
    }

    /// 清空缓冲区 — 重置读写指针（仅在无并发访问时调用）
    /// Clear buffer — reset read/write pointers (only call when no concurrent access).
    pub fn clear(&self) {
        self.buffer.write_pos.store(0, Ordering::Release);
        self.buffer.read_pos.store(0, Ordering::Release);
    }

    /// 采样率 / Sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// 缓冲区总容量 / Total buffer capacity
    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    /// 单次写入块大小 / Chunk size per write
    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }
}

// ════════════════════════════════════════════════════════════════════
// 测试 / Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    /// 创建测试用音频缓冲区（零初始化）
    /// Create test audio buffer (zero-initialized).
    ///
    /// SAFETY: AudioBuffer 为 #[repr(C)]，全零有效：
    /// - [f32; 16384] 全零 = [0.0; 16384]（有效浮点数）
    /// - AtomicU32 全零 = AtomicU32::new(0)（有效原子状态）
    fn make_test_buffer() -> Arc<AudioBuffer> {
        // 安全：AudioBuffer is #[repr(C)], all-zero is valid:
        // - [f32; 16384] all-zero = [0.0; 16384] (valid floats)
        // - AtomicU32 all-zero = AtomicU32::new(0) (valid atomic state)
        Arc::new(unsafe { std::mem::zeroed() })
    }

    /// 创建带初始指针的测试缓冲区 / Create test buffer with initial positions
    fn make_test_buffer_with_positions(write_pos: u32, read_pos: u32) -> Arc<AudioBuffer> {
        let mut buf: AudioBuffer = unsafe { std::mem::zeroed() };
        buf.write_pos = AtomicU32::new(write_pos);
        buf.read_pos = AtomicU32::new(read_pos);
        Arc::new(buf)
    }

    #[test]
    fn test_write_and_read_pointer_advance() {
        // 写入 100 样本后 write_pos 推进 100 / After writing 100 samples, write_pos advances by 100
        let buffer = make_test_buffer();
        let mut manager = AudioManager::new(buffer.clone(), 16000, 1024);
        let samples = vec![0.5f32; 100];
        let written = manager.write_chunk(&samples);
        assert_eq!(written, 100, "should write all 100 samples");
        let new_pos = buffer.write_pos.load(Ordering::Acquire);
        assert_eq!(new_pos, 100, "write_pos should advance by 100");
    }

    #[test]
    fn test_buffer_overflow_protection() {
        // 缓冲区满时仅写部分，不 panic / When buffer full, write partial, no panic
        let capacity = 16384u32;
        let buffer = make_test_buffer_with_positions(capacity - 1, 0);
        let mut manager = AudioManager::new(buffer.clone(), 16000, 1024);
        let samples = vec![0.5f32; 100];
        let written = manager.write_chunk(&samples);
        // available = (capacity + 0 - (capacity-1) - 1) % capacity = 0
        assert_eq!(written, 0, "should write 0 when buffer full");
    }

    #[test]
    fn test_empty_buffer_has_no_audio() {
        // 空缓冲区 has_audio() 返回 false / Empty buffer has_audio() returns false
        let buffer = make_test_buffer();
        let manager = AudioManager::new(buffer, 16000, 1024);
        assert!(!manager.has_audio(), "empty buffer should have no audio");
    }

    #[test]
    fn test_write_then_has_audio() {
        // 写入后有音频 / After writing, has audio
        let buffer = make_test_buffer();
        let mut manager = AudioManager::new(buffer.clone(), 16000, 1024);
        let samples = vec![0.5f32; 50];
        manager.write_chunk(&samples);
        assert!(manager.has_audio(), "should have audio after write");
    }

    #[test]
    fn test_available_write_space_full() {
        // 缓冲区满时可用空间为 0 / Available space is 0 when buffer full
        let capacity = 16384u32;
        let buffer = make_test_buffer_with_positions(capacity - 1, 0);
        let manager = AudioManager::new(buffer, 16000, 1024);
        assert_eq!(
            manager.available_write_space(),
            0,
            "full buffer should have 0 available space"
        );
    }

    #[test]
    fn test_available_write_space_empty() {
        // 空缓冲区可用空间 = capacity - 1 / Empty buffer available space = capacity - 1
        let buffer = make_test_buffer();
        let manager = AudioManager::new(buffer, 16000, 1024);
        assert_eq!(
            manager.available_write_space(),
            16383,
            "empty buffer should have capacity-1 available space"
        );
    }

    #[test]
    fn test_clear_resets_positions() {
        // 清空后读写指针归零 / After clear, read/write positions reset to 0
        let buffer = make_test_buffer();
        let mut manager = AudioManager::new(buffer.clone(), 16000, 1024);
        let samples = vec![0.5f32; 100];
        manager.write_chunk(&samples);
        assert!(buffer.write_pos.load(Ordering::Acquire) > 0);
        manager.clear();
        assert_eq!(buffer.write_pos.load(Ordering::Acquire), 0);
        assert_eq!(buffer.read_pos.load(Ordering::Acquire), 0);
    }

    #[test]
    fn test_wraparound_write() {
        // 回绕写入：write_pos 接近容量末尾时回绕 / Wraparound write: when write_pos near capacity end
        let capacity = 16384u32;
        // write_pos = capacity - 5, read_pos = 0 → available = 4
        // 写入 10 个样本：仅写 4 个，新 write_pos = (capacity-5) + 4 = capacity - 1 = 16383
        // 再次写入时 available = (capacity + 0 - 16383 - 1) % capacity = 0 → 满了
        // 推进 read_pos 到 0 后，再写入触发回绕：新 write_pos = (16383 + N) % capacity
        let buffer = make_test_buffer_with_positions(capacity - 5, 0);
        let mut manager = AudioManager::new(buffer.clone(), 16000, 1024);
        let samples = vec![0.7f32; 10];
        let written = manager.write_chunk(&samples);
        // available = (capacity + 0 - (capacity-5) - 1) % capacity = 4
        assert_eq!(written, 4, "should write 4 samples (available space)");
        // 验证写入的数据正确（4 个样本写到末尾槽位）
        // Verify written data is correct (4 samples written at end slots)
        let buf_ptr = Arc::as_ptr(&buffer);
        let data = unsafe { &(*buf_ptr).data };
        assert_eq!(
            data[(capacity - 5) as usize],
            0.7,
            "first sample at write_pos"
        );
        assert_eq!(
            data[(capacity - 2) as usize],
            0.7,
            "last sample before capacity end"
        );
        // write_pos = capacity - 1 = 16383（尚未回绕）/ write_pos = capacity - 1 = 16383 (not yet wrapped)
        let new_pos = buffer.write_pos.load(Ordering::Acquire);
        assert_eq!(new_pos, capacity - 1, "write_pos should be capacity-1");
    }

    #[test]
    fn test_partial_write_when_near_full() {
        // 接近满时部分写入 / Partial write when near full
        let buffer = make_test_buffer_with_positions(100, 0);
        let mut manager = AudioManager::new(buffer.clone(), 16000, 1024);
        // available = (capacity + 0 - 100 - 1) % capacity = 16283
        let available = manager.available_write_space();
        assert_eq!(available, 16283);
        // 写入比可用空间多的样本 / Write more samples than available
        let samples = vec![0.5f32; 20000];
        let written = manager.write_chunk(&samples);
        assert_eq!(written, 16283, "should write only available space");
    }
}
