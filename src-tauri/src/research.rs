use crate::models::{
    ChatErrorEvent, ChatMessage, ChatStartedEvent, ContentDeltaEvent, PlannedSearchQuery,
    PrepareResearchTaskRequest, PrepareResearchTaskResponse, ResearchActivity, ResearchDepthBudget,
    ResearchPlan, ResearchProgressEvent, ResearchReportDeltaEvent, ResearchSource, ResearchTask,
    ResearchTaskDetail, SearchPlan, SearchResult, StartResearchTaskRequest,
};
use crate::search::tavily_search;
use crate::secrets;
use crate::security::{normalize_settings, MAX_ASSISTANT_OUTPUT_CHARS, MAX_USER_MESSAGE_CHARS};
use crate::AppState;
use chrono::Utc;
use futures_util::StreamExt;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const RESEARCH_REQUEST_TIMEOUT_SECS: u64 = 30 * 60;
const DEFAULT_MAX_ROUNDS: u8 = 5;
const DEFAULT_QUERIES_PER_ROUND: u8 = 6;
const DEFAULT_SOURCE_LIMIT: u16 = 80;
const MAX_DOMAINS: usize = 8;
const MAX_QUERY_CHARS: usize = 120;

#[derive(Debug)]
enum ResearchRunError {
    Cancelled,
    Failed(String),
}

impl From<String> for ResearchRunError {
    fn from(error: String) -> Self {
        Self::Failed(error)
    }
}

impl From<&str> for ResearchRunError {
    fn from(error: &str) -> Self {
        Self::Failed(error.to_string())
    }
}

#[derive(Clone)]
pub struct ResearchRunControl {
    cancellation: CancellationToken,
    paused: Arc<AtomicBool>,
}

impl ResearchRunControl {
    pub fn new() -> Self {
        Self {
            cancellation: CancellationToken::new(),
            paused: Arc::new(AtomicBool::new(false)),
        }
    }

    fn cancel(&self) {
        self.cancellation.cancel();
    }

    fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::SeqCst);
    }
}

