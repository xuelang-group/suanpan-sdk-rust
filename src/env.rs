use serde::Deserialize;
use std::sync::Once;

static INIT: Once = Once::new();
static mut E: Option<Env> = None;

#[derive(Deserialize)]
pub struct Env {
    /* used for suanpan-master-pod */
    #[serde(default = "default_sp_entry")]
    pub config_spentry: String,
    #[serde(default)]
    pub config_node_config: String,
    #[serde(default)]
    pub config_sp_node_id: String,
    #[serde(default = "default_sp_node_group")]
    pub config_sp_node_group: String,
    #[serde(default)]
    pub config_sp_debug: String,
    #[serde(default)]
    pub config_sp_host: String,
    #[serde(default = "default_sp_host_tls")]
    pub config_sp_host_tls: String,
    #[serde(default = "default_sp_os")]
    pub config_sp_os: String,
    #[serde(default = "default_sp_port")]
    pub config_sp_port: String,
    #[serde(default)]
    pub config_sp_user_id: String,
    #[serde(default)]
    pub config_sp_app_id: String,
    #[serde(default = "default_sp_user_id_header_field")]
    pub config_sp_user_id_header_field: String,
    #[serde(default = "default_sp_user_signature_header_field")]
    pub config_sp_user_signature_header_field: String,
    #[serde(default = "default_sp_user_sign_version_header_field")]
    pub config_sp_user_sign_version_header_field: String,
    #[serde(default)]
    pub config_sp_logkit_uri: String,
    #[serde(default = "default_sp_logkit_namespace")]
    pub config_sp_logkit_namespace: String,
    #[serde(default)]
    pub config_sp_logkit_path: String,
    #[serde(default = "default_sp_logkit_events_append")]
    pub config_sp_logkit_events_append: String,
    #[serde(default = "default_sp_logkit_logs_level")]
    pub config_sp_logkit_logs_level: String,
    #[serde(default)]
    pub config_sp_osslog_ext: String,
    #[serde(default = "default_k8s_pod_name")]
    pub config_hostname: String,
    /* used for suanpan-master-pod */

    /* used for suanpan-pod */
    #[serde(default)]
    pub node_config: String,
    #[serde(default)]
    pub sp_param: String,
    #[serde(default = "default_sp_node_group")]
    pub sp_node_group: String,
    #[serde(default)]
    pub sp_node_id: String,
    #[serde(default)]
    pub sp_app_id: String,
    #[serde(default)]
    pub sp_user_id: String,
    /* used for suanpan-pod */
    //-----------------dev mode
    #[cfg(feature = "dev-mode")]
    #[serde(default)]
    pub debug_graphraw: String,

    #[cfg(feature = "dev-mode")]
    #[serde(default)]
    pub debug_redis_uri: String,
}

fn default_sp_entry() -> String {
    "connect".to_string()
}

fn default_sp_node_group() -> String {
    "default".to_string()
}

fn default_sp_host_tls() -> String {
    "false".to_string()
}

fn default_sp_os() -> String {
    "kubernetes".to_string()
}

fn default_sp_port() -> String {
    "7000".to_string()
}

fn default_sp_user_id_header_field() -> String {
    "x-sp-user-id".to_string()
}

fn default_sp_user_signature_header_field() -> String {
    "x-sp-signature".to_string()
}

fn default_sp_user_sign_version_header_field() -> String {
    "x-sp-sign-version".to_string()
}

fn default_sp_logkit_namespace() -> String {
    "/logkit".to_string()
}

fn default_sp_logkit_events_append() -> String {
    "append".to_string()
}

fn default_sp_logkit_logs_level() -> String {
    "info".to_string()
}

fn default_k8s_pod_name() -> String {
    "nok8spodlog".to_string()
}

pub fn get_env() -> &'static Env {
    unsafe {
        INIT.call_once(|| {
            E = Some(build_env());
        });
        E.as_ref().unwrap()
    }
}

