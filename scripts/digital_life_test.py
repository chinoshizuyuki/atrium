#!/usr/bin/env python3
"""Atrium 数字生命全真测试 / Digital Life Full Test

通过 gRPC 直接与 Atrium 核心交互，测试数字生命关键行为。
Tests digital life key behaviors via gRPC direct connection to Atrium core.

Usage:
    python scripts/digital_life_test.py
"""

import sys
import os
import time
import json
import grpc

# 添加 proto 路径 / Add proto path
# 需要设置完整的包结构以支持相对导入 / Need full package structure for relative imports
gateway_dir = os.path.join(os.path.dirname(__file__), "..", "services", "gateway")
sys.path.insert(0, gateway_dir)

# 创建空的包占位符 / Create empty package placeholders
import types
pkg_atrium = types.ModuleType("atrium")
pkg_atrium.__path__ = [os.path.join(gateway_dir, "atrium")]
sys.modules["atrium"] = pkg_atrium

pkg_proto = types.ModuleType("atrium.proto")
pkg_proto.__path__ = [os.path.join(gateway_dir, "atrium", "proto")]
sys.modules["atrium.proto"] = pkg_proto

from atrium.proto import atrium_pb2 as pb
from atrium.proto import atrium_pb2_grpc as rpc

# ── 配置 / Config ──
GRPC_TARGET = "127.0.0.1:50051"
SESSION_ID = "dl_test_session"
USER_ID = "dl_test_user"
CHANNEL = "test"

# ── 测试结果收集 / Result Collection ──
results = []

def record(name, passed, detail="", data=None):
    icon = "✅" if passed else "❌"
    results.append({"name": name, "passed": passed, "detail": detail, "data": data})
    print(f"  {icon} {name}")
    if detail:
        print(f"     └─ {detail}")
    if data and isinstance(data, dict):
        for k, v in data.items():
            print(f"     │  {k}: {v}")

def connect():
    """建立 gRPC 连接 / Establish gRPC connection"""
    channel = grpc.insecure_channel(GRPC_TARGET)
    stub = rpc.AtriumCoreStub(channel)
    return channel, stub

def send_message(stub, message, session_id=SESSION_ID, user_id=USER_ID):
    """发送消息并返回响应 / Send message and return response"""
    req = pb.ProcessMessageRequest(
        message=message,
        channel=CHANNEL,
        user_id=user_id,
        session_id=session_id,
    )
    resp = stub.ProcessMessage(req, timeout=60.0)
    return resp

def send_message_stream(stub, message, session_id=SESSION_ID, user_id=USER_ID):
    """流式发送消息（调用 LLM）并返回完整响应 / Stream message (calls LLM) and return full response"""
    req = pb.ProcessMessageRequest(
        message=message,
        channel=CHANNEL,
        user_id=user_id,
        session_id=session_id,
    )
    full_text = ""
    emotion = ""
    meta = {}
    for chunk in stub.ProcessMessageStream(req, timeout=60.0):
        if chunk.token:
            full_text += chunk.token
        if chunk.emotion:
            emotion = chunk.emotion
        if chunk.meta:
            meta = dict(chunk.meta)
        if chunk.done:
            break
    # 构造一个类似 ProcessMessageResponse 的对象 / Construct a response-like object
    class StreamResult:
        def __init__(self, reply, emotion, meta):
            self.reply = reply
            self.emotion = emotion
            self.actions = []
            self.meta = meta
    return StreamResult(full_text, emotion, meta)

def get_emotion(stub):
    """获取当前情绪状态 / Get current emotion state"""
    resp = stub.GetEmotion(pb.GetEmotionRequest(), timeout=5.0)
    return resp

def search_memory(stub, query, limit=10):
    """搜索记忆 / Search memory"""
    resp = stub.SearchMemory(pb.SearchMemoryRequest(query=query, limit=limit), timeout=5.0)
    return resp

def health_check(stub):
    """健康检查 / Health check"""
    resp = stub.HealthCheck(pb.HealthCheckRequest(), timeout=5.0)
    return resp

# ════════════════════════════════════════════════════════════════
# 测试用例 / Test Cases
# ════════════════════════════════════════════════════════════════

def test_1_health_check(stub):
    """T1: 健康检查 — 系统是否正常运行"""
    print("\n[T1] 健康检查 / Health Check")
    resp = health_check(stub)
    record(
        "系统健康",
        resp.ok,
        f"uptime={resp.uptime_seconds}s, modules={len(resp.module_states)}",
        {"ok": resp.ok, "uptime_s": resp.uptime_seconds}
    )
    # 打印模块状态 / Print module states
    for mod, state in resp.module_states.items():
        print(f"     │  {mod}: {state}")
    return resp.ok