#[tauri::command]
pub async fn prepare_research_task(
    request: PrepareResearchTaskRequest,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<PrepareResearchTaskResponse, String> {
    validate_research_prompt(&request.prompt)?;
    let api_key = secrets::get_secret("deepseek_api_key").ok();
    let settings = normalize_settings(state.db.get_settings().unwrap_or_default())?;
    let source_policy = normalize_source_policy(&request.source_policy);
    let domains = sanitize_domains(request.domains);
    let plan = normalize_research_plan(
        fallback_research_plan(&request.prompt, &source_policy, &domains),
        &request.prompt,
        &source_policy,
        &domains,
    );
    let plan_json = serde_json::to_string(&plan).map_err(|_| "研究计划序列化失败。".to_string())?;
    let domains_json =
        serde_json::to_string(&domains).map_err(|_| "研究域名序列化失败。".to_string())?;
    let now = Utc::now().to_rfc3339();
    let user_message = ChatMessage {
        id: Uuid::new_v4().to_string(),
        conversation_id: request.conversation_id.clone(),
        role: "user".to_string(),
        content: format!("Deep Research: {}", request.prompt.trim()),
        reasoning_content: None,
        tool_calls_json: None,
        tool_result_json: None,
        created_at: now.clone(),
    };
    state.db.save_message(&user_message)?;
    touch_conversation(
        &state,
        &request.conversation_id,
        &request.prompt,
        &request.model,
        &now,
    )?;

    let task = ResearchTask {
        id: Uuid::new_v4().to_string(),
        conversation_id: request.conversation_id,
        user_message_id: user_message.id.clone(),
        assistant_message_id: None,
        topic: plan.title.clone(),
        status: "draft".to_string(),
        source_policy: source_policy.clone(),
        domains_json,
        plan_json,
        report: String::new(),
        error: None,
        created_at: now.clone(),
        updated_at: now,
        completed_at: None,
    };
    state.db.save_research_task(&task)?;
    state.db.save_research_activity(&ResearchActivity {
        id: Uuid::new_v4().to_string(),
        task_id: task.id.clone(),
        activity_type: "plan".to_string(),
        title: "已创建基础研究计划".to_string(),
        detail: Some("正在后台细化计划，可先查看或稍后开始研究。".to_string()),
        created_at: Utc::now().to_rfc3339(),
    })?;
    if let Some(api_key) = api_key {
        emit_progress(
            &app,
            &task.id,
            &task.conversation_id,
            "draft",
            "planning",
            0,
            1,
            "正在细化研究计划",
        );

        let task_id = task.id.clone();
        let prompt = request.prompt.clone();
        let model = request.model.clone();
        let source_policy_for_plan = source_policy.clone();
        let domains_for_plan = domains.clone();
        let app_for_task = app.clone();
        tauri::async_runtime::spawn(async move {
            let state = app_for_task.state::<AppState>();
            let refined = create_research_plan(
                &api_key,
                &settings.deepseek_base_url,
                &model,
                &prompt,
                &source_policy_for_plan,
                &domains_for_plan,
            )
            .await
            .map(|plan| {
                normalize_research_plan(plan, &prompt, &source_policy_for_plan, &domains_for_plan)
            });

            let Ok(plan) = refined else {
                let _ = save_activity(
                    &app_for_task,
                    &state,
                    &task_id,
                    "plan_fallback",
                    "继续使用基础研究计划",
                    Some("后台细化计划失败，可直接开始研究或稍后重试。".to_string()),
                );
                return;
            };

            if let Ok(mut task) = state.db.get_research_task(&task_id) {
                if task.status != "draft" {
                    return;
                }
                task.topic = plan.title.clone();
                if let Ok(plan_json) = serde_json::to_string(&plan) {
                    task.plan_json = plan_json;
                    task.updated_at = Utc::now().to_rfc3339();
                    if state.db.save_research_task(&task).is_ok() {
                        let _ = save_activity(
                            &app_for_task,
                            &state,
                            &task_id,
                            "plan_refined",
                            "已细化研究计划",
                            Some("计划已根据模型规划更新。".to_string()),
                        );
                        if let Ok(detail) = state.db.get_research_task_detail(&task_id) {
                            let _ = app_for_task.emit("research:plan-ready", detail);
                        }
                        emit_progress(
                            &app_for_task,
                            &task.id,
                            &task.conversation_id,
                            "draft",
                            "planned",
                            1,
                            1,
                            "研究计划已更新",
                        );
                    }
                }
            }
        });
    } else {
        state.db.save_research_activity(&ResearchActivity {
            id: Uuid::new_v4().to_string(),
            task_id: task.id.clone(),
            activity_type: "plan_fallback".to_string(),
            title: "继续使用基础研究计划".to_string(),
            detail: Some("缺少 DeepSeek Key，已跳过后台计划细化。".to_string()),
            created_at: Utc::now().to_rfc3339(),
        })?;
        emit_progress(
            &app,
            &task.id,
            &task.conversation_id,
            "draft",
            "planned",
            1,
            1,
            "使用基础研究计划",
        );
    }

    Ok(PrepareResearchTaskResponse {
        detail: state.db.get_research_task_detail(&task.id)?,
        user_message,
    })
}

#[tauri::command]
pub async fn start_research_task(
    request: StartResearchTaskRequest,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ResearchTaskDetail, String> {
    let task = state.db.get_research_task(&request.task_id)?;
    if matches!(task.status.as_str(), "running" | "completed") {
        return Err("研究任务已经在运行或已完成。".to_string());
    }
    let _deepseek_key = secrets::get_secret("deepseek_api_key")?;
    let _tavily_key = secrets::get_secret("tavily_api_key")?;

    let assistant_message_id = task
        .assistant_message_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    state
        .db
        .set_research_assistant_message(&task.id, &assistant_message_id)?;
    state
        .db
        .update_research_status(&task.id, "running", None, None)?;
    let control = ResearchRunControl::new();
    {
        let mut runs = state
            .research_runs
            .lock()
            .map_err(|_| "研究任务状态锁定失败".to_string())?;
        runs.insert(task.id.clone(), control.clone());
    }

    let _ = app.emit(
        "chat:started",
        ChatStartedEvent {
            conversation_id: task.conversation_id.clone(),
            message_id: assistant_message_id.clone(),
        },
    );
    emit_progress(
        &app,
        &task.id,
        &task.conversation_id,
        "running",
        "start",
        0,
        DEFAULT_MAX_ROUNDS as u16 + 2,
        "开始深度研究",
    );

    let task_id = task.id.clone();
    let model = request.model.clone();
    let app_for_task = app.clone();
    tauri::async_runtime::spawn(async move {
        let result = execute_research_task(
            app_for_task.clone(),
            task_id.clone(),
            model,
            assistant_message_id,
            control,
        )
        .await;
        let state = app_for_task.state::<AppState>();
        if let Ok(mut runs) = state.research_runs.lock() {
            runs.remove(&task_id);
        }
        match result {
            Ok(()) => {}
            Err(ResearchRunError::Cancelled) => {
                if let Ok(task) = state.db.get_research_task(&task_id) {
                    let completed_at = Utc::now().to_rfc3339();
                    let _ = state.db.update_research_status(
                        &task_id,
                        "cancelled",
                        None,
                        Some(&completed_at),
                    );
                    emit_progress(
                        &app_for_task,
                        &task.id,
                        &task.conversation_id,
                        "cancelled",
                        "cancel",
                        DEFAULT_MAX_ROUNDS as u16 + 2,
                        DEFAULT_MAX_ROUNDS as u16 + 2,
                        "研究已取消",
                    );
                    let _ = app_for_task.emit(
                        "research:done",
                        ChatStartedEvent {
                            conversation_id: task.conversation_id.clone(),
                            message_id: task.assistant_message_id.clone().unwrap_or_default(),
                        },
                    );
                    let _ = app_for_task.emit(
                        "chat:done",
                        ChatStartedEvent {
                            conversation_id: task.conversation_id,
                            message_id: task.assistant_message_id.unwrap_or_default(),
                        },
                    );
                }
            }
            Err(ResearchRunError::Failed(error)) => {
                let completed_at = Utc::now().to_rfc3339();
                let _ = state.db.update_research_status(
                    &task_id,
                    "failed",
                    Some(&error),
                    Some(&completed_at),
                );
                if let Ok(task) = state.db.get_research_task(&task_id) {
                    let _ = app_for_task.emit(
                        "research:error",
                        ChatErrorEvent {
                            conversation_id: task.conversation_id.clone(),
                            message_id: task.assistant_message_id.clone(),
                            error: error.clone(),
                        },
                    );
                    let _ = app_for_task.emit(
                        "chat:error",
                        ChatErrorEvent {
                            conversation_id: task.conversation_id,
                            message_id: task.assistant_message_id,
                            error,
                        },
                    );
                }
            }
        }
    });

    state.db.get_research_task_detail(&task.id)
}

#[tauri::command]
pub async fn pause_research_task(
    task_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ResearchTaskDetail, String> {
    let control = active_research_control(&state, &task_id)?;
    control.set_paused(true);
    state
        .db
        .update_research_status(&task_id, "paused", None, None)?;
    let task = state.db.get_research_task(&task_id)?;
    emit_progress(
        &app,
        &task.id,
        &task.conversation_id,
        "paused",
        "pause",
        0,
        DEFAULT_MAX_ROUNDS as u16 + 2,
        "研究已暂停",
    );
    state.db.get_research_task_detail(&task_id)
}

#[tauri::command]
pub async fn resume_research_task(
    task_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ResearchTaskDetail, String> {
    let control = active_research_control(&state, &task_id)?;
    control.set_paused(false);
    state
        .db
        .update_research_status(&task_id, "running", None, None)?;
    let task = state.db.get_research_task(&task_id)?;
    emit_progress(
        &app,
        &task.id,
        &task.conversation_id,
        "running",
        "resume",
        0,
        DEFAULT_MAX_ROUNDS as u16 + 2,
        "研究已继续",
    );
    state.db.get_research_task_detail(&task_id)
}

#[tauri::command]
pub async fn cancel_research_task(
    task_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ResearchTaskDetail, String> {
    let task = state.db.get_research_task(&task_id)?;
    if task.status != "draft" {
        let control = {
            let runs = state
                .research_runs
                .lock()
                .map_err(|_| "研究任务状态锁定失败".to_string())?;
            runs.get(&task_id)
                .cloned()
                .ok_or_else(|| "研究任务没有正在运行的后台进程，请重新开始。".to_string())?
        };
        control.cancel();
        if let Ok(mut runs) = state.research_runs.lock() {
            runs.remove(&task_id);
        }
    }
    let completed_at = Utc::now().to_rfc3339();
    state
        .db
        .update_research_status(&task_id, "cancelled", None, Some(&completed_at))?;
    emit_progress(
        &app,
        &task.id,
        &task.conversation_id,
        "cancelled",
        "cancel",
        0,
        DEFAULT_MAX_ROUNDS as u16 + 2,
        "研究已取消",
    );
    state.db.get_research_task_detail(&task_id)
}

#[tauri::command]
pub async fn get_research_task(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<ResearchTaskDetail, String> {
    state.db.get_research_task_detail(&task_id)
}

#[tauri::command]
pub async fn get_research_tasks(
    conversation_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ResearchTaskDetail>, String> {
    state.db.get_research_task_details(&conversation_id)
}

#[tauri::command]
pub async fn export_research_task(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let detail = state.db.get_research_task_detail(&task_id)?;
    Ok(format_export_markdown(&detail))
}

async fn execute_research_task(
    app: AppHandle,
    task_id: String,
    model: String,
    assistant_message_id: String,
    control: ResearchRunControl,
) -> Result<(), ResearchRunError> {
    let state = app.state::<AppState>();
    let deepseek_key = secrets::get_secret("deepseek_api_key")?;
    let tavily_key = secrets::get_secret("tavily_api_key")?;
    let settings = normalize_settings(state.db.get_settings().unwrap_or_default())?;
    let task = state.db.get_research_task(&task_id)?;
    let plan: ResearchPlan =
        serde_json::from_str(&task.plan_json).map_err(|_| "研究计划解析失败。".to_string())?;
    let budget = normalized_budget(plan.depth_budget.as_ref());
    let mut all_sources = state.db.get_research_sources(&task_id)?;

    save_activity(
        &app,
        &state,
        &task_id,
        "start",
        "开始执行研究",
        Some(format!("目标：{}", plan.goal)),
    )?;

    for round in 0..budget.max_rounds.unwrap_or(DEFAULT_MAX_ROUNDS) {
        wait_if_paused_or_cancelled(&control).await?;
        let task = state.db.get_research_task(&task_id)?;
        emit_progress(
            &app,
            &task.id,
            &task.conversation_id,
            "running",
            "search",
            round as u16 + 1,
            budget.max_rounds.unwrap_or(DEFAULT_MAX_ROUNDS) as u16 + 2,
            &format!("第 {} 轮搜索", round + 1),
        );
        let round_queries = if round == 0 {
            plan.initial_queries.clone()
        } else {
            create_follow_up_queries(
                &deepseek_key,
                &settings.deepseek_base_url,
                &model,
                &plan,
                &all_sources,
                budget
                    .queries_per_round
                    .unwrap_or(DEFAULT_QUERIES_PER_ROUND),
            )
            .await
            .unwrap_or_else(|_| fallback_follow_up_queries(&plan, &all_sources))
        };
        let round_queries = apply_source_policy(
            round_queries,
            &task.source_policy,
            &parse_domains_json(&task.domains_json),
            budget
                .queries_per_round
                .unwrap_or(DEFAULT_QUERIES_PER_ROUND),
        );
        if round_queries.is_empty() {
            save_activity(&app, &state, &task_id, "gap", "未发现新的补充查询", None)?;
            break;
        }

        save_activity(
            &app,
            &state,
            &task_id,
            "search",
            &format!("第 {} 轮搜索", round + 1),
            Some(
                round_queries
                    .iter()
                    .map(|query| query.query.clone())
                    .collect::<Vec<_>>()
                    .join(" | "),
            ),
        )?;
        let search_plan = SearchPlan {
            intent: Some(plan.goal.clone()),
            queries: round_queries,
            must_have: plan.must_have.clone(),
            answer_guidance: Some("Collect evidence for a cited deep research report.".to_string()),
        };
        let results = tavily_search(&tavily_key, &plan.goal, search_plan, 10).await?;
        wait_if_paused_or_cancelled(&control).await?;
        let remaining = budget
            .source_limit
            .unwrap_or(DEFAULT_SOURCE_LIMIT)
            .saturating_sub(all_sources.len() as u16) as usize;
        let results = results.into_iter().take(remaining).collect::<Vec<_>>();
        wait_if_paused_or_cancelled(&control).await?;
        let inserted = state.db.save_research_sources(&task_id, &results)?;
        if !inserted.is_empty() {
            let _ = app.emit("research:sources-delta", &inserted);
            all_sources.extend(inserted.clone());
        }
        save_activity(
            &app,
            &state,
            &task_id,
            "sources",
            &format!("新增 {} 个来源", inserted.len()),
            Some(format!("累计 {} 个来源", all_sources.len())),
        )?;

        if all_sources.len() as u16 >= budget.source_limit.unwrap_or(DEFAULT_SOURCE_LIMIT) {
            break;
        }
        if round > 0 && inserted.is_empty() {
            break;
        }
    }

    if all_sources.is_empty() {
        return Err(ResearchRunError::Failed(
            "没有找到可用于研究的来源。".to_string(),
        ));
    }

    wait_if_paused_or_cancelled(&control).await?;
    let task = state.db.get_research_task(&task_id)?;
    emit_progress(
        &app,
        &task.id,
        &task.conversation_id,
        "running",
        "report",
        budget.max_rounds.unwrap_or(DEFAULT_MAX_ROUNDS) as u16 + 1,
        budget.max_rounds.unwrap_or(DEFAULT_MAX_ROUNDS) as u16 + 2,
        "正在撰写研究报告",
    );
    save_activity(
        &app,
        &state,
        &task_id,
        "report",
        "开始撰写报告",
        Some("报告会保留来源编号，并标注不确定性。".to_string()),
    )?;
    let report = stream_research_report(
        &app,
        &deepseek_key,
        &settings.deepseek_base_url,
        &model,
        &task,
        &plan,
        &all_sources,
        &assistant_message_id,
        control.clone(),
    )
    .await?;
    state.db.update_research_report(&task_id, &report)?;
    if !report_has_valid_citation(&report, all_sources.len()) {
        save_activity(
            &app,
            &state,
            &task_id,
            "citation_warning",
            "引用校验未完全通过",
            Some("报告中没有检测到有效来源编号，建议人工复核。".to_string()),
        )?;
    }

    let completed_at = Utc::now().to_rfc3339();
    state
        .db
        .update_research_status(&task_id, "completed", None, Some(&completed_at))?;
    let assistant_message = ChatMessage {
        id: assistant_message_id.clone(),
        conversation_id: task.conversation_id.clone(),
        role: "assistant".to_string(),
        content: report,
        reasoning_content: None,
        tool_calls_json: None,
        tool_result_json: Some(
            serde_json::to_string(&all_sources).unwrap_or_else(|_| "[]".to_string()),
        ),
        created_at: completed_at,
    };
    state.db.save_message(&assistant_message)?;
    emit_progress(
        &app,
        &task.id,
        &task.conversation_id,
        "completed",
        "done",
        budget.max_rounds.unwrap_or(DEFAULT_MAX_ROUNDS) as u16 + 2,
        budget.max_rounds.unwrap_or(DEFAULT_MAX_ROUNDS) as u16 + 2,
        "研究完成",
    );
    let _ = app.emit(
        "research:done",
        ChatStartedEvent {
            conversation_id: task.conversation_id.clone(),
            message_id: assistant_message_id.clone(),
        },
    );
    let _ = app.emit(
        "chat:done",
        ChatStartedEvent {
            conversation_id: task.conversation_id,
            message_id: assistant_message_id,
        },
    );
    Ok(())
}

async fn create_research_plan(
    api_key: &str,
    base_url: &str,
    model: &str,
    prompt: &str,
    source_policy: &str,
    domains: &[String],
) -> Result<ResearchPlan, String> {
    let response = research_client()?
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
                    "content": research_planner_prompt()
                },
                {
                    "role": "user",
                    "content": format!(
                        "User research request:\n{}\n\nSource policy: {}\nDomains: {}",
                        prompt,
                        source_policy,
                        serde_json::to_string(domains).unwrap_or_else(|_| "[]".to_string())
                    )
                }
            ],
            "stream": false,
            "temperature": 0
        }))
        .send()
        .await
        .map_err(|_| "研究计划请求失败或超时。".to_string())?;

    if !response.status().is_success() {
        return Err(format!("研究计划返回错误：{}", response.status()));
    }
    let body = response
        .json::<Value>()
        .await
        .map_err(|_| "研究计划响应解析失败。".to_string())?;
    let content = body["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| "研究计划内容为空。".to_string())?;
    let json_text =
        extract_json_object(content).ok_or_else(|| "研究计划不是 JSON。".to_string())?;
    serde_json::from_str::<ResearchPlan>(&json_text).map_err(|_| "研究计划 JSON 无效。".to_string())
}

fn research_planner_prompt() -> &'static str {
    r#"You are a deep research planner. Return only valid JSON with this camelCase shape:
{
  "title": "short research title",
  "goal": "what the final report must answer",
  "audience": "who the report is for",
  "keyQuestions": ["major questions to answer"],
  "mustHave": ["facts or checks that must be verified"],
  "initialQueries": [
    {
      "query": "short web search query",
      "topic": "general or news",
      "searchDepth": "basic or advanced",
      "maxResults": 8,
      "includeDomains": [],
      "excludeDomains": [],
      "startDate": null,
      "endDate": null,
      "includeRawContent": true
    }
  ],
  "successCriteria": ["what a good final report includes"],
  "sourcePolicy": "web, includeDomains, or preferDomains",
  "domains": [],
  "depthBudget": {
    "maxRounds": 4,
    "queriesPerRound": 6,
    "sourceLimit": 60
  }
}

Rules:
- Plan a research project, not a one-shot search.
- Generate 4 to 8 short initial queries.
- Prefer primary, official, academic, regulatory, or high-quality sources when relevant.
- Do not invent restricted domains; only use domains explicitly supplied by the user.
- Use advanced search and raw content when evidence quality matters.
- Keep all dates as YYYY-MM-DD or null."#
}

async fn create_follow_up_queries(
    api_key: &str,
    base_url: &str,
    model: &str,
    plan: &ResearchPlan,
    sources: &[ResearchSource],
    limit: u8,
) -> Result<Vec<PlannedSearchQuery>, String> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct GapResponse {
        follow_up_queries: Vec<String>,
    }

    let response = research_client()?
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
                    "content": "You find evidence gaps for a deep research task. Return only JSON: {\"followUpQueries\":[\"short query\"]}. Return an empty list when coverage is sufficient."
                },
                {
                    "role": "user",
                    "content": format!(
                        "Plan:\n{}\n\nCurrent sources:\n{}",
                        serde_json::to_string(plan).unwrap_or_default(),
                        format_sources_for_prompt(sources, 30)
                    )
                }
            ],
            "stream": false,
            "temperature": 0
        }))
        .send()
        .await
        .map_err(|_| "研究缺口分析请求失败。".to_string())?;
    if !response.status().is_success() {
        return Err(format!("研究缺口分析返回错误：{}", response.status()));
    }
    let body = response
        .json::<Value>()
        .await
        .map_err(|_| "研究缺口分析响应解析失败。".to_string())?;
    let content = body["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| "研究缺口分析内容为空。".to_string())?;
    let json_text =
        extract_json_object(content).ok_or_else(|| "研究缺口分析不是 JSON。".to_string())?;
    let parsed: GapResponse =
        serde_json::from_str(&json_text).map_err(|_| "研究缺口分析 JSON 无效。".to_string())?;
    Ok(parsed
        .follow_up_queries
        .into_iter()
        .map(|query| planned_query(&query, Some(true)))
        .filter(|query| !query.query.is_empty())
        .take(limit as usize)
        .collect())
}

