use crate::types::{SuanpanError, SuanpanExtraData, SuanpanResult};
use redis::streams::StreamId;
use redis::{FromRedisValue, ToRedisArgs, Value};
use std::collections::HashMap;
use std::ops::Deref;
pub struct SuanpanStreamSendData<'a> {
    pub data: HashMap<String, &'a Vec<u8>>,
    pub extra_data: HashMap<String, Vec<Vec<u8>>>,
}

impl<'a> SuanpanStreamSendData<'a> {
    pub fn new() -> Self {
        SuanpanStreamSendData {
            data: HashMap::new(),
            extra_data: HashMap::new(),
        }
    }

    pub fn insert_data_from_queue_with_specific_key(
        &mut self,
        old_key: &String,
        new_key: &String,
        queue: &'a QueueMessageData,
    ) -> SuanpanResult<()> {
        if let Some(out_data) = queue.data.map.get(old_key) {
            let val_raw = match out_data {
                Value::Data(ref val) => val,
                _ => panic!("not support type"),
            };
            self.data.insert(new_key.clone(), val_raw);
            Ok(())
        } else {
            Err(SuanpanError::from((
                "err from key find",
                format!("key:{}", old_key),
            )))
        }
    }

    pub fn update_by_queue_message_by_filter<F>(
        &mut self,
        queue: &'a QueueMessageData,
        mut f: F,
    ) -> &Self
    where
        F: FnMut(&String) -> bool,
    {
        for (k, v) in queue.data.map.iter().filter(|&(k, _)| f(k)) {
            let val_raw = match v {
                Value::Data(ref val) => val,
                _ => panic!("not support type"),
            };
            self.data.insert(k.clone(), val_raw);
        }
        self
    }

    pub fn update_by_queue_message(&mut self, queue: &'a QueueMessageData) -> &Self {
        for (k, v) in queue.data.map.iter() {
            let val_raw = match v {
                Value::Data(ref val) => val,
                _ => panic!("not support type"),
            };
            self.data.insert(k.clone(), val_raw);
        }
        self
    }

    pub fn set_payload<T: ToRedisArgs>(&mut self, key: &str, value: T) {
        //remove from original data
        self.data.remove(key);
        //put into extra data
        self.extra_data
            .insert(key.to_string(), value.to_redis_args());
    }
}

pub struct QueueMessageRaw {
    pub id: String,
    pub data: QueueMessageData,
}

impl QueueMessageRaw {
    pub fn get_message_data(&self) -> &QueueMessageData {
        &self.data
    }

    pub fn take_message_data(self) -> QueueMessageData {
        self.data
    }

    //pub fn get_message_data_mut(&mut self) -> &mut QueueMessageData {
    //    &mut self.data
    //}
}
pub struct QueueMessageDataGuard {
    pub data: QueueMessageData,

    #[cfg(feature = "queue_msg_drop_notify")]
    pub notifier: flume::Sender<()>,
}

impl Drop for QueueMessageDataGuard {
    fn drop(&mut self) {
        //#[cfg(feature = "queue_msg_drop_notify")]
        //info!("QueueMessageDataGuard drop");
        //TODO: no use flume use crossbeam to get better performance
        #[cfg(feature = "queue_msg_drop_notify")]
        self.notifier
            .send(())
            .expect("flume unbound size is oversize need to increase!!"); //if send failed that's means recv bounded oversize need to increase
    }
}

impl Deref for QueueMessageDataGuard {
    type Target = QueueMessageData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

pub struct QueueMessageData {
    pub data: StreamId,
}

impl QueueMessageData {
    pub fn new() -> Self {
        QueueMessageData {
            data: StreamId::default(),
        }
    }

    pub fn get_out_keys(&self) -> Vec<&String> {
        self.get_keys_with_filter(|k| k.starts_with("out"))
    }

    fn get_keys_with_filter<F>(&self, filter: F) -> Vec<&String>
    where
        F: FnMut(&&String) -> bool,
    {
        self.data.map.keys().filter(filter).collect()
    }

    pub fn get_extra(&self, raw_str: &str) -> SuanpanResult<SuanpanExtraData> {
        serde_json::from_str::<SuanpanExtraData>(raw_str).map_err(SuanpanError::from)
    }

    pub fn get_node_id(&self) -> Option<String> {
        let node_id_queue = self.get_payload::<String>("node_id");
        if node_id_queue.is_err() {
            let extra = self.get_extra(&self.get_payload::<String>("extra").unwrap_or_default());

            match extra {
                Ok(extra) => extra.node_id,
                Err(_) => None,
            }
        } else {
            Some(node_id_queue.unwrap())
        }
    }

    pub fn take_payload_redis_value(&mut self, key: &str) -> Option<Value> {
        self.data.map.remove(key)
    }

    pub fn get_payload_redis_value(&self, key: &str) -> Option<&Value> {
        self.data.map.get(key)
    }

    pub fn get_payload<T: FromRedisValue>(&self, key: &str) -> SuanpanResult<T> {
        if let Some(value) = self.data.get(key) {
            Ok(value)
        } else {
            Err(SuanpanError::from("not found item in pyload"))
        }
    }
}
