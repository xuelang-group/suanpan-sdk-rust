use crate::types::{SuanpanError, SuanpanResult};
use chrono::{DateTime, Utc};
use log::Level;
use serde::{Deserialize, Serialize};
use std::future::Future;
use tokio::sync::mpsc::{self, Receiver, Sender};
// Define structs to handle log data
#[derive(Serialize, Deserialize, Debug)]
pub struct LogData {
    pub node: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LogInfo {
    pub title: String,
    pub level: String,
    pub time: String,
    pub data: LogData,
}

pub struct LogKitInner {
    sender: Sender<LogInfo>,
    reciver: Option<Receiver<LogInfo>>,
}

impl LogKitInner {
    pub fn new() -> Self {
        let (send, recv) = mpsc::channel(32);
        let logkit = LogKitInner {
            sender: send,
            reciver: Some(recv),
        };
        logkit
    }

    pub fn take_revicer(&mut self) -> Receiver<LogInfo> {
        self.reciver.take().unwrap()
    }

    pub async fn run_logkit_handler<F, Fut>(handler: F, mut r: Receiver<LogInfo>)
    where
        F: Fn(LogInfo) -> Fut,
        Fut: Future<Output = SuanpanResult<()>>,
    {
        log::info!("start logit handling thread");
        while let Some(log_info) = r.recv().await {
            // Handle the log info, e.g., sending it to a server
            //println!("Received log: {:?}", log_info);
            if let Err(e) = handler(log_info).await {
                log::error!("error from logkit handler, {}", e.to_string());
            }
        }
    }

    pub fn emit_log(
        &self,
        level: Level,
        time: DateTime<Utc>,
        title: String,
        node_id: String,
    ) -> SuanpanResult<()> {
        let level_str = match level {
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
            _ => {
                return Err(SuanpanError::from((
                    "",
                    format!("level {:?} not support", level),
                )))
            }
        };

        let log_info = LogInfo {
            title,
            level: level_str.to_string(),
            time: time.to_rfc3339(),
            data: LogData { node: node_id },
        };

        self.sender
            .try_send(log_info)
            .map_err(|e| SuanpanError::from(e.to_string()))
    }
}
