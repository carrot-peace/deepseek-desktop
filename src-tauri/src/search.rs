use crate::models::{PlannedSearchQuery, SearchPlan, SearchResult};
use futures_util::future::join_all;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::collections::{HashMap, HashSet};

const MAX_QUERIES_PER_ROUND: usize = 8;
const MAX_SEARCH_ROUNDS: usize = 2;
const MAX_QUERY_WORDS: usize = 12;
const MAX_QUERY_CHARS: usize = 120;

#[derive(Debug, Deserialize)]
struct TavilyResponse {
    results: Vec<TavilyResult>,
}

#[derive(Debug, Deserialize)]
struct TavilyResult {
    title: Option<String>,
    url: Option<String>,
    content: Option<String>,
    #[serde(default)]
    raw_content: Option<String>,
    #[serde(default)]
    published_date: Option<String>,
    #[serde(default)]
    score: Option<f64>,
}

pub struct SearchOrchestrator {
    client: Client,
}

impl SearchOrchestrator {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn search(
        &self,
        api_key: &str,
        question: &str,
        plan: SearchPlan,
        max_results: u8,
    ) -> Result<Vec<SearchResult>, String> {
        if question.trim().is_empty() {
            return Err("搜索关键词不能为空".to_string());
        }

        let mut all_results = Vec::new();
        let mut seen_queries = HashSet::new();
        let mut round_queries = sanitize_queries(plan.queries, question, max_results);

        if round_queries.is_empty() {
            round_queries = fallback_queries(question, max_results);
        }

        for round in 0..MAX_SEARCH_ROUNDS {
            let before_count = all_results.len();
            let fresh_queries = round_queries
                .into_iter()
                .filter(|query| seen_queries.insert(query_signature(query)))
                .take(MAX_QUERIES_PER_ROUND)
                .collect::<Vec<_>>();

            if fresh_queries.is_empty() {
                break;
            }

            let mut round_results = self.run_queries(api_key, &fresh_queries).await?;
            all_results.append(&mut round_results);
            all_results = dedupe_results(all_results);

            if round + 1 >= MAX_SEARCH_ROUNDS {
                break;
            }
            if all_results.len() == before_count {
                break;
            }

            let missing = missing_must_have(&plan.must_have, &all_results);
            if missing.is_empty() {
                break;
            }

            round_queries = supplemental_queries(question, &missing, max_results);
        }

        if all_results.is_empty() {
            return Err("未找到搜索结果。".to_string());
        }

        all_results.sort_by(|a, b| {
            let score_order = b
                .score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal);
            score_order.then_with(|| b.published_at.cmp(&a.published_at))
        });

        Ok(all_results
            .into_iter()
            .take((max_results as usize).saturating_mul(4).max(10))
            .collect())
    }

    async fn run_queries(
        &self,
        api_key: &str,
        queries: &[PlannedSearchQuery],
    ) -> Result<Vec<SearchResult>, String> {
        let futures = queries
            .iter()
            .map(|query| self.tavily_search_with_query(api_key, query));
        let responses = join_all(futures).await;
        let mut results = Vec::new();
        let mut last_error = None;

        for response in responses {
            match response {
                Ok(mut items) => results.append(&mut items),
                Err(error) => last_error = Some(error),
            }
        }

        if results.is_empty() {
            return Err(last_error.unwrap_or_else(|| "未找到搜索结果。".to_string()));
        }

        Ok(dedupe_results(results))
    }

    async fn tavily_search_with_query(
        &self,
        api_key: &str,
        query: &PlannedSearchQuery,
    ) -> Result<Vec<SearchResult>, String> {
        let mut payload = json!({
            "query": query.query,
            "topic": query.topic.as_deref().unwrap_or("general"),
            "search_depth": query.search_depth.as_deref().unwrap_or("basic"),
            "max_results": query.max_results.unwrap_or(5).clamp(1, 10),
            "include_answer": false,
        });

        if query.include_raw_content.unwrap_or(false) {
            payload["include_raw_content"] = json!(true);
        }
        if !query.include_domains.is_empty() {
            payload["include_domains"] = json!(query.include_domains);
        }
        if !query.exclude_domains.is_empty() {
            payload["exclude_domains"] = json!(query.exclude_domains);
        }
        if let Some(start_date) = &query.start_date {
            payload["start_date"] = json!(start_date);
        }
        if let Some(end_date) = &query.end_date {
            payload["end_date"] = json!(end_date);
        }

        let response = self
            .client
            .post("https://api.tavily.com/search")
            .bearer_auth(api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|_| "网络请求失败，请检查网络连接。".to_string())?;

        if !response.status().is_success() {
            let status = response.status();
            let detail = response.text().await.unwrap_or_default();
            return Err(format!(
                "Tavily API 返回错误：{} {}",
                status,
                trim_error_detail(&detail)
            ));
        }

        let body = response
            .json::<TavilyResponse>()
            .await
            .map_err(|_| "搜索结果解析失败。".to_string())?;

        Ok(body
            .results
            .into_iter()
            .filter_map(|item| {
                let url = item.url?;
                let domain = source_domain(&url);
                let category = domain.as_deref().map(source_category);
                Some(SearchResult {
                    title: item.title.unwrap_or_else(|| url.clone()),
                    url,
                    snippet: item.content.unwrap_or_default(),
                    published_at: item.published_date,
                    source: Some("Tavily".to_string()),
                    source_domain: domain,
                    raw_content: item.raw_content,
                    score: item.score,
                    source_query: Some(query.query.clone()),
                    source_category: category,
                })
            })
            .collect())
    }
}

