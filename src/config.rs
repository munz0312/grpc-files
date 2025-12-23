use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub server_bind_address: String,
    pub server_connect_address: String,
    pub upload_directory: String,
    pub download_directory: String,
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let home_dir = std::env::var("HOME")
            .map_err(|_| "HOME environment variable not set")?;
        let config_path = PathBuf::from(home_dir)
            .join(".file_server")
            .join("config.json");

        if !config_path.exists() {
            return Err(format!(
                "Config file not found at {}. Please create it with the following structure:\n\n\
                 {{\n  \"server_bind_address\": \"0.0.0.0:50051\",\n  \"server_connect_address\": \"192.168.1.xxx:50051\",\n  \"upload_directory\": \"/path/to/uploads\",\n  \"download_directory\": \"/path/to/downloads\"\n}}",
                config_path.display()
            ).into());
        }

        let config_content = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;

        let config: Config = serde_json::from_str(&config_content)
            .map_err(|e| format!("Failed to parse config.json: {}", e))?;

        Ok(config)
    }

    pub fn get_auth_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let home_dir = std::env::var("HOME")
            .map_err(|_| "HOME environment variable not set")?;
        let auth_dir = PathBuf::from(home_dir).join(".file_server").join("auth");

        if !auth_dir.exists() {
            return Err(format!(
                "Auth directory not found at {}. Please ensure it contains your certificate files.",
                auth_dir.display()
            ).into());
        }

        Ok(auth_dir)
    }
}
