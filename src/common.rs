pub fn get_master_queue_name(user_id: &str, app_id: &str) -> String {
    format!("mq-master-{}-{}", user_id, app_id)
}

pub fn get_controller_queue_name(user_id: &str, app_id: &str) -> String {
    format!("mq-controller-{}-{}", user_id, app_id)
}

pub fn get_node_queue_name(user_id: &str, app_id: &str, node_id: &str) -> String {
    format!("mq-master-{}-{}-{}", user_id, app_id, node_id)
}

pub fn get_app_cs_name(appid: &str) -> String {
    if appid.is_empty() {
        return "".to_string();
    }

    if appid.contains('.') {
        let parts: Vec<&str> = appid.split('.').collect();
        return get_app_cs_name(parts[0]) + "-" + parts[1];
    }

    if appid.contains('_') {
        return "cron-".to_string() + &appid.replace('_', "-");
    }

    "app-".to_string() + appid
}

pub fn get_user_namespace(user_id: &str) -> String {
    "user-".to_string() + user_id.replace('_', "-").as_str()
}
