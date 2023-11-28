use super::codetype::CodeType;
use crate::types::{SuanpanError, SuanpanResult};
use serde::Deserialize;
use std::collections::HashMap;

fn default_recv_msg_type() -> String {
    "async".to_string()
}

#[derive(Deserialize)]
pub struct GraphJson {
    pub data: GraphData,
}

impl GraphJson {
    pub fn new_graph_from_str(s: &str) -> SuanpanResult<GraphJson> {
        serde_json::from_str(s).map_err(|e| SuanpanError::from(("", format!("{:?}", e))))
    }
}

#[derive(Deserialize)]
pub struct GraphData {
    pub processes: HashMap<String, Process>,
    pub connections: Vec<Connection>,
}

#[derive(Deserialize)]
pub struct Process {
    pub metadata: ProcessMetadata,
}

#[derive(Deserialize)]
pub struct ProcessMetadata {
    #[serde(rename = "receieveMsgType", default = "default_recv_msg_type")]
    pub recv_msg_type: String,

    pub def: ProcessDef,
}

#[derive(Deserialize)]
pub struct ProcessDef {
    pub ports: Vec<Port>,

    #[serde(rename = "compositeAppId")]
    pub composite_appid: Option<String>,

    #[serde(rename = "type")]
    pub def_type: CodeType,
    #[serde(rename = "dashboardId")]
    pub dashboard_id: Option<String>,

    pub params: Option<Params>,
}

#[derive(Deserialize)]
pub struct Params {
    #[serde(rename = "nodeRunMode")]
    pub node_run_mode: Option<NodeRunMode>,
}

#[derive(Deserialize)]
pub struct NodeRunMode {
    pub value: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct Port {
    pub uuid: String,
    #[serde(rename = "type")]
    pub port_type: String,
    #[serde(rename = "subType")]
    pub sub_type: String,
}

#[derive(Deserialize)]
pub struct Connection {
    pub src: Endpoint,
    pub tgt: Endpoint,
}

#[derive(Deserialize)]
pub struct Endpoint {
    pub process: String,
    pub port: String,
}

impl Process {
    pub fn is_single_node(&self) -> bool {
        self.metadata
            .def
            .params
            .as_ref()
            .and_then(|p| p.node_run_mode.as_ref())
            .and_then(|n| n.value.as_ref())
            .map_or(false, |v| v == "singleInstanceNodeService")
    }

    pub fn get_dashboard_id(&self) -> String {
        self.metadata.def.dashboard_id.clone().unwrap_or_default()
    }

    pub fn get_composite_id(&self) -> String {
        self.metadata
            .def
            .composite_appid
            .clone()
            .unwrap_or_default()
    }

    pub fn get_process_codetype(&self) -> CodeType {
        self.metadata.def.def_type
    }
}
