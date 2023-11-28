pub fn get_master_queue_name(user_id: &str, app_id: &str) -> String {
    format!("mq-master-{}-{}", user_id, app_id)
}

pub fn get_controller_queue_name(user_id: &str, app_id: &str) -> String {
    format!("mq-controller-{}-{}", user_id, app_id)
}

pub fn get_node_queue_name(user_id: &str, app_id: &str, node_id: &str) -> String {
    format!("mq-master-{}-{}-{}", user_id, app_id, node_id)
}