async fn stream_research_report(
    app: &AppHandle,
    api_key: &str,
    base_url: &str,
    model: &str,
    task: &ResearchTask,
    plan: &ResearchPlan,
    sources: &[ResearchSource],
    assistant_message_id: &str,
    control: ResearchRunControl,
) -> Result<String, ResearchRunError> {
    let response = research_client()?
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
                    "content": research_report_prompt()
                },
                {
                    "role": "user",
                    "content": format!(
                        "Research plan:\n{}\n\nEvidence sources:\n{}",
                        serde_json::to_string(plan).unwrap_or_default(),
                        format_sources_for_prompt(sources, sources.len())
                    )
                }
            ],
            "stream": true,
            "temperature": 0.2
        }))
        .send()
        .await
        .map_err(|_| "研究报告请求失败或超时。".to_string())?;
    if !response.status().is_success() {
        return Err(format!("研究报告返回错误：{}", response.status()).into());
    }

    let mut report = String::new();
    let mut buffer = String::new();
    let mut stream = response.bytes_stream();

    loop {
        wait_if_paused_or_cancelled(&control).await?;
        let Some(chunk) = stream.next().await else {
            break;
        };
        let chunk = chunk.map_err(|_| "研究报告流式输出中断。".to_string())?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));
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
            if let Some(piece) = value["choices"][0]["delta"]["content"].as_str() {
                if report.chars().count().saturating_add(piece.chars().count())
                    > MAX_ASSISTANT_OUTPUT_CHARS
                {
                    return Err(format!(
                        "研究报告过长，请控制在 {} 字符以内。",
                        MAX_ASSISTANT_OUTPUT_CHARS
                    )
                    .into());
                }
                report.push_str(piece);
                let _ = app.emit(
                    "research:report-delta",
                    ResearchReportDeltaEvent {
                        task_id: task.id.clone(),
                        conversation_id: task.conversation_id.clone(),
                        message_id: assistant_message_id.to_string(),
                        delta: piece.to_string(),
                    },
                );
                let _ = app.emit(
                    "chat:content-delta",
                    ContentDeltaEvent {
                        conversation_id: task.conversation_id.clone(),
                        message_id: assistant_message_id.to_string(),
                        delta: piece.to_string(),
                    },
                );
            }
        }
    }

    Ok(report)
}

