use std::fs;

use serde::Deserialize;
use toml;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub server:   ServerConfig,
    pub registry: Option<RegistryConfig>,
    pub control:  ControlConfig,
}

#[derive(Deserialize, Debug)]
pub struct ServerConfig {
    pub name:      String,
    pub bind_host: String,
    pub bind_port: u16,
}

#[derive(Deserialize, Debug)]
pub struct RegistryConfig {
    pub public_host: String,
    pub public_port: u16,
    pub url:         String,
}

#[derive(Deserialize, Debug)]
pub struct ControlConfig {
    pub socket_path: String,
}

pub fn config_from_file(path: &str) -> anyhow::Result<Config> {
    let toml_config_str = fs::read_to_string(path)?;
    let toml_config: Config = toml::from_str(toml_config_str.as_str())?;
    Ok(toml_config)
}