pub async fn tavily_search(
    api_key: &str,
    query: &str,
    plan: SearchPlan,
    max_results: u8,
) -> Result<Vec<SearchResult>, String> {
    SearchOrchestrator::new()
        .search(api_key, query, plan, max_results)
        .await
}

pub fn fallback_search_plan(question: &str, max_results: u8) -> SearchPlan {
    SearchPlan {
        intent: Some("Fallback search plan".to_string()),
        queries: fallback_queries(question, max_results),
        must_have: vec![short_query(question)],
        answer_guidance: Some(
            "Answer from the available search evidence and clearly note uncertainty.".to_string(),
        ),
    }
}

fn sanitize_queries(
    queries: Vec<PlannedSearchQuery>,
    question: &str,
    default_max_results: u8,
) -> Vec<PlannedSearchQuery> {
    let sanitized = queries
        .into_iter()
        .filter_map(|query| sanitize_query(query, default_max_results))
        .take(MAX_QUERIES_PER_ROUND)
        .collect::<Vec<_>>();

    if sanitized.is_empty() {
        fallback_queries(question, default_max_results)
    } else {
        sanitized
    }
}

fn sanitize_query(
    query: PlannedSearchQuery,
    default_max_results: u8,
) -> Option<PlannedSearchQuery> {
    let text = short_query(&query.query);
    if text.is_empty() {
        return None;
    }

    Some(PlannedSearchQuery {
        query: text,
        topic: sanitize_topic(query.topic),
        search_depth: sanitize_search_depth(query.search_depth),
        max_results: Some(
            query
                .max_results
                .unwrap_or(default_max_results)
                .clamp(1, 10),
        ),
        include_domains: sanitize_domains(query.include_domains),
        exclude_domains: sanitize_domains(query.exclude_domains),
        start_date: sanitize_date(query.start_date),
        end_date: sanitize_date(query.end_date),
        include_raw_content: query.include_raw_content,
    })
}

fn fallback_queries(question: &str, max_results: u8) -> Vec<PlannedSearchQuery> {
    let primary = short_query(question);
    let secondary = question
        .split(['?', '？', '.', '。', ',', '，', ';', '；'])
        .map(short_query)
        .filter(|item| !item.is_empty() && item != &primary)
        .take(2)
        .collect::<Vec<_>>();

    std::iter::once(primary)
        .chain(secondary)
        .filter(|query| !query.is_empty())
        .map(|query| PlannedSearchQuery {
            query,
            topic: Some("general".to_string()),
            search_depth: Some("basic".to_string()),
            max_results: Some(max_results.clamp(1, 10)),
            include_domains: Vec::new(),
            exclude_domains: Vec::new(),
            start_date: None,
            end_date: None,
            include_raw_content: Some(false),
        })
        .take(3)
        .collect()
}

fn supplemental_queries(
    question: &str,
    missing: &[String],
    default_max_results: u8,
) -> Vec<PlannedSearchQuery> {
    let base = short_query(question);
    missing
        .iter()
        .map(|item| short_query(&format!("{base} {item}")))
        .filter(|query| !query.is_empty())
        .map(|query| PlannedSearchQuery {
            query,
            topic: Some("general".to_string()),
            search_depth: Some("basic".to_string()),
            max_results: Some(default_max_results.clamp(1, 10)),
            include_domains: Vec::new(),
            exclude_domains: Vec::new(),
            start_date: None,
            end_date: None,
            include_raw_content: Some(false),
        })
        .take(MAX_QUERIES_PER_ROUND)
        .collect()
}