fn research_report_prompt() -> &'static str {
    r#"Write a ChatGPT Deep Research style report in Markdown.

Required structure:
- Title
- Executive summary
- Key findings
- Detailed analysis
- Uncertainties and limits
- Sources used

Rules:
- Every factual claim that depends on evidence must cite source numbers like [S1] or [S3].
- Distinguish facts from inference.
- Do not claim that missing evidence is confirmed.
- Use source numbers exactly as provided.
- End with a compact source list using source number, title, and URL."#
}

fn research_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(RESEARCH_REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|_| "无法初始化研究网络客户端。".to_string())
}

fn validate_research_prompt(prompt: &str) -> Result<(), String> {
    if prompt.trim().is_empty() {
        return Err("研究主题不能为空。".to_string());
    }
    if prompt.chars().count() > MAX_USER_MESSAGE_CHARS {
        return Err(format!(
            "研究主题过长，请控制在 {} 字符以内。",
            MAX_USER_MESSAGE_CHARS
        ));
    }
    Ok(())
}

fn touch_conversation(
    state: &State<'_, AppState>,
    conversation_id: &str,
    prompt: &str,
    model: &str,
    now: &str,
) -> Result<(), String> {
    if let Some(mut conversation) = state
        .db
        .get_conversations()?
        .into_iter()
        .find(|item| item.id == conversation_id)
    {
        if conversation.title == "新会话" {
            conversation.title = prompt.chars().take(24).collect();
        }
        conversation.model = model.to_string();
        conversation.updated_at = now.to_string();
        state.db.upsert_conversation(&conversation)?;
    }
    Ok(())
}