fn value_to_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Number(num) => Some(num.to_string()),
        serde_json::Value::String(s) => Some(s.clone()),
        _ => None, // or handle other types as needed
    }
}

pub fn get_node_config(key: &str) -> Option<String> {
    let env = get_env();
    let v: serde_json::Value = match serde_json::from_str(env.node_config.as_str()) {
        Ok(val) => val,
        Err(_) => {
            log::warn!("no not have node_config in env");
            return None;
        }
    };

    let keys: Vec<&str> = key.split('.').collect();
    let mut current_val = &v;

    for key in keys {
        current_val = match current_val.get(key) {
            Some(val) => val,
            None => {
                log::warn!("there is no key:{} in node_config", key);
                return None;
            }
        };
    }

    value_to_string(current_val)
}

pub fn get_sp_param(key: &str) -> Option<String> {
    let env = get_env();
    //base 64 decode for env.sp_param
    let param_str = match crate::utils::b64::decode_b64(&env.sp_param) {
        Ok(p) => p,
        Err(e) => {
            log::error!("decode sp_param error: {}", e);
            return None;
        }
    };
    let args = crate::utils::argv::get_args(&param_str);
    let key_with_prefix = format!("--{}", key);
    args.get(&key_with_prefix).cloned()
}

fn build_env() -> Env {
    envy::from_env::<Env>().unwrap()
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    struct Defer<F: FnOnce()> {
        f: Option<F>,
    }

    impl<F: FnOnce()> Defer<F> {
        fn new(f: F) -> Self {
            Defer { f: Some(f) }
        }
    }

    impl<F: FnOnce()> Drop for Defer<F> {
        fn drop(&mut self) {
            if let Some(f) = self.f.take() {
                f();
            }
        }
    }

    fn set_env() {
        // Set an environment variable using std::env
        env::set_var("CONFIG_SPENTRY", "test_value");
        env::set_var("SP_PARAM", r#"LS1hIGFhIC0tYiBiYg=="#);
        env::set_var(
            "NODE_CONFIG",
            r#"{"a":{"b":{"c":"value1", "d":2}},"a2":{"b2":{"c2":"value2"}}}"#,
        )
    }

    #[test]
    fn test_env_reading() {
        set_env();
        // This will call the provided closure when `_defer` goes out of scope.
        let _defer = Defer::new(|| {
            env::remove_var("CONFIG_SPENTRY");
        });
        // Assert that the environment variable was set correctly
        assert_eq!(env::var("CONFIG_SPENTRY").unwrap(), "test_value");

        // Read the environment variable using the Env struct
        let configuration = get_env();

        // Assert that the value was correctly read
        assert_eq!(configuration.config_spentry, "test_value");
        // When the test function exits, `_defer` will go out of scope,
        // and the closure provided to `Defer::new` will be executed,
        // effectively removing the environment variable.
    }

    #[test]
    fn test_get_sp_param() {
        set_env();
        // This will call the provided closure when `_defer` goes out of scope.
        let _defer = Defer::new(|| {
            env::remove_var("SP_PARAM");
        });

        assert_eq!(get_sp_param("a").unwrap(), "aa");
        assert_eq!(get_sp_param("b").unwrap(), "bb");
        assert_eq!(get_sp_param("c").is_none(), true);
    }

    #[test]
    fn test_get_node_config() {
        set_env();
        // This will call the provided closure when `_defer` goes out of scope.
        let _defer = Defer::new(|| {
            env::remove_var("NODE_CONFIG");
        });

        assert_eq!(get_node_config("a.b.c").unwrap(), "value1");
        assert_eq!(get_node_config("a.b.d").unwrap(), "2");
        assert_eq!(get_node_config("a.b").is_none(), true);
        assert_eq!(get_node_config("a5.b").is_none(), true);
        assert_eq!(get_node_config("a2.b2.c2").unwrap(), "value2");
    }
}