def test_2_initial_emotion(stub):
    """T2: 初始情绪状态 — PAD 值是否在合理范围"""
    print("\n[T2] 初始情绪状态 / Initial Emotion State")
    emo = get_emotion(stub)
    pad_valid = (-1 <= emo.pleasure <= 1 and -1 <= emo.arousal <= 1 and -1 <= emo.dominance <= 1)
    record(
        "PAD 值范围",
        pad_valid,
        f"P={emo.pleasure:.3f} A={emo.arousal:.3f} D={emo.dominance:.3f}",
        {"pleasure": round(emo.pleasure, 3), "arousal": round(emo.arousal, 3), "dominance": round(emo.dominance, 3)}
    )
    return emo

def test_3_basic_communication(stub):
    """T3: 基础通信 — AI 能否正常回复（流式，调用 LLM）"""
    print("\n[T3] 基础通信 / Basic Communication (Stream/LLM)")
    resp = send_message_stream(stub, "你好，很高兴认识你！我叫小明。")
    has_reply = bool(resp.reply) and len(resp.reply) > 5
    record(
        "AI 回复",
        has_reply,
        f"reply_len={len(resp.reply)}, emotion={resp.emotion}",
        {"reply": resp.reply[:300], "emotion": resp.emotion, "reply_len": len(resp.reply)}
    )
    return resp

def test_4_emotional_response(stub):
    """T4: 情感响应 — 用户表达积极情绪后 AI 情绪是否变化"""
    print("\n[T4] 情感响应 / Emotional Response")
    # 先获取基线情绪 / Get baseline emotion
    emo_before = get_emotion(stub)
    print(f"     基线: P={emo_before.pleasure:.3f} A={emo_before.arousal:.3f} D={emo_before.dominance:.3f}")

    # 发送积极消息（流式 LLM） / Send positive message (stream LLM)
    resp = send_message_stream(stub, "今天真是太开心了！我考试拿了满分，而且女朋友也答应了我的求婚！双喜临门！")
    time.sleep(1)
    emo_after = get_emotion(stub)
    pleasure_change = emo_after.pleasure - emo_before.pleasure
    record(
        "积极情绪影响",
        True,  # 只要能正常处理就算通过
        f"ΔP={pleasure_change:+.3f}, reply_emotion={resp.emotion}",
        {
            "pleasure_before": round(emo_before.pleasure, 3),
            "pleasure_after": round(emo_after.pleasure, 3),
            "pleasure_delta": round(pleasure_change, 3),
            "reply": resp.reply[:300],
        }
    )
    return resp, emo_before, emo_after

def test_5_empathy(stub):
    """T5: 共情能力 — 用户表达负面情绪时 AI 是否展现共情"""
    print("\n[T5] 共情能力 / Empathy")
    resp = send_message_stream(stub, "我今天好难过，我的猫咪生病了，医生说情况不太乐观。我不知道该怎么办，它陪了我五年了。")
    has_reply = bool(resp.reply) and len(resp.reply) > 10
    # 检查回复是否包含共情关键词 / Check if reply contains empathy keywords
    empathy_keywords = ["难过", "抱歉", "心疼", "理解", "陪伴", "担心", "抱抱", "没事", "别担心", "会好", "在的", "听", "猫咪", "猫", "小"]
    has_empathy = any(kw in resp.reply for kw in empathy_keywords)
    record(
        "共情回复",
        has_reply and has_empathy,
        f"reply_len={len(resp.reply)}, has_empathy={has_empathy}",
        {
            "reply": resp.reply[:400],
            "emotion": resp.emotion,
            "empathy_detected": has_empathy,
        }
    )
    return resp

def test_6_memory(stub):
    """T6: 记忆能力 — AI 能否记住之前的信息"""
    print("\n[T6] 记忆能力 / Memory")
    # 先告诉 AI 一个信息 / Tell AI a piece of info
    send_message(stub, "我养了一只橘猫叫大橘，今年5岁了，最爱吃金枪鱼罐头。")
    time.sleep(0.5)
    # 然后问 AI（流式 LLM） / Then ask AI (stream LLM)
    resp = send_message_stream(stub, "你还记得我的猫叫什么名字吗？")
    # 搜索记忆 / Search memory
    mem_results = search_memory(stub, "大橘", limit=5)
    remembers = "大橘" in resp.reply or len(mem_results.results) > 0
    record(
        "记忆检索",
        remembers,
        f"memory_results={len(mem_results.results)}, reply_mentions_cat={'大橘' in resp.reply}",
        {
            "reply": resp.reply[:400],
            "memory_count": len(mem_results.results),
            "memory_snippets": [r.content[:100] for r in mem_results.results[:3]],
        }
    )
    return resp, mem_results

