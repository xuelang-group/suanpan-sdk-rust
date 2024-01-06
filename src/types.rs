//pub type SuanpanError = Box<dyn std::error::Error>;
pub type SuanpanResult<T> = Result<T, SuanpanError>;
use serde::Deserialize;
use std::fmt;

pub struct SuanpanError {
    pub message: String,
}

#[cfg(feature = "storage")]
impl From<anyhow::Error> for SuanpanError {
    fn from(err: anyhow::Error) -> Self {
        SuanpanError {
            message: format!("anyhow Error: {}", err),
        }
    }
}

#[cfg(feature = "storage")]
impl From<aws_sdk_s3::Error> for SuanpanError {
    fn from(err: aws_sdk_s3::Error) -> Self {
        SuanpanError {
            message: format!("aws_sdk_s3 Error: {}", err),
        }
    }
}

impl From<&'static str> for SuanpanError {
    fn from(err: &'static str) -> Self {
        SuanpanError {
            message: err.to_string(),
        }
    }
}

#[cfg(feature = "logkit")]
impl From<reqwest::Error> for SuanpanError {
    fn from(err: reqwest::Error) -> Self {
        SuanpanError {
            message: err.to_string(),
        }
    }
}

impl From<String> for SuanpanError {
    fn from(err: String) -> Self {
        SuanpanError { message: err }
    }
}

impl From<(&'static str, String)> for SuanpanError {
    fn from((err, detail): (&'static str, String)) -> Self {
        SuanpanError {
            message: format!("{}: detail:{}", err, detail),
        }
    }
}

#[cfg(feature = "redis-stream")]
impl From<redis::RedisError> for SuanpanError {
    fn from(err: redis::RedisError) -> Self {
        SuanpanError {
            message: format!("Redis Error: {}", err),
        }
    }
}

impl From<serde_json::Error> for SuanpanError {
    fn from(err: serde_json::Error) -> Self {
        SuanpanError {
            message: format!("serde json decode Error: {}", err),
        }
    }
}

// 实现 Display trait
impl fmt::Display for SuanpanError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

// 自定义 Debug trait 的实现
impl fmt::Debug for SuanpanError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SuanpanError: {}", self.message)
    }
}

#[derive(Debug, Deserialize)]
pub struct SuanpanExtraData {
    pub node_id: Option<String>,
}

// 实现 Error trait
impl std::error::Error for SuanpanError {}
