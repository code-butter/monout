use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::Error;
use async_trait::async_trait;
use serde::Deserialize;
use anyhow::Result;
use aws_sdk_cloudwatchlogs::types::InputLogEvent;
use crate::{OutType, LogProcessor};

#[derive(Deserialize,Debug)]
pub struct AwsLogConfig {
    pub name: String,
    pub credentials: Option<AwsCredentials>,
    pub region: String,
    pub log_group: String,
    pub log_stream: String
}

#[derive(Deserialize,Debug,Clone)]
pub struct AwsCredentials {
    key: String,
    private_key: String
}

#[derive(Clone)]
pub struct AwsLogProcessor {
    name: String,
    region: String,
    log_group: String,
    log_stream: String,
    client: aws_sdk_cloudwatchlogs::Client
}

impl AwsLogProcessor {
    pub async fn from_config(config: &AwsLogConfig) -> Result<AwsLogProcessor> {
        // TODO: load from the config if available
        let aws_config = aws_config::load_from_env().await;
        let client = aws_sdk_cloudwatchlogs::Client::new(&aws_config);
        Ok(AwsLogProcessor {
            name: config.name.clone(),
            region: config.region.clone(),
            log_group: config.log_group.clone(),
            log_stream: config.log_stream.clone(),
            client
        })
    }
}

#[async_trait]
impl LogProcessor for AwsLogProcessor {
    fn get_name(&self) -> &str { &self.name }
    async fn log(&self, content: String, _out_type: &OutType) -> Result<(), Error> {
        let timestamp: Result<i64,_> = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_millis()
            .try_into();
        let log_event = InputLogEvent::builder()
            .message(content)
            .timestamp(timestamp?)
            .build();
        self.client
            .put_log_events()
            .log_group_name(&self.log_group)
            .log_stream_name(&self.log_stream)
            .log_events(log_event)
            .send()
            .await?;
        Ok(())
    }
}