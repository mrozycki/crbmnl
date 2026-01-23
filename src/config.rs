use url::Url;

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long, value_name = "FILE")]
    pub config: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Config {
    pub base_url: Url,
    pub port: u16,
    pub timezone: chrono_tz::Tz,
    pub home_assistant: HomeAssistantConfig,
    pub temperature: TemperatureConfig,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct HomeAssistantConfig {
    pub host: Url,
    pub api_key: String,
    pub calendar_entity: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct TemperatureConfig {
    pub primary: TemperatureDeviceConfig,
    pub secondaries: Vec<TemperatureDeviceConfig>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct TemperatureDeviceConfig {
    pub name: String,
    pub temperature_entity: String,
    pub humidity_entity: String,
}