fn save_activity(
    app: &AppHandle,
    state: &State<'_, AppState>,
    task_id: &str,
    activity_type: &str,
    title: &str,
    detail: Option<String>,
) -> Result<ResearchActivity, String> {
    let activity = ResearchActivity {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_string(),
        activity_type: activity_type.to_string(),
        title: title.to_string(),
        detail,
        created_at: Utc::now().to_rfc3339(),
    };
    state.db.save_research_activity(&activity)?;
    let _ = app.emit("research:activity", &activity);
    Ok(activity)
}

fn emit_progress(
    app: &AppHandle,
    task_id: &str,
    conversation_id: &str,
    status: &str,
    phase: &str,
    completed_steps: u16,
    total_steps: u16,
    message: &str,
) {
    let _ = app.emit(
        "research:progress",
        ResearchProgressEvent {
            task_id: task_id.to_string(),
            conversation_id: conversation_id.to_string(),
            status: status.to_string(),
            phase: phase.to_string(),
            completed_steps,
            total_steps,
            message: message.to_string(),
        },
    );
}

fn active_research_control(
    state: &State<'_, AppState>,
    task_id: &str,
) -> Result<ResearchRunControl, String> {
    state
        .research_runs
        .lock()
        .map_err(|_| "研究任务状态锁定失败".to_string())?
        .get(task_id)
        .cloned()
        .ok_or_else(|| "研究任务没有正在运行的后台进程，请重新开始。".to_string())
}