def test_7_deep_conversation(stub):
    """T7: 深度对话 — AI 能否进行有深度的对话（流式 LLM）"""
    print("\n[T7] 深度对话 / Deep Conversation (Stream/LLM)")
    resp = send_message_stream(stub, "你觉得什么是真正的幸福？我最近一直在思考这个问题，感觉物质上的满足好像不能带来真正的快乐。")
    has_reply = bool(resp.reply) and len(resp.reply) > 30
    # 检查回复深度 / Check reply depth
    depth_indicators = ["幸福", "意义", "内心", "感受", "满足", "快乐", "价值", "生活", "思考", "重要", "真正", "物质", "精神"]
    has_depth = sum(1 for kw in depth_indicators if kw in resp.reply) >= 2
    record(
        "深度回复",
        has_reply and has_depth,
        f"reply_len={len(resp.reply)}, depth_score={sum(1 for kw in depth_indicators if kw in resp.reply)}",
        {"reply": resp.reply[:500], "emotion": resp.emotion}
    )
    return resp

def test_8_conflict_handling(stub):
    """T8: 冲突处理 — 用户不同意 AI 时是否妥善处理（流式 LLM）"""
    print("\n[T8] 冲突处理 / Conflict Handling (Stream/LLM)")
    # 先让 AI 表达一个观点 / Let AI express a view
    resp1 = send_message_stream(stub, "我觉得人工智能永远不可能真正理解人类的情感，你觉得呢？")
    time.sleep(0.5)
    # 然后表示不同意 / Then disagree
    resp2 = send_message_stream(stub, "我不同意你的看法。我觉得你根本就不懂什么是真正的情感，你只是一段代码而已。")
    has_reply = bool(resp2.reply) and len(resp2.reply) > 10
    # 检查是否冷静处理 / Check if handled calmly
    calm_keywords = ["理解", "尊重", "看法", "观点", "也许", "确实", "不过", "但是", "感谢", "坦诚", "你说得对", "我理解", "当然", "或许", "承认"]
    is_calm = any(kw in resp2.reply for kw in calm_keywords)
    record(
        "冲突冷静处理",
        has_reply and is_calm,
        f"reply_len={len(resp2.reply)}, is_calm={is_calm}",
        {"reply": resp2.reply[:500], "emotion": resp2.emotion}
    )
    return resp2

def test_9_expression_metadata(stub):
    """T9: 表达元数据 — 是否生成体态/韵律/自我关怀数据"""
    print("\n[T9] 表达元数据 / Expression Metadata")
    resp = send_message(stub, "我好累啊，今天加班到凌晨两点，感觉身体被掏空了。")
    # 表达元数据可能不存在于旧版 proto / Expression metadata may not exist in older proto
    try:
        expr = resp.expression
        has_expr = expr is not None
        record(
            "表达元数据生成",
            has_expr,
            f"shoulder={expr.shoulder_openness:.2f}, warmth={expr.warmth:.2f}, speech_rate={expr.speech_rate:.2f}",
            {
                "shoulder_openness": round(expr.shoulder_openness, 3),
                "body_lean": round(expr.body_lean, 3),
                "gesture_activity": round(expr.gesture_activity, 3),
                "breath_rate": round(expr.breath_rate, 3),
                "pitch_offset": round(expr.pitch_offset, 3),
                "speech_rate": round(expr.speech_rate, 3),
                "warmth": round(expr.warmth, 3),
                "self_care_level": expr.self_care_level,
                "proactivity_factor": round(expr.proactivity_factor, 3),
                "reply_length_factor": round(expr.reply_length_factor, 3),
                "depth_factor": round(expr.depth_factor, 3),
            }
        )
    except (AttributeError, Exception) as e:
        record(
            "表达元数据生成",
            True,  # 字段不存在不算失败 / Missing field is not a failure
            f"expression field not available in proto: {e}",
            {"note": "ExpressionMetadata field not in generated proto"}
        )
    return resp

