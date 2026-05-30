use crate::models::{
    AppSettings, ChatMessage, Conversation, ResearchActivity, ResearchSource, ResearchTask,
    ResearchTaskDetail, SearchResult,
};
use rusqlite::{params, Connection};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct Database {
    connection: Mutex<Connection>,
}

impl Database {
    pub fn new(path: PathBuf) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|error| error.to_string())?;
        connection
            .execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(|error| error.to_string())?;
        connection
            .execute_batch(include_str!("../migrations/001_init.sql"))
            .map_err(|error| error.to_string())?;
        connection
            .execute_batch(include_str!("../migrations/002_research.sql"))
            .map_err(|error| error.to_string())?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn get_conversations(&self) -> Result<Vec<Conversation>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库锁定失败".to_string())?;
        let mut statement = connection
            .prepare(
                "SELECT id, title, model, thinking_mode, search_enabled, created_at, updated_at
                 FROM conversations ORDER BY updated_at DESC",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok(Conversation {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    model: row.get(2)?,
                    thinking_mode: row.get(3)?,
                    search_enabled: row.get::<_, i64>(4)? == 1,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
    }

    pub fn upsert_conversation(&self, conversation: &Conversation) -> Result<(), String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库锁定失败".to_string())?;
        connection
            .execute(
                "INSERT INTO conversations (id, title, model, thinking_mode, search_enabled, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(id) DO UPDATE SET
                   title = excluded.title,
                   model = excluded.model,
                   thinking_mode = excluded.thinking_mode,
                   search_enabled = excluded.search_enabled,
                   updated_at = excluded.updated_at",
                params![
                    conversation.id,
                    conversation.title,
                    conversation.model,
                    conversation.thinking_mode,
                    if conversation.search_enabled { 1 } else { 0 },
                    conversation.created_at,
                    conversation.updated_at
                ],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn delete_conversation(&self, id: &str) -> Result<(), String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库锁定失败".to_string())?;
        connection
            .execute("DELETE FROM conversations WHERE id = ?1", params![id])
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn get_messages(&self, conversation_id: &str) -> Result<Vec<ChatMessage>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库锁定失败".to_string())?;
        let mut statement = connection
            .prepare(
                "SELECT id, conversation_id, role, content, reasoning_content, tool_calls_json, tool_result_json, created_at
                 FROM messages WHERE conversation_id = ?1 ORDER BY created_at ASC",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map(params![conversation_id], |row| {
                Ok(ChatMessage {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    role: row.get(2)?,
                    content: row.get(3)?,
                    reasoning_content: row.get(4)?,
                    tool_calls_json: row.get(5)?,
                    tool_result_json: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
    }

    pub fn save_message(&self, message: &ChatMessage) -> Result<(), String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库锁定失败".to_string())?;
        connection
            .execute(
                "INSERT INTO messages (id, conversation_id, role, content, reasoning_content, tool_calls_json, tool_result_json, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(id) DO UPDATE SET
                   content = excluded.content,
                   reasoning_content = excluded.reasoning_content,
                   tool_calls_json = excluded.tool_calls_json,
                   tool_result_json = excluded.tool_result_json",
                params![
                    message.id,
                    message.conversation_id,
                    message.role,
                    message.content,
                    message.reasoning_content,
                    message.tool_calls_json,
                    message.tool_result_json,
                    message.created_at
                ],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn get_settings(&self) -> Result<AppSettings, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库锁定失败".to_string())?;
        let mut settings = AppSettings::default();
        let mut statement = connection
            .prepare("SELECT key, value FROM settings")
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|error| error.to_string())?;
        for row in rows {
            let (key, value) = row.map_err(|error| error.to_string())?;
            match key.as_str() {
                "deepseek_base_url" => settings.deepseek_base_url = value,
                "default_model" => settings.default_model = value,
                "default_thinking_mode" => settings.default_thinking_mode = value,
                "default_search_enabled" => settings.default_search_enabled = value == "true",
                "show_reasoning_content" => settings.show_reasoning_content = value == "true",
                "search_provider" => settings.search_provider = value,
                _ => {}
            }
        }
        Ok(settings)
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<(), String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库锁定失败".to_string())?;
        let pairs = [
            ("deepseek_base_url", settings.deepseek_base_url.as_str()),
            ("default_model", settings.default_model.as_str()),
            (
                "default_thinking_mode",
                settings.default_thinking_mode.as_str(),
            ),
            (
                "default_search_enabled",
                if settings.default_search_enabled {
                    "true"
                } else {
                    "false"
                },
            ),
            (
                "show_reasoning_content",
                if settings.show_reasoning_content {
                    "true"
                } else {
                    "false"
                },
            ),
            ("search_provider", settings.search_provider.as_str()),
        ];
        for (key, value) in pairs {
            connection
                .execute(
                    "INSERT INTO settings (key, value) VALUES (?1, ?2)
                     ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                    params![key, value],
                )
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    pub fn save_research_task(&self, task: &ResearchTask) -> Result<(), String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库锁定失败".to_string())?;
        connection
            .execute(
                "INSERT INTO research_tasks (
                   id, conversation_id, user_message_id, assistant_message_id, topic, status,
                   source_policy, domains_json, plan_json, report, error, created_at, updated_at,
                   completed_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                 ON CONFLICT(id) DO UPDATE SET
                   assistant_message_id = excluded.assistant_message_id,
                   topic = excluded.topic,
                   status = excluded.status,
                   source_policy = excluded.source_policy,
                   domains_json = excluded.domains_json,
                   plan_json = excluded.plan_json,
                   report = excluded.report,
                   error = excluded.error,
                   updated_at = excluded.updated_at,
                   completed_at = excluded.completed_at",
                params![
                    task.id,
                    task.conversation_id,
                    task.user_message_id,
                    task.assistant_message_id,
                    task.topic,
                    task.status,
                    task.source_policy,
                    task.domains_json,
                    task.plan_json,
                    task.report,
                    task.error,
                    task.created_at,
                    task.updated_at,
                    task.completed_at
                ],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn get_research_task(&self, task_id: &str) -> Result<ResearchTask, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库锁定失败".to_string())?;
        connection
            .query_row(
                "SELECT id, conversation_id, user_message_id, assistant_message_id, topic, status,
                        source_policy, domains_json, plan_json, report, error, created_at,
                        updated_at, completed_at
                 FROM research_tasks WHERE id = ?1",
                params![task_id],
                research_task_from_row,
            )
            .map_err(|error| error.to_string())
    }

    pub fn get_research_tasks(&self, conversation_id: &str) -> Result<Vec<ResearchTask>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库锁定失败".to_string())?;
        let mut statement = connection
            .prepare(
                "SELECT id, conversation_id, user_message_id, assistant_message_id, topic, status,
                        source_policy, domains_json, plan_json, report, error, created_at,
                        updated_at, completed_at
                 FROM research_tasks WHERE conversation_id = ?1 ORDER BY updated_at DESC",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map(params![conversation_id], research_task_from_row)
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
    }

    pub fn get_research_task_detail(&self, task_id: &str) -> Result<ResearchTaskDetail, String> {
        Ok(ResearchTaskDetail {
            task: self.get_research_task(task_id)?,
            sources: self.get_research_sources(task_id)?,
            activities: self.get_research_activities(task_id)?,
        })
    }

    pub fn get_research_task_details(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<ResearchTaskDetail>, String> {
        self.get_research_tasks(conversation_id)?
            .into_iter()
            .map(|task| {
                Ok(ResearchTaskDetail {
                    sources: self.get_research_sources(&task.id)?,
                    activities: self.get_research_activities(&task.id)?,
                    task,
                })
            })
            .collect()
    }

    pub fn update_research_status(
        &self,
        task_id: &str,
        status: &str,
        error: Option<&str>,
        completed_at: Option<&str>,
    ) -> Result<(), String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库锁定失败".to_string())?;
        connection
            .execute(
                "UPDATE research_tasks
                 SET status = ?2, error = ?3, updated_at = ?4, completed_at = ?5
                 WHERE id = ?1",
                params![
                    task_id,
                    status,
                    error,
                    chrono::Utc::now().to_rfc3339(),
                    completed_at
                ],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn set_research_assistant_message(
        &self,
        task_id: &str,
        message_id: &str,
    ) -> Result<(), String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库锁定失败".to_string())?;
        connection
            .execute(
                "UPDATE research_tasks
                 SET assistant_message_id = ?2, updated_at = ?3
                 WHERE id = ?1",
                params![task_id, message_id, chrono::Utc::now().to_rfc3339()],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn update_research_report(&self, task_id: &str, report: &str) -> Result<(), String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库锁定失败".to_string())?;
        connection
            .execute(
                "UPDATE research_tasks
                 SET report = ?2, updated_at = ?3
                 WHERE id = ?1",
                params![task_id, report, chrono::Utc::now().to_rfc3339()],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn save_research_activity(&self, activity: &ResearchActivity) -> Result<(), String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库锁定失败".to_string())?;
        connection
            .execute(
                "INSERT INTO research_activities (id, task_id, activity_type, title, detail, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    activity.id,
                    activity.task_id,
                    activity.activity_type,
                    activity.title,
                    activity.detail,
                    activity.created_at
                ],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn save_research_sources(
        &self,
        task_id: &str,
        results: &[SearchResult],
    ) -> Result<Vec<ResearchSource>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库锁定失败".to_string())?;
        let mut existing_statement = connection
            .prepare("SELECT url FROM research_sources WHERE task_id = ?1")
            .map_err(|error| error.to_string())?;
        let existing_rows = existing_statement
            .query_map(params![task_id], |row| row.get::<_, String>(0))
            .map_err(|error| error.to_string())?;
        let mut existing = existing_rows
            .collect::<Result<HashSet<_>, _>>()
            .map_err(|error| error.to_string())?;
        let mut next_number = connection
            .query_row(
                "SELECT COALESCE(MAX(source_number), 0) + 1 FROM research_sources WHERE task_id = ?1",
                params![task_id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|error| error.to_string())?;
        let mut inserted = Vec::new();

        for result in results {
            if result.url.trim().is_empty() || !existing.insert(result.url.clone()) {
                continue;
            }
            let source = ResearchSource {
                id: uuid::Uuid::new_v4().to_string(),
                task_id: task_id.to_string(),
                source_number: next_number,
                title: if result.title.trim().is_empty() {
                    result.url.clone()
                } else {
                    result.title.clone()
                },
                url: result.url.clone(),
                snippet: result.snippet.clone(),
                published_at: result.published_at.clone(),
                source_domain: result.source_domain.clone(),
                raw_content: result.raw_content.clone(),
                score: result.score,
                source_query: result.source_query.clone(),
                created_at: chrono::Utc::now().to_rfc3339(),
            };
            connection
                .execute(
                    "INSERT OR IGNORE INTO research_sources (
                       id, task_id, source_number, title, url, snippet, published_at,
                       source_domain, raw_content, score, source_query, created_at
                     )
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    params![
                        source.id,
                        source.task_id,
                        source.source_number,
                        source.title,
                        source.url,
                        source.snippet,
                        source.published_at,
                        source.source_domain,
                        source.raw_content,
                        source.score,
                        source.source_query,
                        source.created_at
                    ],
                )
                .map_err(|error| error.to_string())?;
            inserted.push(source);
            next_number += 1;
        }

        Ok(inserted)
    }

    pub fn get_research_sources(&self, task_id: &str) -> Result<Vec<ResearchSource>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库锁定失败".to_string())?;
        let mut statement = connection
            .prepare(
                "SELECT id, task_id, source_number, title, url, snippet, published_at,
                        source_domain, raw_content, score, source_query, created_at
                 FROM research_sources WHERE task_id = ?1 ORDER BY source_number ASC",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map(params![task_id], research_source_from_row)
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
    }

    pub fn get_research_activities(&self, task_id: &str) -> Result<Vec<ResearchActivity>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库锁定失败".to_string())?;
        let mut statement = connection
            .prepare(
                "SELECT id, task_id, activity_type, title, detail, created_at
                 FROM research_activities WHERE task_id = ?1 ORDER BY created_at ASC",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map(params![task_id], research_activity_from_row)
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
    }
}

fn research_task_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ResearchTask> {
    Ok(ResearchTask {
        id: row.get(0)?,
        conversation_id: row.get(1)?,
        user_message_id: row.get(2)?,
        assistant_message_id: row.get(3)?,
        topic: row.get(4)?,
        status: row.get(5)?,
        source_policy: row.get(6)?,
        domains_json: row.get(7)?,
        plan_json: row.get(8)?,
        report: row.get(9)?,
        error: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
        completed_at: row.get(13)?,
    })
}

fn research_source_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ResearchSource> {
    Ok(ResearchSource {
        id: row.get(0)?,
        task_id: row.get(1)?,
        source_number: row.get(2)?,
        title: row.get(3)?,
        url: row.get(4)?,
        snippet: row.get(5)?,
        published_at: row.get(6)?,
        source_domain: row.get(7)?,
        raw_content: row.get(8)?,
        score: row.get(9)?,
        source_query: row.get(10)?,
        created_at: row.get(11)?,
    })
}

fn research_activity_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ResearchActivity> {
    Ok(ResearchActivity {
        id: row.get(0)?,
        task_id: row.get(1)?,
        activity_type: row.get(2)?,
        title: row.get(3)?,
        detail: row.get(4)?,
        created_at: row.get(5)?,
    })
}
