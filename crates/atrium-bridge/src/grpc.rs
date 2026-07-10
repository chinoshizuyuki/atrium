// SPDX-License-Identifier: MIT
//! gRPC 服务端
//! gRPC server implementation.
//!
//! 基于 tonic + proto/atrium.proto 编译生成的服务端代码。
//! Built on tonic + proto/atrium.proto generated server code.
//! 包含 ProcessMessageStream 流式 RPC。
//! Includes ProcessMessageStream streaming RPC.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tracing::info;

use crate::error::BridgeError;
use crate::protocol::BridgeEvent;

// proto 编译生成类型
// 在 mod atrium 内部展开 tonic::include_proto!
pub mod atrium {
    tonic::include_proto!("atrium");
}

/// 流式响应的类型别名：Box<dyn Stream<Item = Result<ProcessMessageChunk, Status>> + Send>
pub type ProcessMessageStreamSink = Pin<
    Box<dyn tokio_stream::Stream<Item = Result<atrium::ProcessMessageChunk, tonic::Status>> + Send>,
>;

// 服务 trait（用 atrium:: 前缀引用类型）

#[async_trait::async_trait]
pub trait AtriumCoreService: Send + Sync + 'static {
    async fn process_message(
        &self,
        req: atrium::ProcessMessageRequest,
    ) -> atrium::ProcessMessageResponse;
    async fn get_emotion(&self, req: atrium::GetEmotionRequest) -> atrium::EmotionState;
    async fn search_memory(&self, req: atrium::SearchMemoryRequest)
        -> atrium::SearchMemoryResponse;
    async fn search_canned(&self, req: atrium::SearchCannedRequest)
        -> atrium::SearchCannedResponse;
    async fn import_canned(&self, req: atrium::ImportCannedRequest)
        -> atrium::ImportCannedResponse;
    async fn health_check(&self, req: atrium::HealthCheckRequest) -> atrium::HealthCheckResponse;

    /// 流式处理消息
    /// 返回一个 Stream，逐 token 产出 ProcessMessageChunk
    async fn process_message_stream(
        &self,
        req: atrium::ProcessMessageRequest,
    ) -> ProcessMessageStreamSink;

    /// 上传文件 — 数字生命的工具记忆入口
    /// Upload file — Tool memory entry point of digital life
    async fn upload_file(&self, req: atrium::UploadFileRequest) -> atrium::UploadFileResponse;

    /// 处理音频帧 — 数字生命"听到声音"
    /// Process audio frame — digital life "hears voice"
    ///
    /// 数字生命工程理念：这是数字生命语音输入的核心处理点。
    /// 每个音频帧经 SttEngine 识别后转为文本，
    /// 最终文本送入 process_message 管线——与文字输入完全同构。
    /// Digital life engineering: this is the core processing point of digital life's voice input.
    /// Each audio frame is recognized by SttEngine and converted to text,
    /// which is finally sent to the process_message pipeline — fully isomorphic with text input.
    ///
    /// 当 STT 未启用时，返回空文本（降级模式，零影响）。
    /// When STT is not enabled, returns empty text (degraded mode, zero impact).
    async fn process_audio_frame(
        &self,
        req: atrium::AudioFrameRequest,
    ) -> atrium::AudioFrameResponse;
}

// gRPC 服务实现

pub struct GrpcServer {
    pub event_tx: flume::Sender<BridgeEvent>,
    pub service: Arc<dyn AtriumCoreService>,
}

impl std::fmt::Debug for GrpcServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GrpcServer").finish()
    }
}

#[tonic::async_trait]
impl atrium::atrium_core_server::AtriumCore for GrpcServer {
    async fn process_message(
        &self,
        request: tonic::Request<atrium::ProcessMessageRequest>,
    ) -> Result<tonic::Response<atrium::ProcessMessageResponse>, tonic::Status> {
        let t0 = std::time::Instant::now();
        let req = request.into_inner();
        info!(
        target: "atrium.audit.grpc",
        op = "ProcessMessage",
        channel = %req.channel,
        msg_len = req.message.len(),
        session_id = %req.session_id,
        "gRPC call start"
        );

        let _ = self.event_tx.send(BridgeEvent::UserMessage {
            channel: req.channel.clone(),
            content: req.message.clone(),
            user_id: req.user_id.clone(),
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64,
        });

        let resp = self.service.process_message(req).await;
        info!(
        target: "atrium.audit.grpc",
        op = "ProcessMessage",
        duration_us = t0.elapsed().as_micros(),
        emotion = %resp.emotion,
        "gRPC call completed"
        );
        Ok(tonic::Response::new(resp))
    }

