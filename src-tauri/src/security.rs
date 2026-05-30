use crate::models::{AppSettings, SendMessageRequest};

pub const DEEPSEEK_BASE_URL: &str = "https://api.deepseek.com";
pub const MAX_USER_MESSAGE_CHARS: usize = 32 * 1024;
pub const MAX_ASSISTANT_OUTPUT_CHARS: usize = 256 * 1024;
pub const MAX_SSE_BUFFER_BYTES: usize = 1024 * 1024;
pub const DEEPSEEK_REQUEST_TIMEOUT_SECS: u64 = 5 * 60;

const ALLOWED_SECRET_KEYS: &[&str] = &["deepseek_api_key", "tavily_api_key"];

pub fn normalize_deepseek_base_url(value: &str) -> Result<String, String> {
    let trimmed = value.trim().trim_end_matches('/');
    if trimmed != DEEPSEEK_BASE_URL {
        return Err(format!(
            "DeepSeek Base URL 只能使用官方端点：{DEEPSEEK_BASE_URL}"
        ));
    }

    Ok(DEEPSEEK_BASE_URL.to_string())
}

pub fn normalize_settings(mut settings: AppSettings) -> Result<AppSettings, String> {
    settings.deepseek_base_url = normalize_deepseek_base_url(&settings.deepseek_base_url)?;
    Ok(settings)
}

pub fn normalize_settings_or_default(mut settings: AppSettings) -> AppSettings {
    settings.deepseek_base_url = normalize_deepseek_base_url(&settings.deepseek_base_url)
        .unwrap_or_else(|_| DEEPSEEK_BASE_URL.to_string());
    settings
}

pub fn validate_secret_key(key: &str) -> Result<(), String> {
    if ALLOWED_SECRET_KEYS.contains(&key) {
        Ok(())
    } else {
        Err("不支持的密钥名称。".to_string())
    }
}

pub fn validate_send_message_request(request: &SendMessageRequest) -> Result<(), String> {
    if request.content.chars().count() > MAX_USER_MESSAGE_CHARS {
        return Err(format!(
            "消息过长，请控制在 {} 字符以内。",
            MAX_USER_MESSAGE_CHARS
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_official_deepseek_origin() {
        assert_eq!(
            normalize_deepseek_base_url("https://api.deepseek.com").unwrap(),
            DEEPSEEK_BASE_URL
        );
        assert_eq!(
            normalize_deepseek_base_url(" https://api.deepseek.com/ ").unwrap(),
            DEEPSEEK_BASE_URL
        );
    }

    #[test]
    fn rejects_non_official_deepseek_urls() {
        for value in [
            "http://api.deepseek.com",
            "https://api.deepseek.com.evil.example",
            "https://evil.example/api.deepseek.com",
            "https://api.deepseek.com/v1",
            "https://api.deepseek.com:443",
            "https://api.deepseek.com?next=https://evil.example",
            "https://127.0.0.1",
            "file:///tmp/key",
        ] {
            assert!(normalize_deepseek_base_url(value).is_err(), "{value}");
        }
    }

    #[test]
    fn allows_only_known_secret_keys() {
        assert!(validate_secret_key("deepseek_api_key").is_ok());
        assert!(validate_secret_key("tavily_api_key").is_ok());
        assert!(validate_secret_key("other_key").is_err());
    }

    #[test]
    fn rejects_overlong_user_messages() {
        let request = SendMessageRequest {
            conversation_id: "conversation".to_string(),
            content: "x".repeat(MAX_USER_MESSAGE_CHARS + 1),
            model: "deepseek-v4-pro".to_string(),
            thinking_mode: "off".to_string(),
            search_enabled: false,
        };

        assert!(validate_send_message_request(&request).is_err());
    }
}
