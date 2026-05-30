use crate::models::{ChatMessage, ContentDeltaEvent, SearchPlan, SearchResult, SendMessageRequest};
use crate::search::fallback_search_plan;
use crate::security::{
    DEEPSEEK_REQUEST_TIMEOUT_SECS, MAX_ASSISTANT_OUTPUT_CHARS, MAX_SSE_BUFFER_BYTES,
};
use crate::tools::run_tool_call;
use futures_util::StreamExt;
use reqwest::Client;
use serde::Serialize;
use serde_json::{json, Value};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Serialize)]
struct ApiMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolCall {
    id: String,
    function: ToolFunction,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolFunction {
    name: String,
    arguments: String,
}

struct SearchContext {
    plan: SearchPlan,
    results: Vec<SearchResult>,
}

pub async fn send_deepseek_message(
    app: AppHandle,
    api_key: String,
    base_url: String,
    request: SendMessageRequest,
    history: Vec<ChatMessage>,
    assistant_message_id: String,
    cancellation: CancellationToken,
) -> Result<ChatMessage, String> {
    let mut api_messages = to_api_messages(history);

    if request.search_enabled {
        match search_current_prompt(&app, &api_key, &base_url, &request.model, &request.content)
            .await
        {
            Ok(search_context) => {
                api_messages.push(ApiMessage {
                    role: "system".to_string(),
                    content: format!(
                        "Use the following Tavily web search evidence to answer the user's latest question. Cite URLs when they are relevant.\n\nSearch intent: {}\nMust-have checks from the search plan: {}\nAnswer guidance from the search plan: {}\n\nWrite the answer in the structure that best fits the user's question. Distinguish facts, source type, inference, and uncertainty when it matters. Do not claim missing evidence as confirmed.\n\nSearch results:\n{}",
                        search_context
                            .plan
                            .intent
                            .as_deref()
                            .unwrap_or("Not specified"),
                        format_string_list(&search_context.plan.must_have),
                        search_context
                            .plan
                            .answer_guidance
                            .as_deref()
                            .unwrap_or("Use the available evidence directly and avoid overclaiming."),
                        format_search_results(&search_context.results)
                    ),
                    reasoning_content: None,
                    tool_call_id: None,
                    tool_calls: None,
                });
            }
            Err(error) => {
                let _ = app.emit(
                    "chat:error",
                    json!({
                        "conversationId": request.conversation_id,
                        "messageId": assistant_message_id,
                        "error": format!("{error} 已改用模型自身知识回答。")
                    }),
                );
            }
        }
    }

    stream_final_answer(
        app,
        api_key,
        base_url,
        request,
        api_messages,
        assistant_message_id,
        cancellation,
    )
    .await
}

async fn search_current_prompt(
    app: &AppHandle,
    deepseek_api_key: &str,
    base_url: &str,
    model: &str,
    query: &str,
) -> Result<SearchContext, String> {
    let plan = create_search_plan(deepseek_api_key, base_url, model, query)
        .await
        .unwrap_or_else(|_| fallback_search_plan(query, 5));
    let arguments = json!({
        "query": query,
        "max_results": 5,
        "plan": plan
    })
    .to_string();
    let tool_call = ToolCall {
        id: uuid::Uuid::new_v4().to_string(),
        function: ToolFunction {
            name: "web_search".to_string(),
            arguments: arguments.clone(),
        },
    };
    let _ = app.emit("chat:tool-call", &tool_call);
    let results = run_tool_call("web_search", &arguments).await?;
    let _ = app.emit("chat:search-results", &results);
    Ok(SearchContext { plan, results })
}

async fn create_search_plan(
    api_key: &str,
    base_url: &str,
    model: &str,
    query: &str,
) -> Result<SearchPlan, String> {
    let response = deepseek_client()?
        .post(format!(
            "{}/chat/completions",
            base_url.trim_end_matches('/')
        ))
        .bearer_auth(api_key)
        .json(&json!({
            "model": model,
            "messages": [
                {
                    "role": "system",
                    "content": search_planner_prompt()
                },
                {
                    "role": "user",
                    "content": query
                }
            ],
            "stream": false,
            "temperature": 0
        }))
        .send()
        .await
        .map_err(|_| "搜索规划请求失败或超时。".to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        return Err(format!("搜索规划返回错误：{}", status));
    }

    let body = response
        .json::<Value>()
        .await
        .map_err(|_| "搜索规划解析失败。".to_string())?;
    let content = body["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| "搜索规划内容为空。".to_string())?;
    let json_text =
        extract_json_object(content).ok_or_else(|| "搜索规划不是 JSON。".to_string())?;
    serde_json::from_str::<SearchPlan>(&json_text).map_err(|_| "搜索规划 JSON 无效。".to_string())
}

fn search_planner_prompt() -> &'static str {
    r#"You are a search query planner for a general web search tool. Return only valid JSON with this shape:
{
  "intent": "brief description of what the user wants to find",
  "queries": [
    {
      "query": "short web search query, not the user's full question",
      "topic": "general or news",
      "search_depth": "basic or advanced",
      "max_results": 5,
      "include_domains": [],
      "exclude_domains": [],
      "start_date": null,
      "end_date": null,
      "include_raw_content": false
    }
  ],
  "must_have": ["key facts that should be covered before answering"],
  "answer_guidance": "brief guidance for using sources and uncertainty"
}

Rules:
- Plan ordinary web search, not a research project.
- Generate 3 to 8 short queries.
- Do not copy the user's long question as a query.
- Do not assume fixed categories or fixed sources.
- Use include_domains only when the user explicitly requests a source, site, publisher, or domain.
- Use topic "news" only when the user is asking for current or recent news.
- Use advanced/raw content only when source detail, recency, or exact evidence matters.
- Dates must be YYYY-MM-DD or null."#
}

async fn stream_final_answer(
    app: AppHandle,
    api_key: String,
    base_url: String,
    request: SendMessageRequest,
    messages: Vec<ApiMessage>,
    assistant_message_id: String,
    cancellation: CancellationToken,
) -> Result<ChatMessage, String> {
    let (thinking, effort) = thinking_payload(&request.thinking_mode);
    let response = deepseek_client()?
        .post(format!(
            "{}/chat/completions",
            base_url.trim_end_matches('/')
        ))
        .bearer_auth(api_key)
        .json(&json!({
            "model": request.model,
            "messages": messages,
            "stream": true,
            "thinking": thinking,
            "reasoning_effort": effort
        }))
        .send()
        .await
        .map_err(|_| "网络请求失败或超时，请检查网络连接。".to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        return Err(format!("DeepSeek API 返回错误：{}", status));
    }

    let mut content = String::new();
    let mut reasoning_content = String::new();
    let mut buffer = String::new();
    let mut stream = response.bytes_stream();

    loop {
        tokio::select! {
            _ = cancellation.cancelled() => {
                return Err("生成已停止。".to_string());
            }
            chunk = stream.next() => {
                let Some(chunk) = chunk else { break; };
                let chunk = chunk.map_err(|_| "流式输出中断。".to_string())?;
                if chunk.len() > MAX_SSE_BUFFER_BYTES {
                    return Err("响应片段过大，已停止生成。".to_string());
                }
                buffer.push_str(&String::from_utf8_lossy(&chunk));
                if buffer.len() > MAX_SSE_BUFFER_BYTES {
                    return Err("响应缓冲区过大，已停止生成。".to_string());
                }
                while let Some(index) = buffer.find('\n') {
                    let line = buffer[..index].trim().to_string();
                    buffer = buffer[index + 1..].to_string();
                    if !line.starts_with("data:") {
                        continue;
                    }
                    let data = line.trim_start_matches("data:").trim();
                    if data == "[DONE]" {
                        break;
                    }
                    let Ok(value) = serde_json::from_str::<Value>(data) else {
                        continue;
                    };
                    let delta = &value["choices"][0]["delta"];
                    if let Some(piece) = delta.get("reasoning_content").and_then(Value::as_str) {
                        append_limited(&mut reasoning_content, piece, content.chars().count())?;
                        let _ = app.emit("chat:reasoning-delta", ContentDeltaEvent {
                            conversation_id: request.conversation_id.clone(),
                            message_id: assistant_message_id.clone(),
                            delta: piece.to_string(),
                        });
                    }
                    if let Some(piece) = delta.get("content").and_then(Value::as_str) {
                        append_limited(&mut content, piece, reasoning_content.chars().count())?;
                        let _ = app.emit("chat:content-delta", ContentDeltaEvent {
                            conversation_id: request.conversation_id.clone(),
                            message_id: assistant_message_id.clone(),
                            delta: piece.to_string(),
                        });
                    }
                }
            }
        }
    }

    Ok(ChatMessage {
        id: assistant_message_id,
        conversation_id: request.conversation_id,
        role: "assistant".to_string(),
        content,
        reasoning_content: if reasoning_content.is_empty() {
            None
        } else {
            Some(reasoning_content)
        },
        tool_calls_json: None,
        tool_result_json: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    })
}

fn deepseek_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(DEEPSEEK_REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|_| "无法初始化网络客户端。".to_string())
}

