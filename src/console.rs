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

    fn get_name(&self) -> &str { &self.name }

    async fn log(&self, content: String, out_type: &OutType) -> anyhow::Result<()> {
        match out_type {
            OutType::Std => println!("{}", content),
            OutType::Err => eprintln!("{}", content)
        };
        Ok(())
    }
}