async fn wait_if_paused_or_cancelled(control: &ResearchRunControl) -> Result<(), ResearchRunError> {
    loop {
        if control.cancellation.is_cancelled() {
            return Err(ResearchRunError::Cancelled);
        }
        if !control.paused.load(Ordering::SeqCst) {
            return Ok(());
        }
        sleep(Duration::from_millis(400)).await;
    }
}

fn fallback_research_plan(prompt: &str, source_policy: &str, domains: &[String]) -> ResearchPlan {
    ResearchPlan {
        title: prompt.chars().take(64).collect(),
        goal: prompt.trim().to_string(),
        audience: Some("general".to_string()),
        key_questions: vec![prompt.trim().to_string()],
        must_have: vec![
            "current evidence".to_string(),
            "major viewpoints".to_string(),
        ],
        initial_queries: fallback_initial_queries(prompt),
        success_criteria: vec![
            "Answer the main research question with citations.".to_string(),
            "Call out uncertainty and missing evidence.".to_string(),
        ],
        source_policy: Some(source_policy.to_string()),
        domains: domains.to_vec(),
        depth_budget: Some(ResearchDepthBudget {
            max_rounds: Some(DEFAULT_MAX_ROUNDS),
            queries_per_round: Some(DEFAULT_QUERIES_PER_ROUND),
            source_limit: Some(DEFAULT_SOURCE_LIMIT),
        }),
    }
}

fn normalize_research_plan(
    mut plan: ResearchPlan,
    prompt: &str,
    source_policy: &str,
    domains: &[String],
) -> ResearchPlan {
    if plan.title.trim().is_empty() {
        plan.title = prompt.chars().take(64).collect();
    }
    if plan.goal.trim().is_empty() {
        plan.goal = prompt.trim().to_string();
    }
    plan.source_policy = Some(source_policy.to_string());
    plan.domains = domains.to_vec();
    plan.key_questions = sanitize_string_list(plan.key_questions, 12);
    if plan.key_questions.is_empty() {
        plan.key_questions.push(prompt.trim().to_string());
    }
    plan.must_have = sanitize_string_list(plan.must_have, 16);
    if plan.must_have.is_empty() {
        plan.must_have.push("relevant evidence".to_string());
    }
    plan.success_criteria = sanitize_string_list(plan.success_criteria, 12);
    plan.initial_queries = plan
        .initial_queries
        .into_iter()
        .map(normalize_planned_query)
        .filter(|query| !query.query.is_empty())
        .take(DEFAULT_QUERIES_PER_ROUND as usize + 2)
        .collect();
    if plan.initial_queries.is_empty() {
        plan.initial_queries = fallback_initial_queries(prompt);
    }
    plan.depth_budget = Some(normalized_budget(plan.depth_budget.as_ref()));
    plan
}

