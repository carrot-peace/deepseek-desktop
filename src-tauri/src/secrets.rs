const SERVICE: &str = "DeepSeek Desktop";

pub fn set_secret(key: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err("密钥不能为空".to_string());
    }
    keyring::Entry::new(SERVICE, key)
        .map_err(|error| error.to_string())?
        .set_password(value)
        .map_err(|error| error.to_string())
}

pub fn get_secret(key: &str) -> Result<String, String> {
    keyring::Entry::new(SERVICE, key)
        .map_err(|error| error.to_string())?
        .get_password()
        .map_err(|_| match key {
            "deepseek_api_key" => "DeepSeek API Key 未配置，请先在设置中填写。".to_string(),
            "tavily_api_key" => "Tavily API Key 未配置，请先在设置中填写。".to_string(),
            _ => "密钥未配置".to_string(),
        })
}

pub fn has_secret(key: &str) -> bool {
    keyring::Entry::new(SERVICE, key)
        .and_then(|entry| entry.get_password())
        .map(|value| !value.is_empty())
        .unwrap_or(false)
}

pub fn delete_secret(key: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE, key).map_err(|error| error.to_string())?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(_) => Ok(()),
    }
}
