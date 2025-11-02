use once_cell::sync::Lazy;
use serde::Deserialize;
use std::fs;

const DEFAULT_CONFIG_PATH: &str = "config.toml";

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let path = env::var("CONFIG_PATH").unwrap_or(DEFAULT_CONFIG_PATH.to_string());
    read_config(&path).expect("Failed to load config")
});

#[derive(Deserialize)]
pub struct Config {
    pub main: MainConfig,
    pub database: DatabaseConfig,
}

#[derive(Deserialize)]
pub struct MainConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
}

fn read_config(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let config_content = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&config_content)?;
    Ok(config)
}