fn normalized_budget(budget: Option<&ResearchDepthBudget>) -> ResearchDepthBudget {
    ResearchDepthBudget {
        max_rounds: Some(
            budget
                .and_then(|item| item.max_rounds)
                .unwrap_or(DEFAULT_MAX_ROUNDS)
                .clamp(1, 6),
        ),
        queries_per_round: Some(
            budget
                .and_then(|item| item.queries_per_round)
                .unwrap_or(DEFAULT_QUERIES_PER_ROUND)
                .clamp(1, 8),
        ),
        source_limit: Some(
            budget
                .and_then(|item| item.source_limit)
                .unwrap_or(DEFAULT_SOURCE_LIMIT)
                .clamp(10, 100),
        ),
    }
}

fn fallback_initial_queries(prompt: &str) -> Vec<PlannedSearchQuery> {
    let short = short_query(prompt);
    [
        short.clone(),
        format!("{short} official source"),
        format!("{short} analysis report"),
        format!("{short} latest data"),
    ]
    .into_iter()
    .map(|query| planned_query(&query, Some(true)))
    .filter(|query| !query.query.is_empty())
    .collect()
}

fn fallback_follow_up_queries(
    plan: &ResearchPlan,
    sources: &[ResearchSource],
) -> Vec<PlannedSearchQuery> {
    let evidence = sources
        .iter()
        .map(|source| format!("{} {}", source.title, source.snippet))
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase();
    plan.must_have
        .iter()
        .filter(|item| !roughly_covered(item, &evidence))
        .map(|item| planned_query(&format!("{} {}", plan.goal, item), Some(true)))
        .take(DEFAULT_QUERIES_PER_ROUND as usize)
        .collect()
}

fn apply_source_policy(
    queries: Vec<PlannedSearchQuery>,
    source_policy: &str,
    domains: &[String],
    limit: u8,
) -> Vec<PlannedSearchQuery> {
    let mut output = Vec::new();
    for mut query in queries {
        query = normalize_planned_query(query);
        if query.query.is_empty() {
            continue;
        }
        match source_policy {
            "includeDomains" if !domains.is_empty() => {
                query.include_domains = domains.to_vec();
                output.push(query);
            }
            "preferDomains" if !domains.is_empty() => {
                let mut preferred = query.clone();
                preferred.include_domains = domains.to_vec();
                output.push(preferred);
                output.push(query);
            }
            _ => output.push(query),
        }
        if output.len() >= limit as usize {
            break;
        }
    }
    output
}

fn normalize_planned_query(mut query: PlannedSearchQuery) -> PlannedSearchQuery {
    query.query = short_query(&query.query);
    query.topic = match query.topic.as_deref() {
        Some("news") => Some("news".to_string()),
        _ => Some("general".to_string()),
    };
    query.search_depth = match query.search_depth.as_deref() {
        Some("advanced") => Some("advanced".to_string()),
        _ => Some("basic".to_string()),
    };
    query.max_results = Some(query.max_results.unwrap_or(8).clamp(1, 10));
    query.include_domains = sanitize_domains(query.include_domains);
    query.exclude_domains = sanitize_domains(query.exclude_domains);
    query.include_raw_content = Some(query.include_raw_content.unwrap_or(true));
    query
}

fn planned_query(query: &str, include_raw_content: Option<bool>) -> PlannedSearchQuery {
    PlannedSearchQuery {
        query: short_query(query),
        topic: Some("general".to_string()),
        search_depth: Some("advanced".to_string()),
        max_results: Some(8),
        include_domains: Vec::new(),
        exclude_domains: Vec::new(),
        start_date: None,
        end_date: None,
        include_raw_content,
    }
}

fn normalize_source_policy(policy: &str) -> String {
    match policy {
        "includeDomains" | "preferDomains" => policy.to_string(),
        _ => "web".to_string(),
    }
}

fn sanitize_domains(domains: Vec<String>) -> Vec<String> {
    domains
        .into_iter()
        .filter_map(|domain| {
            let domain = domain
                .trim()
                .trim_start_matches("https://")
                .trim_start_matches("http://")
                .trim_start_matches("www.")
                .split('/')
                .next()
                .unwrap_or_default()
                .to_lowercase();
            if domain.is_empty()
                || domain.contains(' ')
                || !domain
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.'))
            {
                None
            } else {
                Some(domain)
            }
        })
        .collect::<HashSet<_>>()
        .into_iter()
        .take(MAX_DOMAINS)
        .collect()
}

fn sanitize_string_list(items: Vec<String>, limit: usize) -> Vec<String> {
    items
        .into_iter()
        .map(|item| item.trim().chars().take(240).collect::<String>())
        .filter(|item| !item.is_empty())
        .take(limit)
        .collect()
}

fn parse_domains_json(value: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(value)
        .map(sanitize_domains)
        .unwrap_or_default()
}

fn short_query(text: &str) -> String {
    text.split_whitespace()
        .take(12)
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_QUERY_CHARS)
        .collect::<String>()
        .trim_matches(|c: char| c.is_ascii_punctuation())
        .trim()
        .to_string()
}

