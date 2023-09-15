extern crate core;

mod console;
mod aws;
mod process;
mod futures_counter;

use std::collections::BTreeMap;
use std::env;
use std::fs::File;
use std::io::Read;
use std::process::exit;
use async_trait::async_trait;
use anyhow::Result;
use lazy_static::lazy_static;
use regex::Regex;
use serde::Deserialize;
use crate::aws::{AwsCredentials, AwsLogConfig};
use crate::futures_counter::FuturesCounter;
use crate::process::Process;

lazy_static! {
    static ref ENV_VAR_MATCHER: Regex = Regex::new(r"^\$\w+$").unwrap();
}

pub enum OutType { Std, Err }

#[async_trait]
pub trait LogProcessor: Sync + Send {
    async fn log(&self, timestamp: i64, content: String, out_type: &OutType) -> Result<()>;
}

#[derive(Deserialize)]
pub struct Config {
    failure_restart_delay: Option<u64>,
    #[serde(default)]
    console_labels: bool,
    machine_id: Option<String>,
    aws_credentials: Option<AwsCredentials>,
    #[serde(flatten)]
    runners: BTreeMap<String, Runner>
}

#[derive(Deserialize)]
pub struct Runner {
    command: String,
    output_type: String,
    failure_restart_delay: Option<u64>,
    machine_id: Option<String>,
    aws: Option<AwsLogConfig>
}

fn env_replace<T: Into<Option<String>>>(value: T) -> Option<String> {
    let optional_str = value.into();
    match optional_str {
        None => None,
        Some(str) => {
            let trimmed = str.trim();
            if ENV_VAR_MATCHER.is_match(trimmed) {
                env::var(trimmed.replace("$", "")).ok()
            } else {
                Some(str.to_owned())
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("monout only takes one argument: the location of the configuration file.");
        exit(1);
    }
    let mut file = File::open(&args[1])?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    let Config { failure_restart_delay: restart_delay, console_labels, runners, aws_credentials, mut machine_id }: Config = serde_yaml::from_str(&content)?;
    machine_id = env_replace(machine_id);
    let mut processes: BTreeMap<String, Process> = BTreeMap::new();
    for (name, mut runner) in runners.into_iter() {
        if runner.failure_restart_delay.is_none() {
            runner.failure_restart_delay = restart_delay;
        }
        match runner.machine_id {
            None => { runner.machine_id = machine_id.clone(); }
            Some(str) => { runner.machine_id = env_replace(str); }
        }
        if let Some(aws) = &mut runner.aws {
            if aws.credentials.is_none() {
                aws.credentials = aws_credentials.clone();
            }
        }
        let mut process = Process::from_runner(&name, runner).await?;
        process.show_console_label = console_labels;
        processes.insert(name,process);
    }
    let mut futures = FuturesCounter::new();
    for (_, process) in processes {
        let static_ref: &'static mut Process = Box::leak(Box::new(process));
        futures.push(tokio::spawn(static_ref.run()))
    };
    while !futures.is_empty() {
        futures.next().await;
    }
    Ok(())
}