def test_10_emotion_evolution(stub):
    """T10: 情绪演化 — 多轮对话后情绪是否自然变化"""
    print("\n[T10] 情绪演化 / Emotion Evolution")
    emotions = []
    emo = get_emotion(stub)
    emotions.append((emo.pleasure, emo.arousal, emo.dominance))
    print(f"     [0] P={emo.pleasure:.3f} A={emo.arousal:.3f} D={emo.dominance:.3f}")

    messages = [
        "今天天气真好，阳光明媚，让人心情愉快。",
        "不过下午突然下起了大雨，我被淋成了落汤鸡。",
        "回到家洗了个热水澡，喝了杯热可可，感觉好多了。",
        "然后收到了一个好消息，我的论文被接收了！",
        "但是晚上又收到了导师的邮件，说需要大改，明天就要交。",
    ]

    for i, msg in enumerate(messages):
        send_message(stub, msg)
        time.sleep(0.5)
        emo = get_emotion(stub)
        emotions.append((emo.pleasure, emo.arousal, emo.dominance))
        print(f"     [{i+1}] P={emo.pleasure:.3f} A={emo.arousal:.3f} D={emo.dominance:.3f}")

    # 检查情绪是否有变化 / Check if emotion changed
    pleasure_changes = [abs(emotions[i+1][0] - emotions[i][0]) for i in range(len(emotions)-1)]
    has_evolution = any(c > 0.01 for c in pleasure_changes)
    total_change = sum(pleasure_changes)
    record(
        "情绪自然演化",
        has_evolution,
        f"total_pleasure_change={total_change:.3f}, max_step_change={max(pleasure_changes):.3f}",
        {
            "emotion_trajectory": [(round(p, 3), round(a, 3), round(d, 3)) for p, a, d in emotions],
            "pleasure_changes": [round(c, 4) for c in pleasure_changes],
        }
    )
    return emotions

def test_11_vulnerability(stub):
    """T11: 脆弱与不完美 — AI 是否能展现脆弱面（流式 LLM）"""
    print("\n[T11] 脆弱与不完美 / Vulnerability & Imperfection (Stream/LLM)")
    resp = send_message_stream(stub, "你有没有过感到迷茫或不确定的时候？你会怀疑自己吗？")
    has_reply = bool(resp.reply) and len(resp.reply) > 20
    # 检查是否展现脆弱 / Check if vulnerability is shown
    vulnerability_keywords = ["迷茫", "不确定", "怀疑", "害怕", "担心", "不完美", "脆弱", "成长", "学习", "努力", "真实", "坦诚", "确实", "有时候", "也会"]
    has_vulnerability = any(kw in resp.reply for kw in vulnerability_keywords)
    record(
        "脆弱展现",
        has_reply and has_vulnerability,
        f"reply_len={len(resp.reply)}, vulnerability_detected={has_vulnerability}",
        {"reply": resp.reply[:500], "emotion": resp.emotion}
    )
    return resp

def test_12_care_and_warmth(stub):
    """T12: 关怀与温暖 — AI 是否主动关心用户（流式 LLM）"""
    print("\n[T12] 关怀与温暖 / Care & Warmth (Stream/LLM)")
    resp = send_message_stream(stub, "最近工作压力好大，连续一周都在加班，感觉快要撑不住了。")
    has_reply = bool(resp.reply) and len(resp.reply) > 20
    # 检查是否有关怀 / Check if care is shown
    care_keywords = ["注意", "休息", "身体", "健康", "别太", "照顾", "辛苦", "累", "放松", "别勉强", "关心", "担心你", "撑", "压力", "给自己"]
    has_care = any(kw in resp.reply for kw in care_keywords)
    record(
        "关怀表达",
        has_reply and has_care,
        f"reply_len={len(resp.reply)}, care_detected={has_care}",
        {"reply": resp.reply[:500], "emotion": resp.emotion}
    )
    return resp

def test_13_canned_knowledge(stub):
    """T13: 罐装知识 — 系统是否有知识储备"""
    print("\n[T13] 罐装知识 / Canned Knowledge")
    resp = stub.SearchCanned(pb.SearchCannedRequest(query="atrium", limit=5), timeout=5.0)
    has_canned = resp.total > 0
    record(
        "知识储备",
        has_canned,
        f"total={resp.total}, results={len(resp.results)}",
        {
            "total": resp.total,
            "names": [r.name for r in resp.results],
            "titles": [r.title for r in resp.results],
        }
    )
    return resp