    async fn get_emotion(
        &self,
        request: tonic::Request<atrium::GetEmotionRequest>,
    ) -> Result<tonic::Response<atrium::EmotionState>, tonic::Status> {
        let t0 = std::time::Instant::now();
        let req = request.into_inner();
        let state = self.service.get_emotion(req).await;
        info!(
        target: "atrium.audit.grpc",
        op = "GetEmotion",
        duration_us = t0.elapsed().as_micros(),
        pleasure = state.pleasure,
        arousal = state.arousal,
        dominance = state.dominance,
        "gRPC call completed"
        );
        Ok(tonic::Response::new(state))
    }

    async fn search_memory(
        &self,
        request: tonic::Request<atrium::SearchMemoryRequest>,
    ) -> Result<tonic::Response<atrium::SearchMemoryResponse>, tonic::Status> {
        let t0 = std::time::Instant::now();
        let req = request.into_inner();
        let query = req.query.clone();
        let resp = self.service.search_memory(req).await;
        info!(
        target: "atrium.audit.grpc",
        op = "SearchMemory",
        duration_us = t0.elapsed().as_micros(),
        query = %query,
        result_count = resp.results.len(),
        "gRPC call completed"
        );
        Ok(tonic::Response::new(resp))
    }

    async fn health_check(
        &self,
        request: tonic::Request<atrium::HealthCheckRequest>,
    ) -> Result<tonic::Response<atrium::HealthCheckResponse>, tonic::Status> {
        let t0 = std::time::Instant::now();
        let req = request.into_inner();
        let resp = self.service.health_check(req).await;
        info!(
        target: "atrium.audit.grpc",
        op = "HealthCheck",
        duration_us = t0.elapsed().as_micros(),
        module_count = resp.module_states.len(),
        "gRPC call completed"
        );
        Ok(tonic::Response::new(resp))
    }

    async fn search_canned(
        &self,
        request: tonic::Request<atrium::SearchCannedRequest>,
    ) -> Result<tonic::Response<atrium::SearchCannedResponse>, tonic::Status> {
        let req = request.into_inner();
        let resp = self.service.search_canned(req).await;
        Ok(tonic::Response::new(resp))
    }

    async fn import_canned(
        &self,
        request: tonic::Request<atrium::ImportCannedRequest>,
    ) -> Result<tonic::Response<atrium::ImportCannedResponse>, tonic::Status> {
        let req = request.into_inner();
        let resp = self.service.import_canned(req).await;
        Ok(tonic::Response::new(resp))
    }

    /// 流式 ProcessMessageStream
    type ProcessMessageStreamStream = ProcessMessageStreamSink;

    async fn process_message_stream(
        &self,
        request: tonic::Request<atrium::ProcessMessageRequest>,
    ) -> Result<tonic::Response<Self::ProcessMessageStreamStream>, tonic::Status> {
        let t0 = std::time::Instant::now();
        let req = request.into_inner();
        info!(
        target: "atrium.audit.grpc",
        op = "ProcessMessageStream",
        channel = %req.channel,
        msg_len = req.message.len(),
        session_id = %req.session_id,
        "gRPC stream call start"
        );

        let _ = self.event_tx.send(BridgeEvent::UserMessage {
            channel: req.channel.clone(),
            content: req.message.clone(),
            user_id: req.user_id.clone(),
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64,
        });

        let stream = self.service.process_message_stream(req).await;
        info!(
        target: "atrium.audit.grpc",
        op = "ProcessMessageStream",
        elapsed_us = t0.elapsed().as_micros(),
        "gRPC stream started (spawning)"
        );
        Ok(tonic::Response::new(stream))
    }