fn roughly_covered(item: &str, evidence: &str) -> bool {
    let tokens = item
        .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
        .map(|token| token.trim().to_lowercase())
        .filter(|token| token.chars().count() >= 3)
        .collect::<Vec<_>>();
    if tokens.is_empty() {
        return true;
    }
    let hits = tokens
        .iter()
        .filter(|token| evidence.contains(token.as_str()))
        .count();
    hits >= 1.max(tokens.len().saturating_div(2))
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

fn format_sources_for_prompt<S: SourceLike>(sources: &[S], limit: usize) -> String {
    sources
        .iter()
        .take(limit)
        .map(SourceLike::to_prompt_line)
        .collect::<Vec<_>>()
        .join("\n")
}

trait SourceLike {
    fn to_prompt_line(&self) -> String;
}

impl SourceLike for ResearchSource {
    fn to_prompt_line(&self) -> String {
        format!(
            "[S{}] {}\nURL: {}\nPublished: {}\nQuery: {}\nSnippet: {}\nRaw: {}",
            self.source_number,
            self.title,
            self.url,
            self.published_at.as_deref().unwrap_or("unknown"),
            self.source_query.as_deref().unwrap_or("unknown"),
            self.snippet,
            self.raw_content.as_deref().unwrap_or("")
        )
    }
}

impl SourceLike for SearchResult {
    fn to_prompt_line(&self) -> String {
        format!(
            "{}\nURL: {}\nSnippet: {}",
            self.title, self.url, self.snippet
        )
    }
}

fn report_has_valid_citation(report: &str, source_count: usize) -> bool {
    (1..=source_count).any(|index| report.contains(&format!("[S{index}]")))
}

fn format_export_markdown(detail: &ResearchTaskDetail) -> String {
    let plan = serde_json::from_str::<ResearchPlan>(&detail.task.plan_json).ok();
    let mut output = String::new();
    output.push_str(&format!("# {}\n\n", detail.task.topic));
    output.push_str(&format!("Status: `{}`\n\n", detail.task.status));
    if let Some(plan) = plan {
        output.push_str("## Research Plan\n\n");
        output.push_str(&format!("Goal: {}\n\n", plan.goal));
        if !plan.key_questions.is_empty() {
            output.push_str("Key questions:\n");
            for item in plan.key_questions {
                output.push_str(&format!("- {item}\n"));
            }
            output.push('\n');
        }
    }
    output.push_str("## Report\n\n");
    output.push_str(if detail.task.report.trim().is_empty() {
        "_No report generated yet._\n"
    } else {
        &detail.task.report
    });
    output.push_str("\n\n## Activity\n\n");
    for activity in &detail.activities {
        output.push_str(&format!(
            "- {}: {}{}\n",
            activity.created_at,
            activity.title,
            activity
                .detail
                .as_ref()
                .map(|detail| format!(" - {detail}"))
                .unwrap_or_default()
        ));
    }
    output.push_str("\n## Sources\n\n");
    for source in &detail.sources {
        output.push_str(&format!(
            "- [S{}] [{}]({})\n",
            source.source_number, source.title, source.url
        ));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_domains_removes_unsafe_entries() {
        let domains = sanitize_domains(vec![
            "https://www.example.com/path".to_string(),
            "bad domain".to_string(),
            "file:///tmp/key".to_string(),
            "EXAMPLE.com".to_string(),
        ]);

        assert!(domains.contains(&"example.com".to_string()));
        assert!(!domains.contains(&"bad domain".to_string()));
        assert!(!domains.contains(&"file:".to_string()));
    }

    #[test]
    fn normalize_plan_fills_defaults() {
        let plan = normalize_research_plan(
            ResearchPlan {
                title: String::new(),
                goal: String::new(),
                audience: None,
                key_questions: Vec::new(),
                must_have: Vec::new(),
                initial_queries: Vec::new(),
                success_criteria: Vec::new(),
                source_policy: None,
                domains: Vec::new(),
                depth_budget: None,
            },
            "research rust async runtime",
            "web",
            &[],
        );

        assert_eq!(plan.source_policy.as_deref(), Some("web"));
        assert!(!plan.initial_queries.is_empty());
        assert_eq!(
            plan.depth_budget.unwrap().max_rounds,
            Some(DEFAULT_MAX_ROUNDS)
        );
    }

    #[test]
    fn apply_prefer_domains_duplicates_restricted_and_open_queries() {
        let queries = apply_source_policy(
            vec![planned_query("rust release", Some(true))],
            "preferDomains",
            &["rust-lang.org".to_string()],
            4,
        );

        assert_eq!(queries.len(), 2);
        assert_eq!(queries[0].include_domains, vec!["rust-lang.org"]);
        assert!(queries[1].include_domains.is_empty());
    }

    #[test]
    fn detects_valid_report_citation() {
        assert!(report_has_valid_citation("A claim [S2].", 3));
        assert!(!report_has_valid_citation("A claim without citation.", 3));
    }

    #[tokio::test]
    async fn cancelled_control_returns_typed_cancel_error() {
        let control = ResearchRunControl::new();
        control.cancel();

        match wait_if_paused_or_cancelled(&control).await {
            Err(ResearchRunError::Cancelled) => {}
            other => panic!("expected typed cancellation, got {other:?}"),
        }
    }
}
