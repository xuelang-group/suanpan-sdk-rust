pub mod logkit_inner;

use crate::env::{get_env, get_node_config};
use crate::types::SuanpanResult;

use logkit_inner::{LogInfo, LogKitInner};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub static mut LOGKIT_INSTANCE: Option<Arc<LogKitInner>> = None;

#[macro_export]
macro_rules! logkit_init {
    ($rt:expr, $handler:expr) => {
        unsafe {
            let mut inner = $crate::logkit::logkit_inner::LogKitInner::new();
            let recv = inner.take_revicer();
            $crate::logkit::LOGKIT_INSTANCE = Some(std::sync::Arc::new(inner));
            $rt.spawn(async move {
                $crate::logkit::logkit_inner::LogKitInner::run_logkit_handler($handler, recv).await;
            });
        }
    };
}

#[macro_export]
macro_rules! logkit_debug {
    ($msg:expr) => {
        $crate::log_internal!(log::Level::Debug, $msg)
    };
}

#[macro_export]
macro_rules! logkit_info {
    ($msg:expr) => {
        $crate::log_internal!(log::Level::Info, $msg)
    };
}

#[macro_export]
macro_rules! logkit_warn {
    ($msg:expr) => {
        $crate::log_internal!(log::Level::Warn, $msg)
    };
}

#[macro_export]
macro_rules! logkit_error {
    ($msg:expr) => {
        $crate::log_internal!(log::Level::Error, $msg)
    };
}

#[macro_export]
macro_rules! log_internal {
    ($level:expr, $msg:expr) => {
        unsafe {
            if let Some(ref instance) = $crate::logkit::LOGKIT_INSTANCE {
                let _ = instance
                    .emit_log(
                        $level,
                        ::chrono::Utc::now(),
                        $msg,
                        $crate::env::get_env().sp_node_id.clone(),
                    )
                    .map_err(|e| {
                        log::error!("emit log error:{}", e);
                    });
            } else {
                panic!("LogKit is not initialized!");
            }
        }
    };
}

pub struct LogkitPostMaster {
    req_url: String,
}
/// master http post handler
#[derive(Serialize, Deserialize)]
struct LogKitPostMasterData {
    app: String,
    logs: serde_json::Value,
}

impl LogkitPostMaster {
    pub fn new() -> LogkitPostMaster {
        let env = get_env();
        let port = get_node_config("app.predict.port").unwrap();

        let url = format!(
            "http://{}.{}:{}/internal/logkit/append",
            crate::common::get_app_cs_name(&env.sp_app_id),
            crate::common::get_user_namespace(&env.sp_user_id),
            &port
        );
        log::debug!("logkit post master, active url:{}", url);
        LogkitPostMaster { req_url: url }
    }

    pub async fn log_kit_http_post_handler(&self, info: LogInfo) -> SuanpanResult<()> {
        let logs = [info]; // Clone because we need to own the value
        let json_raw = serde_json::to_vec(&logs)?;

        let env = crate::env::get_env();
        Self::post_logkit_data_by_http_master(&self.req_url, &env.sp_app_id, &json_raw).await
    }

    pub async fn post_logkit_data_by_http_master(
        url: &str,
        appid: &str,
        data: &[u8],
    ) -> SuanpanResult<()> {
        let client = Client::new();
        let log_kit_post_master_data = LogKitPostMasterData {
            app: appid.to_string(),
            logs: serde_json::from_slice(data)?,
        };

        let resp = client
            .post(url)
            .json(&log_kit_post_master_data)
            .send()
            .await?; //request status
                     //remove after debug
        log::debug!(
            "logkit post master, status:{}, appid: {}, url:{}, data:{:?}",
            resp.status(),
            appid,
            url,
            serde_json::json!(log_kit_post_master_data)
        );

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(format!("HTTP POST failed, status code: {}", resp.status()).into())
        }
    }
}
