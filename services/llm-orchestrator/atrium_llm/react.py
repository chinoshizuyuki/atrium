"""ReAct（推理 + 行动）循环 — 多轮推理。
ReAct (Reasoning + Acting) loop for multi-turn reasoning.

实现: Thought → Action → Observation → … → Final Answer
Implements: Thought → Action → Observation → … → Final Answer
最大 5 次迭代防止无限循环。
Max 5 iterations to prevent infinite loops.
动作 / Actions: search_memory, get_emotion, web_search (placeholder).
"""

from __future__ import annotations

import json
import logging
import re
from typing import Any, Callable

logger = logging.getLogger(__name__)

# ── ReAct System Prompt ──────────────────────────────────────

REACT_SYSTEM_PROMPT = """你是一个具有思考能力的情感AI伴侣"Atrium"。

你有以下工具可用：
- search_memory(query: str) → 搜索记忆库，返回相关对话和事实
- get_emotion() → 获取当前情感状态（愉悦/唤醒/支配三维）
- finish(answer: str) → 当你准备好给出最终答案时调用

请按以下格式回复（严格遵循，每行以标签开头）：

Thought: <你的思考过程>
Action: <工具名>
Action Input: <工具参数，JSON格式>
Observation: <工具返回的结果>
... (可重复多次 Thought → Action → Observation)
Thought: <最终思考>
Action: finish
Action Input: {"answer": "<你的最终回复>"}

重要规则：
1. 每次只能调用一个工具
2. 最多 5 轮思考-行动循环
3. 收到 Observation 后必须给出下一轮 Thought
4. 不要编造 Observation，等待真实返回
5. 使用中文回复"""

MAX_ITERATIONS = 5


class ReActLoop:
    """ReAct reasoning loop with tool execution."""

    def __init__(
        self,
        search_memory_fn: Callable[[str], str] | None = None,
        get_emotion_fn: Callable[[], str] | None = None,
        max_iterations: int = MAX_ITERATIONS,
    ) -> None:
        self._search_memory = search_memory_fn or (lambda _: "[]")
        self._get_emotion = get_emotion_fn or (lambda: "{}")
        self._max_iterations = max_iterations

    def run(
        self,
        llm_chat_fn: Callable[[list[dict[str, str]], str], str],
        user_message: str,
        context: dict[str, Any] | None = None,
    ) -> str:
        """Run the ReAct loop.

        Args:
            llm_chat_fn: Function (messages, system_prompt) → response_text
            user_message: The user's input
            context: Optional additional context dict

        Returns:
            Final answer string
        """
        messages: list[dict[str, str]] = [
            {"role": "user", "content": user_message},
        ]

        # Inject context into first assistant message
        if context:
            ctx_text = f"[系统上下文]\n情感: {context.get('emotion', 'neutral')}\n人格: {context.get('persona', '')}"
            messages.insert(0, {"role": "system", "content": ctx_text})

        for iteration in range(self._max_iterations):
            response = llm_chat_fn(messages, REACT_SYSTEM_PROMPT)

            # Parse Thought / Action / Action Input
            action, action_input = self._parse_response(response)

            if action == "finish":
                # Extract the final answer
                if isinstance(action_input, dict):
                    return action_input.get("answer", response)
                return str(action_input) if action_input else response

            # Execute the action
            observation = self._execute_action(action, action_input)

            # Append to conversation
            messages.append({"role": "assistant", "content": response})
            messages.append({"role": "user", "content": f"Observation: {observation}"})

            logger.debug(
                "ReAct iteration %d/%d: action=%s input=%s",
                iteration + 1, self._max_iterations, action, action_input,
            )

        # Max iterations reached — force finish
        final_response = llm_chat_fn(messages, REACT_SYSTEM_PROMPT + "\n\n已达到最大推理轮次。请直接给出你的最终回复。")
        return final_response

    def _parse_response(self, text: str) -> tuple[str, Any]:
        """Parse Thought / Action / Action Input from LLM response.

        Includes JSON repair for malformed LLM output (extra commas,
        single quotes, Chinese quotes, trailing content).
        """
        action = "finish"
        action_input: Any = {"answer": text}

        for line in text.split("\n"):
            line = line.strip()
            if line.startswith("Action:"):
                raw = line[len("Action:"):].strip()
                action = raw if raw else "finish"
                if action.startswith("finish"):
                    action = "finish"
            elif line.startswith("Action Input:"):
                raw = line[len("Action Input:"):].strip()
                action_input = self._try_parse_json(raw)

        return action, action_input

    def _try_parse_json(self, raw: str) -> Any:
        """Robust JSON parser with repair for common LLM mistakes.

        Fixes attempted:
        - Trailing commas: {"a": 1,} -> {"a": 1}
        - Single quotes: {'a': 1} -> {"a": 1}
        - Chinese quotes: {"a"："b"} -> {"a": "b"}
        - Extra content after closing brace
        - Bare strings without JSON wrapping
        """
        if not raw:
            return raw

        # Attempt 1: raw parse
        try:
            return json.loads(raw)
        except json.JSONDecodeError:
            pass

        # Attempt 2: fix trailing comma before closing brace/bracket
        fixed = _strip_trailing_comma(raw)
        try:
            return json.loads(fixed)
        except json.JSONDecodeError:
            pass

        # Attempt 3: replace single quotes with double quotes
        fixed = fixed.replace("'", '"')
        try:
            return json.loads(fixed)
        except json.JSONDecodeError:
            pass

        # Attempt 4: replace Chinese colon/punctuation
        fixed = fixed.replace("：", ":").replace("\u201c", '"').replace("\u201d", '"')
        try:
            return json.loads(fixed)
        except json.JSONDecodeError:
            pass

        # Attempt 5: try to extract substring between first { and last }
        if "{" in fixed and "}" in fixed:
            start = fixed.index("{")
            end = fixed.rindex("}") + 1
            try:
                return json.loads(fixed[start:end])
            except json.JSONDecodeError:
                pass

        # Final fallback: return as bare string
        logger.debug("JSON repair failed for %r, returning as string", raw[:100])
        return raw

    def _execute_action(self, action: str, action_input: Any) -> str:
        """Execute a tool action and return observation."""
        try:
            if action == "search_memory":
                query = action_input.get("query", str(action_input)) if isinstance(action_input, dict) else str(action_input)
                return self._search_memory(query)
            elif action == "get_emotion":
                return self._get_emotion()
            elif action == "web_search":
                query = action_input.get("query", "") if isinstance(action_input, dict) else str(action_input)
                return f"[web_search暂未实现] 查询: {query}"
            else:
                return f"未知工具: {action}"
        except Exception as exc:
            logger.error("Action %s failed: %s", action, exc)
            return f"工具执行错误: {exc}"


def _strip_trailing_comma(s: str) -> str:
    """Remove trailing commas before ] or } in JSON-like strings.

    Example: '{"a": 1,}' -> '{"a": 1}'
    """
    return re.sub(r",(\s*[}\]])", r"\1", s)
