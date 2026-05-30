use crate::deepseek::send_deepseek_message;
use crate::models::{
    AppSettings, ChatErrorEvent, ChatMessage, ChatStartedEvent, Conversation, SendMessageRequest,
};
use crate::secrets;
use crate::security::{
    normalize_settings, normalize_settings_or_default, validate_secret_key,
    validate_send_message_request,
};
use crate::AppState;
use chrono::Utc;
use tauri::{AppHandle, Emitter, State};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[tauri::command]
pub async fn get_conversations(state: State<'_, AppState>) -> Result<Vec<Conversation>, String> {
    state.db.get_conversations()
}

#[tauri::command]
pub async fn create_conversation(state: State<'_, AppState>) -> Result<Conversation, String> {
    let settings = state.db.get_settings().unwrap_or_default();
    let now = Utc::now().to_rfc3339();
    let conversation = Conversation {
        id: Uuid::new_v4().to_string(),
        title: "新会话".to_string(),
        model: settings.default_model,
        thinking_mode: settings.default_thinking_mode,
        search_enabled: settings.default_search_enabled,
        created_at: now.clone(),
        updated_at: now,
    };
    state.db.upsert_conversation(&conversation)?;
    Ok(conversation)
}

#[tauri::command]
pub async fn update_conversation(
    conversation: Conversation,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.db.upsert_conversation(&conversation)
}

#[tauri::command]
pub async fn delete_conversation(
    conversation_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.db.delete_conversation(&conversation_id)
}

#[tauri::command]
pub async fn get_messages(
    conversation_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ChatMessage>, String> {
    state.db.get_messages(&conversation_id)
}

#[tauri::command]
pub async fn save_message(message: ChatMessage, state: State<'_, AppState>) -> Result<(), String> {
    state.db.save_message(&message)
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    state.db.get_settings().map(normalize_settings_or_default)
}

#[tauri::command]
pub async fn save_settings(
    settings: AppSettings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let settings = normalize_settings(settings)?;
    state.db.save_settings(&settings)
}

#[tauri::command]
pub async fn set_secret(key: String, value: String) -> Result<(), String> {
    validate_secret_key(&key)?;
    secrets::set_secret(&key, &value)
}

#[tauri::command]
pub async fn has_secret(key: String) -> Result<bool, String> {
    validate_secret_key(&key)?;
    Ok(secrets::has_secret(&key))
}

#[tauri::command]
pub async fn delete_secret(key: String) -> Result<(), String> {
    validate_secret_key(&key)?;
    secrets::delete_secret(&key)
}

#[tauri::command]
pub async fn send_message(
    request: SendMessageRequest,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    validate_send_message_request(&request)?;
    let api_key = secrets::get_secret("deepseek_api_key")?;
    let settings = normalize_settings(state.db.get_settings().unwrap_or_default())?;
    let now = Utc::now().to_rfc3339();
    let user_message = ChatMessage {
        id: Uuid::new_v4().to_string(),
        conversation_id: request.conversation_id.clone(),
        role: "user".to_string(),
        content: request.content.clone(),
        reasoning_content: None,
        tool_calls_json: None,
        tool_result_json: None,
        created_at: now.clone(),
    };
    state.db.save_message(&user_message)?;

    if let Some(mut conversation) = state
        .db
        .get_conversations()?
        .into_iter()
        .find(|item| item.id == request.conversation_id)
    {
        if conversation.title == "新会话" {
            conversation.title = request.content.chars().take(24).collect();
        }
        conversation.model = request.model.clone();
        conversation.thinking_mode = request.thinking_mode.clone();
        conversation.search_enabled = request.search_enabled;
        conversation.updated_at = now;
        state.db.upsert_conversation(&conversation)?;
    }

    let assistant_message_id = Uuid::new_v4().to_string();
    let _ = app.emit(
        "chat:started",
        ChatStartedEvent {
            conversation_id: request.conversation_id.clone(),
            message_id: assistant_message_id.clone(),
        },
    );

    let cancellation = CancellationToken::new();
    {
        let mut cancellations = state
            .cancellations
            .lock()
            .map_err(|_| "取消状态锁定失败".to_string())?;
        cancellations.insert(request.conversation_id.clone(), cancellation.clone());
    }

    let history = state.db.get_messages(&request.conversation_id)?;
    let result = send_deepseek_message(
        app.clone(),
        api_key,
        settings.deepseek_base_url,
        request.clone(),
        history,
        assistant_message_id.clone(),
        cancellation,
    )
    .await;

    {
        let mut cancellations = state
            .cancellations
            .lock()
            .map_err(|_| "取消状态锁定失败".to_string())?;
        cancellations.remove(&request.conversation_id);
    }

    match result {
        Ok(message) => {
            state.db.save_message(&message)?;
            let _ = app.emit(
                "chat:done",
                ChatStartedEvent {
                    conversation_id: request.conversation_id,
                    message_id: assistant_message_id,
                },
            );
            Ok(())
        }
        Err(error) => {
            let _ = app.emit(
                "chat:error",
                ChatErrorEvent {
                    conversation_id: request.conversation_id,
                    message_id: Some(assistant_message_id),
                    error: error.clone(),
                },
            );
            Err(error)
        }
    }
}

#[tauri::command]
pub async fn stop_generation(
    conversation_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let cancellations = state
        .cancellations
        .lock()
        .map_err(|_| "取消状态锁定失败".to_string())?;
    if let Some(token) = cancellations.get(&conversation_id) {
        token.cancel();
    }
    Ok(())
}
