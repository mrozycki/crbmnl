use anyhow::anyhow;
use chrono::{DateTime, Days, NaiveDate, NaiveTime, Utc};
use serde::Deserialize;

use crate::config::Config;

pub struct Calendar {
    config: Config,
    client: reqwest::Client,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Event {
    pub summary: String,
    pub start: DateMaybeTime,
    pub end: DateMaybeTime,
    pub description: Option<String>,
    pub location: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DateMaybeTime {
    DateTime(DateTime<Utc>),
    Date(NaiveDate),
}

impl DateMaybeTime {
    pub fn date(self) -> NaiveDate {
        match self {
            Self::DateTime(d) => d.date_naive(),
            Self::Date(d) => d,
        }
    }
}

impl Calendar {
    pub fn new(config: Config) -> Self {
        let client = reqwest::Client::new();
        Self { config, client }
    }

    pub async fn get_entries(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> anyhow::Result<Vec<Event>> {
        Ok(self
            .client
            .get(
                self.config
                    .home_assistant
                    .host
                    .join("api/calendars/")?
                    .join(&self.config.home_assistant.calendar_entity)?,
            )
            .query(&[("start", start), ("end", end)])
            .bearer_auth(&self.config.home_assistant.api_key)
            .send()
            .await?
            .json()
            .await?)
    }

    pub async fn get_next_n_days(&self, n: u64) -> anyhow::Result<Vec<Event>> {
        let today = Utc::now().with_timezone(&self.config.timezone).date_naive();
        let start = today
            .and_time(NaiveTime::MIN)
            .and_local_timezone(self.config.timezone)
            .earliest()
            .ok_or_else(|| anyhow!("failed to calculate midnight local time"))?
            .to_utc();
        let end = start
            .checked_add_days(Days::new(n))
            .ok_or_else(|| anyhow!("failed to calculate midnight 7 days from now"))?
            .to_utc();
        self.get_entries(start, end).await
    }
}
