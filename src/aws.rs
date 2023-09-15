use anyhow::Error;
use async_trait::async_trait;
use serde::Deserialize;
use anyhow::Result;
use aws_sdk_cloudwatchlogs::config::Region;
use aws_sdk_cloudwatchlogs::error::SdkError;
use aws_sdk_cloudwatchlogs::types::InputLogEvent;
use aws_smithy_runtime_api::client::orchestrator::{HttpRequest, HttpResponse};
use crate::{OutType, LogProcessor};

#[derive(Deserialize,Debug)]
pub struct AwsLogConfig {
    pub credentials: Option<AwsCredentials>,
    pub region: Option<String>,
    pub log_group: String,
    pub log_stream_prefix: String
}

#[derive(Deserialize,Debug,Clone)]
pub struct AwsCredentials {
    key: String,
    private_key: String
}

#[derive(Clone)]
pub struct AwsLogProcessor {
    log_group: String,
    log_stream_prefix: String,
    client: aws_sdk_cloudwatchlogs::Client,
    machine_id: Option<String>
}

fn show_aws_error<T,E>(result: std::result::Result<T,SdkError<E,HttpResponse>>) -> std::result::Result<T,SdkError<E,HttpResponse>> {
    if let Err(e) = &result {
        if let Some(response) = e.raw_response() {
            eprintln!("{}", response.status());
            if let Some(bytes) = response.body().bytes() {
                eprintln!("{}", String::from_utf8(bytes.to_vec()).unwrap())
            }
        }
    }
    result
}

impl AwsLogProcessor {
    pub async fn from_config(config: &AwsLogConfig, machine_id: Option<String>) -> AwsLogProcessor {
        let mut aws_builder = aws_config::from_env();
        // TODO: load credentials from the config if available
        if let Some(region) = &config.region {
            aws_builder = aws_builder.region(Region::new(region.clone()));
        }
        let aws_config = aws_builder.load().await;
        let client = aws_sdk_cloudwatchlogs::Client::new(&aws_config);
        AwsLogProcessor {
            log_group: config.log_group.clone(),
            log_stream_prefix: config.log_stream_prefix.clone(),
            client,
            machine_id
        }
    }

    pub async fn create_stream(&self) -> Result<()> {
        let result = self.client.create_log_stream()
            .log_group_name(&self.log_group)
            .log_stream_name(&self.get_stream_name())
            .send()
            .await;
        show_aws_error(result)?;
        Ok(())
    }

    pub fn get_stream_name(&self) -> String {
        let now = time::OffsetDateTime::now_utc();
        let date = format!("{}/{}/{}", now.year(), now.month(), now.day());
        match &self.machine_id {
            None => format!("{}/{}", date, self.log_stream_prefix),
            Some(machine_id) => format!("{}/{}/{}", date, self.log_stream_prefix, machine_id)
        }
    }
}

#[async_trait]
impl LogProcessor for AwsLogProcessor {
    async fn log(&self, timestamp: i64, content: String, _out_type: &OutType) -> Result<(), Error> {
        let log_event = InputLogEvent::builder()
            .message(content)
            .timestamp(timestamp)
            .build();
        let result = self.client
            .put_log_events()
            .log_group_name(&self.log_group)
            .log_stream_name(&self.get_stream_name())
            .log_events(log_event)
            .send()
            .await;
        show_aws_error(result)?;
        Ok(())
    }
}