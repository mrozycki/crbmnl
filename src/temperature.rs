use crate::config::{Config, TemperatureDeviceConfig};

pub struct Temperature {
    client: reqwest::Client,
    config: Config,
}

pub struct TemperatureReportEntry {
    pub temperature: f64,
    pub humidity: f64,
}

pub struct TemperatureReport {
    pub primary: TemperatureReportEntry,
    pub secondaries: Vec<(String, TemperatureReportEntry)>,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct SensorResponse {
    state: String,
}

impl Temperature {
    pub fn new(config: Config) -> Self {
        let client = reqwest::Client::new();
        Self { config, client }
    }

    pub async fn get_data(&self) -> anyhow::Result<TemperatureReport> {
        let mut secondaries = Vec::new();
        for secondary in self.config.temperature.secondaries.iter() {
            secondaries.push((
                secondary.name.clone(),
                self.get_data_for_device(secondary).await?,
            ));
        }

        Ok(TemperatureReport {
            primary: self
                .get_data_for_device(&self.config.temperature.primary)
                .await?,
            secondaries,
        })
    }

    async fn get_data_for_device(
        &self,
        device: &TemperatureDeviceConfig,
    ) -> anyhow::Result<TemperatureReportEntry> {
        Ok(TemperatureReportEntry {
            temperature: self
                .get_value_for_entity(&device.temperature_entity)
                .await?,
            humidity: self.get_value_for_entity(&device.humidity_entity).await?,
        })
    }

    async fn get_value_for_entity(&self, entity: &str) -> anyhow::Result<f64> {
        Ok(self
            .client
            .get(
                self.config
                    .home_assistant
                    .host
                    .join("api/states/")?
                    .join(entity)?,
            )
            .bearer_auth(&self.config.home_assistant.api_key)
            .send()
            .await?
            .json::<SensorResponse>()
            .await?
            .state
            .parse()?)
    }
}