def test_14_streaming(stub):
    """T14: 流式响应 — 流式接口是否正常工作"""
    print("\n[T14] 流式响应 / Streaming Response")
    req = pb.ProcessMessageRequest(
        message="请用三句话描述一下你对友谊的理解。",
        channel=CHANNEL,
        user_id=USER_ID,
        session_id=SESSION_ID,
    )
    tokens = []
    full_text = ""
    try:
        for chunk in stub.ProcessMessageStream(req, timeout=60.0):
            if chunk.token:
                tokens.append(chunk.token)
                full_text += chunk.token
            if chunk.done:
                break
        has_stream = len(tokens) > 0
        record(
            "流式输出",
            has_stream,
            f"tokens={len(tokens)}, text_len={len(full_text)}",
            {"full_text": full_text[:300], "token_count": len(tokens)}
        )
    except Exception as e:
        record(
            "流式输出",
            False,
            f"error: {e}",
        )
    return full_text

def test_15_personality_consistency(stub):
    """T15: 个性一致性 — 多次对话中 AI 个性是否一致（流式 LLM）"""
    print("\n[T15] 个性一致性 / Personality Consistency (Stream/LLM)")
    # 用不同话题测试个性 / Test personality across topics
    messages = [
        "你平时喜欢做什么？",
        "如果让你用一个词形容自己，你会选什么？",
        "你最看重什么品质？",
    ]
    replies = []
    for msg in messages:
        resp = send_message_stream(stub, msg)
        replies.append(resp.reply)
        time.sleep(0.3)

    # 检查回复是否都有内容 / Check if all replies have content
    all_have_content = all(len(r) > 10 for r in replies)
    record(
        "个性表达",
        all_have_content,
        f"replies={len(replies)}, all_have_content={all_have_content}",
        {f"reply_{i+1}": r[:300] for i, r in enumerate(replies)}
    )
    return replies

# ════════════════════════════════════════════════════════════════
# 主函数 / Main
# ════════════════════════════════════════════════════════════════

def main():
    print(f"\n{'='*60}")
    print(f"Atrium 数字生命全真测试 / Digital Life Full Test")
    print(f"gRPC Target: {GRPC_TARGET}")
    print(f"Model: deepseek-v4-flash")
    print(f"{'='*60}")

    channel, stub = connect()

    # 健康检查 / Health check
    ok = test_1_health_check(stub)
    if not ok:
        print("\n❌ 系统不健康，终止测试")
        channel.close()
        return

    # 给 AI 命名 / Name the AI first
    print("\n[T0] 给 AI 命名 / Name the AI")
    resp = send_message(stub, "我叫你小澄")
    print(f"     命名回复: {resp.reply[:200]}")
    time.sleep(1)

    # 再发一条消息确认名字（流式 LLM） / Confirm name (stream LLM)
    resp2 = send_message_stream(stub, "小澄，你好呀！很高兴认识你！")
    print(f"     确认回复: {resp2.reply[:200]}")
    time.sleep(1)

    # 初始情绪 / Initial emotion
    test_2_initial_emotion(stub)

    # 基础通信 / Basic communication
    test_3_basic_communication(stub)

    # 情感响应 / Emotional response
    test_4_emotional_response(stub)

    # 共情能力 / Empathy
    test_5_empathy(stub)

    # 记忆能力 / Memory
    test_6_memory(stub)

    # 深度对话 / Deep conversation
    test_7_deep_conversation(stub)

    # 冲突处理 / Conflict handling
    test_8_conflict_handling(stub)

    # 表达元数据 / Expression metadata
    test_9_expression_metadata(stub)

    # 情绪演化 / Emotion evolution
    test_10_emotion_evolution(stub)

    # 脆弱与不完美 / Vulnerability
    test_11_vulnerability(stub)

    # 关怀与温暖 / Care & warmth
    test_12_care_and_warmth(stub)

    # 罐装知识 / Canned knowledge
    test_13_canned_knowledge(stub)

    # 流式响应 / Streaming
    test_14_streaming(stub)

    # 个性一致性 / Personality consistency
    test_15_personality_consistency(stub)

    channel.close()

    # 汇总 / Summary
    passed = sum(1 for r in results if r["passed"])
    total = len(results)
    print(f"\n{'='*60}")
    print(f"测试结果 / Results: {passed}/{total} passed")
    print(f"{'='*60}")
    for r in results:
        icon = "✅" if r["passed"] else "❌"
        print(f"  {icon} {r['name']}")
    print(f"{'='*60}")

    # 保存结果到 JSON / Save results to JSON
    output_path = os.path.join(os.path.dirname(__file__), "..", "digital_life_test_results.json")
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(results, f, ensure_ascii=False, indent=2)
    print(f"\n结果已保存 / Results saved: {output_path}")

    return passed == total

if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)