fn missing_must_have(must_have: &[String], results: &[SearchResult]) -> Vec<String> {
    if must_have.is_empty() {
        return Vec::new();
    }

    let combined = results
        .iter()
        .map(|result| {
            format!(
                "{} {} {} {}",
                result.title,
                result.snippet,
                result.raw_content.as_deref().unwrap_or_default(),
                result.source_domain.as_deref().unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase();

    must_have
        .iter()
        .filter(|item| !roughly_covered(item, &combined))
        .take(MAX_QUERIES_PER_ROUND)
        .cloned()
        .collect()
}

fn roughly_covered(item: &str, combined: &str) -> bool {
    let tokens = item
        .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
        .map(|token| token.trim().to_lowercase())
        .filter(|token| token.chars().count() >= 2)
        .collect::<Vec<_>>();

    if tokens.is_empty() {
        return true;
    }

    let hits = tokens
        .iter()
        .filter(|token| combined.contains(token.as_str()))
        .count();
    hits >= 1.max(tokens.len().saturating_div(2))
}

fn short_query(text: &str) -> String {
    text.split_whitespace()
        .take(MAX_QUERY_WORDS)
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_QUERY_CHARS)
        .collect::<String>()
        .trim_matches(|c: char| c.is_ascii_punctuation())
        .trim()
        .to_string()
}

fn sanitize_topic(topic: Option<String>) -> Option<String> {
    match topic.as_deref() {
        Some("general") | Some("news") => topic,
        _ => Some("general".to_string()),
    }
}

fn sanitize_search_depth(depth: Option<String>) -> Option<String> {
    match depth.as_deref() {
        Some("basic") | Some("advanced") => depth,
        _ => Some("basic".to_string()),
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
        .take(8)
        .collect()
}

fn sanitize_date(date: Option<String>) -> Option<String> {
    let date = date?;
    let date = date.trim();
    if chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_ok() {
        Some(date.to_string())
    } else {
        None
    }
}

fn query_signature(query: &PlannedSearchQuery) -> String {
    format!(
        "{}::{:?}::{:?}",
        query.query.to_lowercase(),
        query.include_domains,
        query.exclude_domains
    )
}

fn dedupe_results(results: Vec<SearchResult>) -> Vec<SearchResult> {
    let mut by_url: HashMap<String, SearchResult> = HashMap::new();
    for result in results {
        let key = normalize_url(&result.url);
        by_url
            .entry(key)
            .and_modify(|existing| {
                if result.score.unwrap_or(0.0) > existing.score.unwrap_or(0.0) {
                    *existing = result.clone();
                } else if existing.raw_content.is_none() && result.raw_content.is_some() {
                    existing.raw_content = result.raw_content.clone();
                }
            })
            .or_insert(result);
    }
    by_url.into_values().collect()
}

fn normalize_url(url: &str) -> String {
    url.split('#')
        .next()
        .unwrap_or(url)
        .trim_end_matches('/')
        .to_string()
}

fn source_domain(url: &str) -> Option<String> {
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let host = without_scheme.split('/').next()?.trim().to_lowercase();
    if host.is_empty() {
        None
    } else {
        Some(host.strip_prefix("www.").unwrap_or(&host).to_string())
    }
}

fn source_category(domain: &str) -> String {
    if domain.ends_with(".gov") || domain.ends_with(".gov.cn") || domain.ends_with(".gov.hk") {
        "official".to_string()
    } else if matches!(domain, "reuters.com" | "apnews.com" | "bloomberg.com") {
        "wire_or_agency".to_string()
    } else {
        "web".to_string()
    }
}

fn trim_error_detail(detail: &str) -> String {
    let trimmed = detail.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    trimmed.chars().take(240).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_plan_uses_short_queries() {
        let plan = fallback_search_plan(
            "请搜索一个非常长的问题，里面包含很多上下文，但是搜索 query 不应该原样全部传给 Tavily",
            5,
        );

        assert!(!plan.queries.is_empty());
        assert!(plan.queries.len() <= 3);
        assert!(plan.queries[0].query.chars().count() <= MAX_QUERY_CHARS);
    }

    #[test]
    fn sanitize_query_keeps_only_supported_tavily_options() {
        let query = PlannedSearchQuery {
            query: "latest rust release official site".to_string(),
            topic: Some("unsupported".to_string()),
            search_depth: Some("deep".to_string()),
            max_results: Some(30),
            include_domains: vec![
                "https://www.rust-lang.org/learn".to_string(),
                "bad domain".to_string(),
            ],
            exclude_domains: vec!["example.com/path".to_string()],
            start_date: Some("2026-05-08".to_string()),
            end_date: Some("not-a-date".to_string()),
            include_raw_content: Some(true),
        };

        let sanitized = sanitize_query(query, 5).expect("query should survive sanitization");

        assert_eq!(sanitized.topic.as_deref(), Some("general"));
        assert_eq!(sanitized.search_depth.as_deref(), Some("basic"));
        assert_eq!(sanitized.max_results, Some(10));
        assert_eq!(sanitized.include_domains, vec!["rust-lang.org"]);
        assert_eq!(sanitized.exclude_domains, vec!["example.com"]);
        assert_eq!(sanitized.start_date.as_deref(), Some("2026-05-08"));
        assert_eq!(sanitized.end_date, None);
    }

    #[test]
    fn missing_must_have_drives_second_round_candidates() {
        let results = vec![SearchResult {
            title: "Rust 1.90 released".to_string(),
            url: "https://blog.rust-lang.org/release".to_string(),
            snippet: "Rust 1.90 release notes and compiler changes".to_string(),
            published_at: Some("2026-01-01".to_string()),
            source: Some("Tavily".to_string()),
            source_domain: Some("blog.rust-lang.org".to_string()),
            raw_content: None,
            score: Some(0.9),
            source_query: Some("rust release".to_string()),
            source_category: Some("web".to_string()),
        }];

        let missing = missing_must_have(
            &[
                "release notes".to_string(),
                "installation instructions".to_string(),
            ],
            &results,
        );

        assert_eq!(missing, vec!["installation instructions"]);
    }
}
