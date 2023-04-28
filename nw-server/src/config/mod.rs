use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub server: ServerConfig,
    pub registry: Option<RegistryConfig>,
    pub control: ControlConfig,
}

#[derive(Deserialize, Debug)]
pub struct ServerConfig {
    pub name: String,
    pub bind_host: String,
    pub bind_port: u16,
}

#[derive(Deserialize, Debug)]
pub struct RegistryConfig {
    pub public_host: String,
    pub public_port: u16,
    pub url: String,
}

#[derive(Deserialize, Debug)]
pub struct ControlConfig {
    pub socket_path: String,
}