    /// 文件上传 RPC — 用户文件 → FileStore 存储
    /// File upload RPC — user file → FileStore persistence
    async fn upload_file(
        &self,
        request: tonic::Request<atrium::UploadFileRequest>,
    ) -> Result<tonic::Response<atrium::UploadFileResponse>, tonic::Status> {
        let t0 = std::time::Instant::now();
        let req = request.into_inner();
        info!(
        target: "atrium.audit.grpc",
        op = "UploadFile",
        filename = %req.filename,
        data_len = req.data.len(),
        session_id = %req.session_id,
        "gRPC call start"
        );

        let resp = self.service.upload_file(req).await;
        info!(
        target: "atrium.audit.grpc",
        op = "UploadFile",
        duration_us = t0.elapsed().as_micros(),
        hash = %resp.hash,
        "gRPC call completed"
        );
        Ok(tonic::Response::new(resp))
    }

    /// 音频流 RPC — 双向流式语音输入（STT）
    /// Audio stream RPC — bidirectional streaming voice input (STT)
    ///
    /// 数字生命"听到声音"的 gRPC 入口：客户端发送 PCM 帧，服务端逐帧调用
    /// process_audio_frame 进行识别并返回文本。客户端断开时优雅终止。
    /// Digital life's "hearing" gRPC entry: client sends PCM frames, server calls
    /// process_audio_frame per frame for recognition and returns text.
    /// Terminates gracefully on client disconnect.
    type AudioStreamStream =
        tokio_stream::wrappers::ReceiverStream<Result<atrium::AudioFrameResponse, tonic::Status>>;

    async fn audio_stream(
        &self,
        request: tonic::Request<tonic::Streaming<atrium::AudioFrameRequest>>,
    ) -> Result<tonic::Response<Self::AudioStreamStream>, tonic::Status> {
        let mut stream = request.into_inner();
        let (tx, rx) = tokio::sync::mpsc::channel(16);
        let service = Arc::clone(&self.service);

        tokio::spawn(async move {
            while let Ok(Some(frame)) = stream.message().await {
                // 处理音频帧 — 数字生命"听到声音" / Process audio frame — digital life "hears voice"
                let resp = service.process_audio_frame(frame).await;
                if tx.send(Ok(resp)).await.is_err() {
                    // 客户端断开 — 优雅终止 / Client disconnected — graceful termination
                    break;
                }
            }
        });

        Ok(tonic::Response::new(
            tokio_stream::wrappers::ReceiverStream::new(rx),
        ))
    }
}

impl GrpcServer {
    pub fn new(event_tx: flume::Sender<BridgeEvent>, service: Arc<dyn AtriumCoreService>) -> Self {
        Self { event_tx, service }
    }
}

// 启动函数

pub async fn start_grpc_server(
    addr: String,
    event_tx: flume::Sender<BridgeEvent>,
    service: Arc<dyn AtriumCoreService>,
) -> Result<(), BridgeError> {
    let server = GrpcServer::new(event_tx, service);

    let addr: std::net::SocketAddr = addr
        .parse()
        .map_err(|e| BridgeError::Grpc(format!("地址解析失败: {}", e)))?;

    info!("gRPC 服务器启动, 监听: {}", addr);

    tonic::transport::Server::builder()
        .add_service(atrium::atrium_core_server::AtriumCoreServer::new(server))
        .serve(addr)
        .await
        .map_err(|e| BridgeError::Grpc(format!("gRPC 服务器异常: {}", e)))
}

// 占位实现

pub struct PlaceholderCoreService;

#[async_trait::async_trait]
impl AtriumCoreService for PlaceholderCoreService {
    async fn process_message(
        &self,
        req: atrium::ProcessMessageRequest,
    ) -> atrium::ProcessMessageResponse {
        atrium::ProcessMessageResponse {
            reply: format!("[占位回复] 收到: {}", req.message),
            emotion: "neutral".into(),
            actions: vec![],
            expression: None,
        }
    }

    async fn get_emotion(&self, _req: atrium::GetEmotionRequest) -> atrium::EmotionState {
        atrium::EmotionState::default()
    }

    async fn search_memory(
        &self,
        _req: atrium::SearchMemoryRequest,
    ) -> atrium::SearchMemoryResponse {
        atrium::SearchMemoryResponse { results: vec![] }
    }

    async fn health_check(&self, req: atrium::HealthCheckRequest) -> atrium::HealthCheckResponse {
        atrium::HealthCheckResponse {
            ok: true,
            event_count: req.event_count,
            uptime_seconds: 0,
            module_states: HashMap::new(),
        }
    }

