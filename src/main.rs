extern crate core;

mod console;
mod aws;
mod utils;

use std::collections::BTreeMap;
use std::env;
use std::fs::File;
use std::io::Read;
use std::process::{exit, Stdio};
use std::sync::Arc;
use async_trait::async_trait;
use anyhow::Result;
use serde::Deserialize;
use time::OffsetDateTime;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::{Child, Command};
use tokio::task::JoinSet;
use crate::aws::{AwsLogConfig, AwsLogProcessor};
use crate::console::ConsoleLogProcessor;

enum OutType { Std, Err }

#[async_trait]
trait LogProcessor: Sync + Send {
    fn get_name(&self) -> &str;
    async fn log(&self, content: String, out_type: &OutType) -> Result<()>;
}

#[derive(Deserialize)]
struct Config {
    restart_delay: Option<usize>,
    #[serde(default)]
    console_labels: bool,
    #[serde(flatten)]
    runners: BTreeMap<String, Runner>
}

#[derive(Deserialize)]
struct Runner {
    command: String,
    output_type: String,
    restart_delay: Option<usize>,
    aws: Option<AwsLogConfig>
}

struct Process {
    last_started: Option<OffsetDateTime>,
    show_console_label: bool,
    runner: Runner,
    log_processor: Option<Arc<dyn LogProcessor>>,
    child: Option<Child>
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

async fn read_stream<R, T>(stream: R, out_type: &OutType, processor: Arc<T>) -> Result<()>
    where
        R: AsyncRead + Unpin,
        T: LogProcessor + ?Sized
{
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
        processor.log(line.clone(), out_type).await?;
    };
    Ok(())
}

async fn start_process(name: &str, mut process: Process) -> Result<(impl std::future::Future<Output = Result<()>>, impl std::future::Future<Output = Result<()>>)> {
    let mut child = run_process(&process.runner.command)?;
    let stdout = child.stdout.take().expect(&*format!("Could not get stdout for {}", name));
    let stderr = child.stderr.take().expect(&*format!("Could not get stderr for {}", name));
    let processor = match process.log_processor {
        None => {
            let processor: Arc<dyn LogProcessor> = match process.runner.output_type.as_str() {
                "console" => {
                    Arc::new(ConsoleLogProcessor { name: name.to_owned() })
                },
                "aws" => {
                    match process.runner.aws {
                        None => panic!("{} requires AWS options to be set", name),
                        Some(aws_config) => {
                            let log_processor: AwsLogProcessor = aws_config.into();
                            Arc::new(log_processor)
                        }
                    }
                },
                _ => {
                    panic!("Unknown output_type for {}: {}", name, process.runner.output_type);
                }
            };
            let clone = processor.clone();
            process.log_processor = Some(processor);
            clone
        },
        Some(lp) => lp.clone()
    };
    let out_task = read_stream(stdout, &OutType::Std, processor.clone());
    let err_task = read_stream(stderr, &OutType::Err, processor.clone());
    Ok((out_task, err_task))
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 1 {
        eprintln!("monout only takes one argument: the location of the configuration file.");
        exit(1);
    }
    let mut file = File::open(&args[0])?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    let Config { restart_delay, console_labels, runners }: Config = toml::from_str(&content)?;
    let mut processes: BTreeMap<String, Process> = BTreeMap::new();
    for (name, mut runner) in runners.into_iter() {
        if runner.restart_delay.is_none() {
            runner.restart_delay = restart_delay.clone();
        }
        processes.insert(name, Process {
            last_started: None,
            show_console_label: console_labels,
            runner,
            log_processor: None,
            child: None,
        });
    }
    
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
