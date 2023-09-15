use std::process::{ExitStatus, Stdio};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::process::{Child, Command};
use anyhow::{anyhow, Result};
use time::OffsetDateTime;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::time::sleep;
use crate::{LogProcessor, OutType, Runner};
use crate::aws::AwsLogProcessor;
use crate::console::ConsoleLogProcessor;

async fn read_stream<R, T>(stream: R, out_type: &OutType, processor: Arc<T>) -> Result<()>
    where
        R: AsyncRead + Unpin,
        T: LogProcessor + ?Sized
{
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
        let timestamp: Result<i64,_> = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_millis()
            .try_into();
        if let Err(e) = processor.log(timestamp?, line.clone(), out_type).await {
            eprintln!("Log processor returned error: {}", e);
        }
    };
    Ok(())
}

pub struct ManagedProcess {
    child: Child,
    log_processor: Arc<dyn LogProcessor>
}

impl ManagedProcess {
    pub fn start(command: &str, log_processor: Arc<dyn LogProcessor>) -> Result<Self> {
        let child = Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        Ok(Self { child, log_processor })
    }

    pub fn id(&self) -> Option<u32> {
        self.child.id()
    }

    pub async fn wait(&mut self) -> Result<ExitStatus> {
        let stdout = self.child.stdout.take().unwrap();
        let stderr = self.child.stderr.take().unwrap();
        let out_handle = tokio::spawn(read_stream(stdout, &OutType::Std, self.log_processor.clone()));
        let err_handle = tokio::spawn(read_stream(stderr, &OutType::Err, self.log_processor.clone()));
        let exit_status = self.child.wait().await?;
        out_handle.abort();
        err_handle.abort();
        Ok(exit_status)
    }

}

pub struct Process {
    last_started: Option<OffsetDateTime>,
    pub(crate) show_console_label: bool,
    runner: Runner,
    log_processor: Arc<dyn LogProcessor>,
    managed_process: Option<ManagedProcess>,
    running: bool
}

impl Process {

    async fn get_processor(name: &str, runner: &mut Runner) -> Result<Arc<dyn LogProcessor>> {
        let output_type = runner.output_type.as_str();
        if output_type == "console" {
            return Ok(Arc::new(ConsoleLogProcessor { name: name.to_owned() }));
        }
        if output_type == "aws" {
            return match &mut runner.aws {
                None => {
                    Err(anyhow!("{} requires AWS options to be set", name))
                },
                Some(aws_config) => {
                    let log_processor = AwsLogProcessor::from_config(aws_config, runner.machine_id.clone()).await;
                    if let Err(e) = log_processor.create_stream().await {
                        eprintln!("Unable to create AWS log stream: {}", e)
                    }
                    Ok(Arc::new(log_processor))
                }
            }
        }
        Err(anyhow!("Unknown output_type for {}: {}", name, runner.output_type))
    }

    pub async fn from_runner(name: &str, mut runner: Runner) -> Result<Self> {
        let log_processor = Self::get_processor(name, &mut runner).await?;
        Ok(Self {
            last_started: None,
            show_console_label: false,
            runner,
            log_processor,
            managed_process: None,
            running: false
        })
    }

    pub fn start(&mut self) -> Result<()> {
        self.running = true;
        match &self.managed_process {
            Some(_) => return Err(anyhow!("Process already started.")),
            None => {
                let mp = ManagedProcess::start(&self.runner.command, self.log_processor.clone())?;
                self.managed_process = Some(mp);
            }
        }
        Ok(())
    }

    pub async fn wait(&mut self) -> Result<ExitStatus> {
        let result = Ok(match &mut self.managed_process {
            None => return Err(anyhow!("Process not started")),
            Some(mp) => mp.wait().await?
        });
        self.managed_process = None;
        self.running = false;
        result
    }

    pub async fn run(&mut self) -> Result<ExitStatus> {
        let mut exit_status;
        loop {
            self.start()?;
            exit_status = self.wait().await?;
            if !exit_status.success() {
                let delay = self.runner.failure_restart_delay.unwrap_or(1);
                sleep(Duration::from_secs(delay.into())).await;
            }
        }
        return Ok(exit_status);
    }

    pub fn running(&self) -> bool {
        self.running
    }
}