    async fn search_canned(
        &self,
        _req: atrium::SearchCannedRequest,
    ) -> atrium::SearchCannedResponse {
        atrium::SearchCannedResponse {
            results: vec![],
            total: 0,
        }
    }

    async fn import_canned(
        &self,
        _req: atrium::ImportCannedRequest,
    ) -> atrium::ImportCannedResponse {
        atrium::ImportCannedResponse {
            imported: 0,
            names: vec![],
            error: "not connected to canned manager".into(),
        }
    }

    /// 占位文件上传实现 / Placeholder file upload implementation
    async fn upload_file(&self, _req: atrium::UploadFileRequest) -> atrium::UploadFileResponse {
        atrium::UploadFileResponse {
            hash: String::new(),
            original_name: String::new(),
            size: 0,
            text_extracted: false,
            extracted_text: String::new(),
            error: "placeholder: file store not connected".into(),
        }
    }

    /// 占位音频帧处理 — STT 未接入，返回静默降级响应
    /// Placeholder audio frame processing — STT not connected, returns silence degraded response
    async fn process_audio_frame(
        &self,
        req: atrium::AudioFrameRequest,
    ) -> atrium::AudioFrameResponse {
        atrium::AudioFrameResponse {
            text: String::new(),
            status: "silence".to_string(),
            processed_samples: req.pcm_data.len() as u32 / 4, // f32 = 4 字节 / f32 = 4 bytes
            latency_ms: 0,
        }
    }

    /// 占位流式实现：将完整回复拆成逐 token 的 chunk 流
    #[allow(clippy::result_large_err)]
    async fn process_message_stream(
        &self,
        req: atrium::ProcessMessageRequest,
    ) -> ProcessMessageStreamSink {
        let reply = format!("[占位流式回复] 收到: {}", req.message);
        let emotion = "neutral".to_string();

        // 将完整回复按字符拆成 chunk 流（占位实现，每 chunk 一个字符）
        let chunks: Vec<Result<atrium::ProcessMessageChunk, tonic::Status>> = reply
            .chars()
            .map(|c| {
                Ok(atrium::ProcessMessageChunk {
                    token: c.to_string(),
                    emotion: emotion.clone(),
                    done: false,
                    meta: HashMap::new(),
                    expression: None,
                })
            })
            .chain(std::iter::once(Ok(atrium::ProcessMessageChunk {
                token: String::new(),
                emotion: emotion.clone(),
                done: true,
                meta: HashMap::new(),
                expression: None,
            })))
            .collect();

        let stream = tokio_stream::iter(chunks);
        Box::pin(stream)
    }
}

/// 测试用例
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_placeholder_process_message() {
        let service = PlaceholderCoreService;
        let req = atrium::ProcessMessageRequest {
            message: "你好".into(),
            channel: "test".into(),
            user_id: "u1".into(),
            session_id: "s1".into(),
        };
        let resp = service.process_message(req).await;
        assert!(resp.reply.contains("你好"));
    }

    #[tokio::test]
    async fn test_placeholder_health_check() {
        let service = PlaceholderCoreService;
        let req = atrium::HealthCheckRequest {
            event_count: 42,
            room_incoming_json: String::new(),
        };
        let resp = service.health_check(req).await;
        assert!(resp.ok);
        assert_eq!(resp.event_count, 42);
    }

    /// 测试占位流式实现能产出 chunk 并以 done=true 结束
    #[tokio::test]
    async fn test_placeholder_process_message_stream() {
        use futures_util::StreamExt;

        let service = PlaceholderCoreService;
        let req = atrium::ProcessMessageRequest {
            message: "hi".into(),
            channel: "test".into(),
            user_id: "u1".into(),
            session_id: "s1".into(),
        };
        let mut stream = service.process_message_stream(req).await;

        let mut tokens = String::new();
        let mut got_done = false;
        while let Some(result) = stream.next().await {
            let chunk = result.expect("chunk should be Ok");
            if chunk.done {
                got_done = true;
            } else {
                tokens.push_str(&chunk.token);
            }
        }
        assert!(got_done, "stream should end with done=true");
        assert!(
            tokens.contains("hi"),
            "tokens should contain original message"
        );
    }
}
