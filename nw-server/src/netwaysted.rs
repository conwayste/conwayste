use anyhow;
use clap::{self, Parser};
use serde::Deserialize;
use toml;

#[derive(Deserialize, Debug)]
struct Config {
    server: ServerConfig,
    registry: Option<RegistryConfig>,
    control: ControlConfig,
}

#[derive(Deserialize, Debug)]
struct ServerConfig {
    name: String,
    bind_host: String,
    bind_port: u16,
}

#[derive(Deserialize, Debug)]
struct RegistryConfig {
    public_host: String,
    public_port: u16,
    url: String,
}

#[derive(Deserialize, Debug)]
struct ControlConfig {
    socket_path: String,
}

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    config_file: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let toml_config_str = std::fs::read_to_string(args.config_file)?;
    let toml_config: Config = toml::from_str(toml_config_str.as_str())?;

    println!("{:#?}", toml_config);

    Ok(())
}
