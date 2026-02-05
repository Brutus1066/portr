//! Configuration file support for portr
//!
//! Loads settings from `~/.config/portr/config.toml` (Linux/macOS)
//! or `%APPDATA%\portr\config.toml` (Windows)

use std::collections::HashMap;
use std::path::PathBuf;

/// Application configuration loaded from config file
#[derive(Debug, Clone, Default)]
pub struct Config {
    /// Default settings
    pub defaults: Defaults,
    /// Port aliases (e.g., "react" -> 3000)
    pub aliases: HashMap<String, u16>,
    /// Theme customization
    pub theme: Theme,
}

/// Default behavior settings
#[derive(Debug, Clone)]
pub struct Defaults {
    /// Kill signal to use (SIGTERM or SIGKILL)
    pub signal: String,
    /// Whether to confirm before killing
    pub confirm: bool,
    /// Color output mode: auto, always, never
    pub color: String,
    /// Default output format
    pub format: String,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            signal: "SIGTERM".to_string(),
            confirm: true,
            color: "auto".to_string(),
            format: "pretty".to_string(),
        }
    }
}

/// Theme customization
#[derive(Debug, Clone)]
pub struct Theme {
    pub banner_color: String,
    pub success_color: String,
    pub warning_color: String,
    pub error_color: String,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            banner_color: "cyan".to_string(),
            success_color: "green".to_string(),
            warning_color: "yellow".to_string(),
            error_color: "red".to_string(),
        }
    }
}

/// Get the config file path for the current platform
pub fn config_path() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var("APPDATA")
            .ok()
            .map(|p| PathBuf::from(p).join("portr").join("config.toml"))
    }

    #[cfg(not(windows))]
    {
        std::env::var("HOME")
            .ok()
            .map(|p| PathBuf::from(p).join(".config").join("portr").join("config.toml"))
    }
}

/// Load configuration from the config file
pub fn load_config() -> Config {
    let path = match config_path() {
        Some(p) => p,
        None => return Config::default(),
    };

    if !path.exists() {
        return Config::default();
    }

    match std::fs::read_to_string(&path) {
        Ok(content) => parse_config(&content),
        Err(_) => Config::default(),
    }
}

/// Parse TOML config content
fn parse_config(content: &str) -> Config {
    let mut config = Config::default();

    // Simple TOML parser for our limited config format
    let mut current_section = "";

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Section header
        if line.starts_with('[') && line.ends_with(']') {
            current_section = &line[1..line.len() - 1];
            continue;
        }

        // Key = value pair
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim();
            let value = line[eq_pos + 1..].trim().trim_matches('"');

            match current_section {
                "defaults" => match key {
                    "signal" => config.defaults.signal = value.to_string(),
                    "confirm" => config.defaults.confirm = value == "true",
                    "color" => config.defaults.color = value.to_string(),
                    "format" => config.defaults.format = value.to_string(),
                    _ => {}
                },
                "aliases" => {
                    if let Ok(port) = value.parse::<u16>() {
                        config.aliases.insert(key.to_string(), port);
                    }
                }
                "theme" => match key {
                    "banner_color" => config.theme.banner_color = value.to_string(),
                    "success_color" => config.theme.success_color = value.to_string(),
                    "warning_color" => config.theme.warning_color = value.to_string(),
                    "error_color" => config.theme.error_color = value.to_string(),
                    _ => {}
                },
                _ => {}
            }
        }
    }

    config
}

/// Resolve a port alias to its actual port number
pub fn resolve_alias(alias: &str, config: &Config) -> Option<u16> {
    config.aliases.get(alias).copied()
}

/// Check if a string is a port number or could be an alias
pub fn is_port_or_alias(s: &str) -> bool {
    s.parse::<u16>().is_ok() || s.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

/// Generate a default config file content
pub fn default_config_content() -> String {
    r#"# portr configuration file
# Location: ~/.config/portr/config.toml (Linux/macOS)
#           %APPDATA%\portr\config.toml (Windows)

[defaults]
# Kill signal: SIGTERM (graceful) or SIGKILL (force)
signal = "SIGTERM"

# Prompt before killing processes
confirm = true

# Color mode: auto, always, never
color = "auto"

# Default output format: pretty, json, csv, md
format = "pretty"

[aliases]
# Port aliases for quick access
# Usage: portr react → portr 3000
react = 3000
next = 3000
vite = 5173
vue = 8080
angular = 4200
backend = 8080
api = 8000
flask = 5000
django = 8000
rails = 3000
postgres = 5432
mysql = 3306
redis = 6379
mongo = 27017
ollama = 11434
docker = 2375

[theme]
# Color customization
banner_color = "cyan"
success_color = "green"
warning_color = "yellow"
error_color = "red"
"#
    .to_string()
}

/// Create config directory and write default config if it doesn't exist
pub fn init_config() -> Result<PathBuf, String> {
    let path = config_path().ok_or("Could not determine config path")?;

    if path.exists() {
        return Err(format!("Config already exists at: {}", path.display()));
    }

    // Create parent directories
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    // Write default config
    std::fs::write(&path, default_config_content())
        .map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.defaults.signal, "SIGTERM");
        assert!(config.defaults.confirm);
        assert_eq!(config.defaults.color, "auto");
    }

    #[test]
    fn test_parse_config_aliases() {
        let content = r#"
[aliases]
react = 3000
backend = 8080
db = 5432
"#;
        let config = parse_config(content);
        assert_eq!(config.aliases.get("react"), Some(&3000));
        assert_eq!(config.aliases.get("backend"), Some(&8080));
        assert_eq!(config.aliases.get("db"), Some(&5432));
    }

    #[test]
    fn test_parse_config_defaults() {
        let content = r#"
[defaults]
signal = "SIGKILL"
confirm = false
color = "never"
"#;
        let config = parse_config(content);
        assert_eq!(config.defaults.signal, "SIGKILL");
        assert!(!config.defaults.confirm);
        assert_eq!(config.defaults.color, "never");
    }

    #[test]
    fn test_resolve_alias() {
        let mut config = Config::default();
        config.aliases.insert("react".to_string(), 3000);
        config.aliases.insert("db".to_string(), 5432);

        assert_eq!(resolve_alias("react", &config), Some(3000));
        assert_eq!(resolve_alias("db", &config), Some(5432));
        assert_eq!(resolve_alias("unknown", &config), None);
    }

    #[test]
    fn test_is_port_or_alias() {
        assert!(is_port_or_alias("3000"));
        assert!(is_port_or_alias("react"));
        assert!(is_port_or_alias("my-app"));
        assert!(is_port_or_alias("app_123"));
    }

    #[test]
    fn test_config_path_exists() {
        // Just verify it returns something
        let path = config_path();
        assert!(path.is_some());
    }
}
