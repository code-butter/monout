use anyhow::Error;
use async_trait::async_trait;
use serde::Deserialize;
use crate::{OutType, LogProcessor};

#[derive(Deserialize,Debug)]
pub struct AwsLogConfig {
    pub name: String,
    pub credentials: Option<AwsCredentials>,
    pub region: String,
    pub log_group: String,
    pub log_stream_prefix: String
}

#[derive(Deserialize,Debug)]
pub struct AwsCredentials {
    key: String,
    private_key: String
}

impl Into<AwsLogProcessor> for AwsLogConfig {
    fn into(self) -> AwsLogProcessor {
        AwsLogProcessor {
            name: self.name,
            region: self.region,
            log_group: self.log_group,
            log_stream_prefix: self.log_stream_prefix
        }
    }
}

#[derive(Clone)]
pub struct AwsLogProcessor {
    name: String,
    region: String,
    log_group: String,
    log_stream_prefix: String
}

impl AwsLogProcessor {
    pub fn from_config(config: &AwsLogConfig) -> AwsLogProcessor {
        AwsLogProcessor {
            name: config.name.clone(),
            region: config.region.clone(),
            log_group: config.log_group.clone(),
            log_stream_prefix: config.log_stream_prefix.clone()
        }
    }
}

#[async_trait]
impl LogProcessor for AwsLogProcessor {
    fn get_name(&self) -> &str { &self.name }
    async fn log(&self, content: String, out_type: &OutType) -> Result<(), Error> {
        todo!()
    }
}