fn append_limited(target: &mut String, piece: &str, other_chars: usize) -> Result<(), String> {
    let next_total = target
        .chars()
        .count()
        .saturating_add(other_chars)
        .saturating_add(piece.chars().count());
    if next_total > MAX_ASSISTANT_OUTPUT_CHARS {
        return Err(format!(
            "回复过长，请控制在 {} 字符以内。",
            MAX_ASSISTANT_OUTPUT_CHARS
        ));
    }
    target.push_str(piece);
    Ok(())
}

fn to_api_messages(history: Vec<ChatMessage>) -> Vec<ApiMessage> {
    history
        .into_iter()
        .rev()
        .take(20)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .filter(|message| message.role != "tool")
        .map(|message| ApiMessage {
            role: message.role,
            content: message.content,
            reasoning_content: None,
            tool_call_id: None,
            tool_calls: None,
        })
        .collect()
}

fn thinking_payload(mode: &str) -> (Value, Option<&'static str>) {
    match mode {
        "high" => (json!({ "type": "enabled" }), Some("high")),
        "max" => (json!({ "type": "enabled" }), Some("max")),
        _ => (json!({ "type": "disabled" }), None),
    }
}

fn format_search_results(results: &[SearchResult]) -> String {
    if results.is_empty() {
        return "No search results found.".to_string();
    }
    serde_json::to_string(results).unwrap_or_else(|_| "[]".to_string())
}

fn format_string_list(items: &[String]) -> String {
    if items.is_empty() {
        "None specified".to_string()
    } else {
        serde_json::to_string(items).unwrap_or_else(|_| "[]".to_string())
    }
}

fn extract_json_object(content: &str) -> Option<String> {
    let trimmed = content.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return Some(trimmed.to_string());
    }

    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    if start < end {
        Some(trimmed[start..=end].to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_limited_rejects_overlarge_assistant_output() {
        let mut content = "x".repeat(MAX_ASSISTANT_OUTPUT_CHARS);
        assert!(append_limited(&mut content, "x", 0).is_err());
    }
}
