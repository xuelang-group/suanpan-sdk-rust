pub mod logkit_inner;

use crate::env::{get_env, get_node_config};
use crate::types::SuanpanResult;

use logkit_inner::{LogInfo, LogKitInner};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
// used by marco
#[allow(unused_imports)]
use log::Level;

//used by marco
#[allow(dead_code)]
static mut LOGKIT_INSTANCE: Option<Arc<LogKitInner>> = None;

#[macro_export]
macro_rules! logkit_init {
    ($rt:expr, $handler:expr) => {
        unsafe {
            let mut inner = LogKitInner::new();
            let recv = inner.take_revicer();
            LOGKIT_INSTANCE = Some(Arc::new(inner));
            $rt.spawn(async move {
                LogKitInner::run_logkit_handler($handler, recv).await;
            });
        }
    };
}

#[macro_export]
macro_rules! logkit_debug {
    ($msg:expr) => {
        log_internal!(Level::Debug, $msg)
    };
}

#[macro_export]
macro_rules! logkit_info {
    ($msg:expr) => {
        log_internal!(Level::Info, $msg)
    };
}

#[macro_export]
macro_rules! logkit_warn {
    ($msg:expr) => {
        log_internal!(Level::Warn, $msg)
    };
}

#[macro_export]
macro_rules! logkit_error {
    ($msg:expr) => {
        log_internal!(Level::Error, $msg)
    };
}

#[macro_export]
macro_rules! log_internal {
    ($level:expr, $msg:expr) => {
        unsafe {
            if let Some(ref instance) = LOGKIT_INSTANCE {
                let _ = instance
                    .emit_log($level, Utc::now(), $msg, get_env().sp_node_id.clone())
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
    appid: String,
    logs: serde_json::Value,
}

impl LogkitPostMaster {
    pub fn new() -> LogkitPostMaster {
        let env = get_env();
        let port = get_node_config("App.Predict.Port").unwrap();

        let url = format!(
            "http://{}.{}:{}",
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
            appid: appid.to_string(),
            logs: serde_json::from_slice(data)?,
        };

        let resp = client
            .post(url)
            .json(&log_kit_post_master_data)
            .send()
            .await?; //request status

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(format!("HTTP POST failed, status code: {}", resp.status()).into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::sync::Mutex;
    use tokio::runtime::Runtime;

    struct TestStruct {
        test_info: Mutex<Vec<String>>,
        test_level: Mutex<Vec<String>>,
        test_nodeid: Mutex<Vec<String>>,
    }

    impl TestStruct {
        async fn test_logkit_handler(&self, info: LogInfo) -> SuanpanResult<()> {
            self.test_info.lock().unwrap().push(info.title);
            self.test_level.lock().unwrap().push(info.level);
            self.test_nodeid.lock().unwrap().push(info.data.node);
            Ok(())
        }
    }

    #[test]
    fn test_logkit() {
        let ts = TestStruct {
            test_info: Mutex::new(vec![]),
            test_level: Mutex::new(vec![]),
            test_nodeid: Mutex::new(vec![]),
        };
        let rt = Runtime::new().unwrap();

        let ts = Arc::new(ts);
        let ts_res = ts.clone();

        logkit_init!(rt, |info| ts.test_logkit_handler(info));
        logkit_debug!("test logkit".into());
        logkit_error!("errorb".into());
        logkit_warn!("warnbbc".into());

        rt.block_on(async {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        });
        assert_eq!(ts_res.test_info.lock().unwrap().len(), 3);
        assert_eq!(
            ts_res.test_info.lock().unwrap()[0],
            "test logkit".to_string()
        );
        assert_eq!(ts_res.test_info.lock().unwrap()[2], "warnbbc".to_string());
        assert_eq!(ts_res.test_level.lock().unwrap()[1], "ERROR".to_string());
    }
}
