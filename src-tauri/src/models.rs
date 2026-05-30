use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub model: String,
    pub thinking_mode: String,
    pub search_enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub reasoning_content: Option<String>,
    pub tool_calls_json: Option<String>,
    pub tool_result_json: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub deepseek_base_url: String,
    pub default_model: String,
    pub default_thinking_mode: String,
    pub default_search_enabled: bool,
    pub show_reasoning_content: bool,
    pub search_provider: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            deepseek_base_url: "https://api.deepseek.com".to_string(),
            default_model: "deepseek-v4-pro".to_string(),
            default_thinking_mode: "off".to_string(),
            default_search_enabled: false,
            show_reasoning_content: false,
            search_provider: "tavily".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageRequest {
    pub conversation_id: String,
    pub content: String,
    pub model: String,
    pub thinking_mode: String,
    pub search_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareResearchTaskRequest {
    pub conversation_id: String,
    pub prompt: String,
    pub model: String,
    pub source_policy: String,
    #[serde(default)]
    pub domains: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartResearchTaskRequest {
    pub task_id: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchTask {
    pub id: String,
    pub conversation_id: String,
    pub user_message_id: String,
    pub assistant_message_id: Option<String>,
    pub topic: String,
    pub status: String,
    pub source_policy: String,
    pub domains_json: String,
    pub plan_json: String,
    pub report: String,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchSource {
    pub id: String,
    pub task_id: String,
    pub source_number: i64,
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub published_at: Option<String>,
    pub source_domain: Option<String>,
    pub raw_content: Option<String>,
    pub score: Option<f64>,
    pub source_query: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchActivity {
    pub id: String,
    pub task_id: String,
    pub activity_type: String,
    pub title: String,
    pub detail: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchTaskDetail {
    pub task: ResearchTask,
    pub sources: Vec<ResearchSource>,
    pub activities: Vec<ResearchActivity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareResearchTaskResponse {
    pub detail: ResearchTaskDetail,
    pub user_message: ChatMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchPlan {
    pub title: String,
    pub goal: String,
    pub audience: Option<String>,
    #[serde(default)]
    pub key_questions: Vec<String>,
    #[serde(default)]
    pub must_have: Vec<String>,
    #[serde(default)]
    pub initial_queries: Vec<PlannedSearchQuery>,
    #[serde(default)]
    pub success_criteria: Vec<String>,
    pub source_policy: Option<String>,
    #[serde(default)]
    pub domains: Vec<String>,
    pub depth_budget: Option<ResearchDepthBudget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchDepthBudget {
    pub max_rounds: Option<u8>,
    pub queries_per_round: Option<u8>,
    pub source_limit: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchProgressEvent {
    pub task_id: String,
    pub conversation_id: String,
    pub status: String,
    pub phase: String,
    pub completed_steps: u16,
    pub total_steps: u16,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchReportDeltaEvent {
    pub task_id: String,
    pub conversation_id: String,
    pub message_id: String,
    pub delta: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentDeltaEvent {
    pub conversation_id: String,
    pub message_id: String,
    pub delta: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatStartedEvent {
    pub conversation_id: String,
    pub message_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatErrorEvent {
    pub conversation_id: String,
    pub message_id: Option<String>,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub published_at: Option<String>,
    pub source: Option<String>,
    pub source_domain: Option<String>,
    pub raw_content: Option<String>,
    pub score: Option<f64>,
    pub source_query: Option<String>,
    pub source_category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchPlan {
    pub intent: Option<String>,
    #[serde(default)]
    pub queries: Vec<PlannedSearchQuery>,
    #[serde(default)]
    pub must_have: Vec<String>,
    pub answer_guidance: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedSearchQuery {
    pub query: String,
    pub topic: Option<String>,
    #[serde(alias = "searchDepth")]
    pub search_depth: Option<String>,
    #[serde(alias = "maxResults")]
    pub max_results: Option<u8>,
    #[serde(default, alias = "includeDomains")]
    pub include_domains: Vec<String>,
    #[serde(default, alias = "excludeDomains")]
    pub exclude_domains: Vec<String>,
    #[serde(alias = "startDate")]
    pub start_date: Option<String>,
    #[serde(alias = "endDate")]
    pub end_date: Option<String>,
    #[serde(alias = "includeRawContent")]
    pub include_raw_content: Option<bool>,
}
