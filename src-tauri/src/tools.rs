use crate::models::{SearchPlan, SearchResult};
use crate::search::{fallback_search_plan, tavily_search};
use crate::secrets::get_secret;
use serde_json::Value;

pub async fn run_tool_call(name: &str, arguments: &str) -> Result<Vec<SearchResult>, String> {
    if name != "web_search" {
        return Err("不支持的工具调用。".to_string());
    }
    let value: Value =
        serde_json::from_str(arguments).map_err(|_| "Tool Call JSON 解析失败。".to_string())?;
    let query = value
        .get("query")
        .and_then(Value::as_str)
        .ok_or_else(|| "Tool Call 参数缺失。".to_string())?;
    let max_results = value
        .get("max_results")
        .and_then(Value::as_u64)
        .unwrap_or(5)
        .clamp(1, 10) as u8;
    let plan = value
        .get("plan")
        .cloned()
        .and_then(|plan| serde_json::from_value::<SearchPlan>(plan).ok())
        .unwrap_or_else(|| fallback_search_plan(query, max_results));
    let api_key = get_secret("tavily_api_key")?;
    tavily_search(&api_key, query, plan, max_results).await
}
