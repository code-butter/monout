mod console;
mod aws;
mod utils;

use std::collections::BTreeMap;
use std::process::Stdio;
use std::sync::Arc;
use async_trait::async_trait;
use anyhow::Result;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::{Child, Command};
use tokio::task::JoinSet;
use crate::aws::AwsLogProcessor;
use crate::console::ConsoleLogProcessor;

enum OutType { Std, Err }

struct LogProcessorInstances<'a> {
    processors: BTreeMap<String, Box<dyn LogProcessor + 'a>>
}

impl<'a> LogProcessorInstances<'a> {
    fn new() -> Self {
        LogProcessorInstances { processors: BTreeMap::new() }
    }

    fn add<P: LogProcessor + 'a>(&mut self, processor: P) -> Result<()> {
        let name = processor.get_name().to_owned();
        self.processors.insert(name, Box::new(processor));
        Ok(())
    }
}

#[async_trait]
trait LogProcessor: Sync + Send {
    fn get_name(&self) -> &str;
    async fn log(&self, content: String, out_type: &OutType) -> Result<()>;
}

struct Runner {
    command: String,
    aws: Option<AwsLogProcessor>
}

fn run_process(command: &str) -> Result<Child> {
    let child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    Ok(child)
}

async fn read_stream<R, T>(mut stream: R, out_type: &OutType, processor: Arc<T>) -> Result<()>
    where
        R: AsyncRead + Unpin,
        T: LogProcessor
{
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
        processor.log(line.clone(), out_type).await?;
    };
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut process = run_process("echo Hi! && sleep 10")?;
    let log_processor = Arc::new(ConsoleLogProcessor { name: "Test".to_owned() });
    let stdout = process.stdout.take().expect("Could not get stdout");
    let stderr = process.stderr.take().expect("Could not get stderr");
    let mut set = JoinSet::new();
    set.spawn(read_stream(stdout, &OutType::Std, log_processor.clone()));
    set.spawn(read_stream(stderr, &OutType::Err, log_processor.clone()));
    while let Some(_) = set.join_next().await {}
    Ok(())
}
