use async_trait::async_trait;
use crate::{LogProcessor, OutType};

pub struct ConsoleLogConfig {
    pub name: String
}

impl Into<ConsoleLogProcessor> for ConsoleLogConfig {
    fn into(self) -> ConsoleLogProcessor {
        ConsoleLogProcessor {
            name: self.name
        }
    }
}

pub struct ConsoleLogProcessor {
    pub name: String
}

#[async_trait]
impl LogProcessor for ConsoleLogProcessor {

    async fn log(&self, timestamp: i64, content: String, out_type: &OutType) -> anyhow::Result<()> {
        match out_type {
            OutType::Std => print!("{} {}", timestamp, content),
            OutType::Err => eprint!("{} {}", timestamp, content)
        };
        Ok(())
    }
}