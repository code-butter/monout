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
use std::time::Duration;
use async_trait::async_trait;
use anyhow::Result;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use serde::Deserialize;
use time::OffsetDateTime;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdout, Command};
use tokio::task::JoinHandle;
use tokio::time::sleep;
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

async fn start_process(name: &str, process: &mut Process) -> Result<(ChildStdout, ChildStderr)> {
    let mut child = run_process(&process.runner.command)?;
    let stdout = child.stdout.take().expect(&*format!("Could not get stdout for {}", name));
    let stderr = child.stderr.take().expect(&*format!("Could not get stderr for {}", name));
    let processor = match &process.log_processor {
        None => {
            let processor: Arc<dyn LogProcessor> = match process.runner.output_type.as_str() {
                "console" => {
                    Arc::new(ConsoleLogProcessor { name: name.to_owned() })
                },
                "aws" => {
                    match &process.runner.aws {
                        None => panic!("{} requires AWS options to be set", name),
                        Some(aws_config) => {
                            Arc::new(AwsLogProcessor::from_config(aws_config))
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
    Ok((stdout, stderr))
}

async fn restart_processes(processes: &mut BTreeMap<String, Process>, futures: &mut FuturesUnordered<JoinHandle<Result<()>>>) -> Result<()> {
    for (name, process) in processes.iter_mut() {
        if process.child.as_ref().map(|c| c.id()).is_none() {
            let outs = start_process(name, process).await?;
            let processor = process.log_processor.clone().unwrap();
            let out_task = tokio::spawn(read_stream(outs.0, &OutType::Std, processor.clone()));
            let err_task = tokio::spawn(read_stream(outs.1, &OutType::Err, processor.clone()));
            futures.push(out_task);
            futures.push(err_task);
        }
    }
    Ok(())
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

    let mut futures = FuturesUnordered::new();
    restart_processes(&mut processes, &mut futures);
    loop {
        match futures.next().await {
            None => { sleep(Duration::from_millis(100)).await; }
            Some(result) => {
                if let Err(error) = result {
                    eprintln!("monout: A task has failed: {}", error);
                }
            }
        }
        restart_processes(&mut processes, &mut futures);
    }
}
