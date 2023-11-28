use std::sync::atomic::AtomicU32;

use super::types::{QueueMessageData, QueueMessageRaw, SuanpanStreamSendData};
use crate::types::SuanpanResult;
use log::{debug, info};
use redis::aio::MultiplexedConnection;
use redis::streams::{StreamMaxlen, StreamReadReply};
use redis::{AsyncCommands, Client, Commands, RedisWrite};
use tokio_stream::StreamExt;

const DEFAULT_MAX_LEN: usize = 2000;
//const DEFAULT_BLOCK: usize = 6000;

//used for send to redis
//use &Value to avoid copy data make SuanpanSTreamSendData lifetime
struct RedisMutiplexedConnectionPool {
    connections: Vec<MultiplexedConnection>,
    next_index: AtomicU32,
    single_conn: MultiplexedConnection,
}

impl RedisMutiplexedConnectionPool {
    pub async fn async_init(client: &redis::Client, pool_size: usize) -> Self {
        if pool_size == 0 {
            panic!("RedisMutiplexedConnectionPool init pool_size is 0");
        }

        info!(
            "RedisMutiplexedConnectionPool init, pool_size:{}",
            pool_size
        );
        let mut connections = Vec::with_capacity(pool_size);

        for _ in 0..pool_size {
            let connection = client
                .get_multiplexed_tokio_connection()
                .await
                .expect("Failed to create connection");
            connections.push(connection);
        }

        Self {
            connections,
            next_index: AtomicU32::new(0),
            single_conn: client
                .get_multiplexed_tokio_connection()
                .await
                .expect("Failed to create connection"),
        }
    }

    //get async connection from pool with round-robin to avoid every mutiplexec connection block
    pub fn get_async_connection_from_pool(&self) -> MultiplexedConnection {
        let start = std::time::Instant::now();

        let index = self
            .next_index
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let con = self.connections[index as usize % self.connections.len()].clone();

        if start.elapsed().as_millis() > 10 {
            log::info!("!! Elapsed over 10ms, {:?}", start.elapsed());
        }

        con
    }

    pub fn get_single_conn(&self) -> MultiplexedConnection {
        self.single_conn.clone()
    }
}

pub struct RedisSubscriber {
    max_len: usize,
    client: Client,
    group: String,
    consumer: String,
    xread_options: redis::streams::StreamReadOptions,
    async_mulitplexconn_pool: Option<RedisMutiplexedConnectionPool>,
}

impl RedisSubscriber {
    pub fn new(uri: &str, group: &str, consumer: &str, recv_count: usize) -> SuanpanResult<Self> {
        let client = Client::open(uri)?;
        let xread_read_option = redis::streams::StreamReadOptions::default()
            .group(group, consumer)
            .block(0)
            .count(recv_count);
        info!(
            "RedisSubscriber init, uri:{}, block_count:{}",
            uri, recv_count
        );

        Ok(RedisSubscriber {
            client,
            max_len: DEFAULT_MAX_LEN,
            xread_options: xread_read_option,
            group: group.to_string(),
            consumer: consumer.to_string(),
            async_mulitplexconn_pool: None,
        })
    }

    pub fn init_queue(&self, queue: &str) {
        //start from consume id 0
        let created: SuanpanResult<()> = self.create_queue(&queue, &self.group, "0");
        if let Err(e) = created {
            debug!("Group already exists: {e:?}");
        }
    }

    pub async fn async_init(&mut self, pool_size: usize) {
        //init connection pool
        self.async_mulitplexconn_pool =
            Some(RedisMutiplexedConnectionPool::async_init(&self.client, pool_size).await);
    }

    pub fn get_group(&self) -> &str {
        &self.group
    }

    pub fn get_consumer(&self) -> &str {
        &self.consumer
    }

    //used for test do not remove
    pub async fn flushall(&self) -> SuanpanResult<()> {
        let mut con = self.client.get_async_connection().await?;
        let cmd = redis::cmd("FLUSHALL");
        cmd.query_async(&mut con).await?;
        Ok(())
    }

    fn recv_messages(&self, queue: &str) -> SuanpanResult<Vec<QueueMessageRaw>> {
        let mut con = self.client.get_connection()?;

        let results: StreamReadReply = con.xread_options(&[queue], &[">"], &self.xread_options)?;

        let mut messages = Vec::new();

        for stream_key in results.keys {
            for stream_id in stream_key.ids {
                debug!(
                    "redis-queue msg received: stream-id:{} queue:{}",
                    stream_id.id, queue
                );
                messages.push(QueueMessageRaw {
                    id: stream_id.id.clone(),
                    data: QueueMessageData { data: stream_id },
                });
            }
        }

        //TODO: test this logic
        //heng: how to lost message from redis? how can test this？
        //let lost_messages: Vec<_> = messages
        //    .iter()
        //    .filter(|m| !m.id.is_empty() && m.data.data.map.is_empty())
        //    .collect();

        //if !lost_messages.is_empty() {
        //    let lost_message_ids: Vec<&str> = lost_messages.iter().map(|m| m.id.as_str()).collect();
        //    if lost_message_ids.len() > 0 {
        //        let _: () = con.xack(queue, group, &lost_message_ids).await?;
        //    }
        //    warn!("Messages have lost: {:?}", lost_message_ids);
        //}

        Ok(messages)
    }

    pub async fn subscribe(&self, channel: String) -> SuanpanResult<String> {
        let mut pubsub_conn = self.client.get_async_connection().await?.into_pubsub();

        pubsub_conn.subscribe(channel.as_str()).await?;
        let mut pubsub_stream = pubsub_conn.on_message();
        let pubsub_msg = pubsub_stream.next().await.unwrap().get_payload()?;
        Ok(pubsub_msg)
    }

    fn create_queue(&self, queue: &str, group: &str, consume_id: &str) -> SuanpanResult<()> {
        let mut con = self
            .client
            .get_connection()
            .expect("Failed to get connection");

        info!(
            "Create queue queue:{} group:{}, conusmeid:{}",
            queue, group, consume_id
        );

        con.xgroup_create_mkstream(queue, group, consume_id)?;
        Ok(())
    }

    pub async fn recv_messages_async(&self, queue: &str) -> SuanpanResult<Vec<QueueMessageRaw>> {
        let mut con = self.get_async_recv_conn();

        let results: StreamReadReply = con
            .xread_options(&[queue], &[">"], &self.xread_options)
            .await?;

        let mut messages = Vec::new();

        for stream_key in results.keys {
            for stream_id in stream_key.ids {
                debug!(
                    "redis-queue msg received: stream-id:{} queue:{}",
                    stream_id.id, queue
                );
                messages.push(QueueMessageRaw {
                    id: stream_id.id.clone(),
                    data: QueueMessageData { data: stream_id },
                });
            }
        }

        //TODO: test this logic
        //heng: how to lost message from redis? how can test this？
        //let lost_messages: Vec<_> = messages
        //    .iter()
        //    .filter(|m| !m.id.is_empty() && m.data.data.map.is_empty())
        //    .collect();

        //if !lost_messages.is_empty() {
        //    let lost_message_ids: Vec<&str> = lost_messages.iter().map(|m| m.id.as_str()).collect();
        //    if lost_message_ids.len() > 0 {
        //        let _: () = con.xack(queue, group, &lost_message_ids).await?;
        //    }
        //    warn!("Messages have lost: {:?}", lost_message_ids);
        //}

        Ok(messages)
    }

    pub fn subscribe_queue(&self, queue: &str) -> SuanpanResult<Vec<QueueMessageRaw>> {
        let msgs: Vec<QueueMessageRaw> = self.recv_messages(&queue)?;
        //info!("Messages received, len:{}", msgs.len());
        Ok(msgs)
    }

    pub async fn ack_message_async(&self, queue: &str, msg_ids: Vec<String>) -> SuanpanResult<()> {
        if msg_ids.len() > 0 {
            let mut con = self.get_async_recv_conn();
            // TODO: optimized to deal when message handlering
            //do not use async fn xack because it will cause context switch
            //let _: () = con.xack(queue, &self.group, &msg_ids).await?;
            let _: () = con.xack(queue, &self.group, &msg_ids).await?;
        }
        Ok(())
    }

    pub fn ack_message(&self, queue: &str, msg_ids: Vec<String>) -> SuanpanResult<()> {
        if msg_ids.len() > 0 {
            let mut con = self.client.get_connection()?;
            // TODO: optimized to deal when message handlering
            //do not use async fn xack because it will cause context switch
            //let _: () = con.xack(queue, &self.group, &msg_ids).await?;
            let _: () = con.xack(queue, &self.group, &msg_ids)?;
        }
        Ok(())
    }

    pub async fn subscribe_queue_async(&self, queue: &str) -> SuanpanResult<Vec<QueueMessageRaw>> {
        let msgs: Vec<QueueMessageRaw> = self.recv_messages_async(&queue).await?;
        //let msg_ids: Vec<&str> = msgs.iter().map(|m| m.id.as_str()).collect();

        //if msg_ids.len() > 0 {
        //    //do not use async fn xack because it will cause context switch
        //    //let _: () = con.xack(queue, &self.group, &msg_ids).await?;
        //    let _: () = con.xack(queue, &self.group, &msg_ids).unwrap();
        //}

        //if msg_ids.len() == 0 {
        //    debug!("No messages received");
        //}
        //info!("Messages received, len:{}", msgs.len());

        //Ok(Vec::new())
        Ok(msgs)
    }

    pub fn get_async_recv_conn(&self) -> MultiplexedConnection {
        self.async_mulitplexconn_pool
            .as_ref()
            .unwrap()
            .get_single_conn()
    }

    pub fn get_async_connection_from_async_mutplex_con(&self) -> MultiplexedConnection {
        let start = std::time::Instant::now(); // Start the stopwatch
        let con = self
            .async_mulitplexconn_pool
            .as_ref()
            .unwrap()
            .get_async_connection_from_pool();
        if start.elapsed().as_millis() > 10 {
            info!("!! elasped over 10ms, {:?}", start.elapsed());
        }
        con
    }

    pub async fn send_message_async(
        &self,
        queue: &str,
        stream_data: SuanpanStreamSendData<'_>,
    ) -> SuanpanResult<()> {
        //let mut con = self.client.get_async_connection().await?;
        let mut con = self.get_async_connection_from_async_mutplex_con();

        let mut cmd = redis::cmd("XADD");
        cmd.arg(queue);
        cmd.arg(StreamMaxlen::Approx(self.max_len));
        cmd.arg("*");
        ////test
        //let vec = vec![0u8; 10_000];
        //cmd.arg("request_id").write_arg("test_id".as_bytes());
        //cmd.arg("in1").write_arg(&vec);
        //append raw data to xadd
        for (k, redis_value) in stream_data.data.iter() {
            cmd.arg(k).write_arg(redis_value);
        }

        //extra data added to xadd
        for (k, v) in stream_data.extra_data.iter() {
            cmd.arg(k);
            v.iter().for_each(|val| {
                cmd.write_arg(val);
            });
        }
        cmd.query_async(&mut con).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use redis::streams::StreamId;
    use redis::Value;
    use std::collections::HashMap;

    #[test]
    fn test_update_by_filter() {
        // Prepare the QueueMessageData
        let mut test_map = HashMap::new();
        test_map.insert("a".to_string(), Value::Data(vec![1]));
        test_map.insert("b".to_string(), Value::Data(vec![2]));
        test_map.insert("c".to_string(), Value::Data(vec![3]));
        test_map.insert("out1".to_string(), Value::Data(vec![4]));
        test_map.insert("out2".to_string(), Value::Data(vec![5]));
        test_map.insert("out3".to_string(), Value::Data(vec![6]));

        let queue_msg = QueueMessageData {
            data: StreamId {
                id: "test_id".to_string(),
                map: test_map,
            },
        };

        // Prepare the SuanpanStreamSendData
        let mut ssd = SuanpanStreamSendData::new();

        // Use the function
        ssd.update_by_queue_message_by_filter(&queue_msg, |key| !key.starts_with("out"));

        // Validate
        assert_eq!(ssd.data.get("a"), Some(&vec![1].as_ref()));
        assert_eq!(ssd.data.get("b"), Some(&vec![2].as_ref()));
        assert_eq!(ssd.data.get("c"), Some(&vec![3].as_ref()));
        assert_eq!(ssd.data.get("out1"), None);
        assert_eq!(ssd.data.get("out2"), None);
        assert_eq!(ssd.data.get("out3"), None);
    }

    #[test]
    fn test_insert_data_with_specific_key() {
        // Prepare the QueueMessageData with a specific key and value
        let mut test_map = HashMap::new();
        test_map.insert("oldKey".to_string(), Value::Data(vec![1, 2, 3]));

        let queue_msg = QueueMessageData {
            data: StreamId {
                id: "test_id".to_string(),
                map: test_map,
            },
        };

        // Prepare the SuanpanStreamSendData
        let mut ssd = SuanpanStreamSendData::new();

        // Use the function
        let result = ssd.insert_data_from_queue_with_specific_key(
            &"oldKey".to_string(),
            &"newKey".to_string(),
            &queue_msg,
        );

        // Check for success
        assert!(result.is_ok());
        assert_eq!(ssd.data.get("newKey"), Some(&vec![1, 2, 3].as_ref()));
        assert_eq!(ssd.data.get("oldKey"), None);
    }

    #[test]
    fn test_insert_data_with_missing_key() {
        // Prepare the QueueMessageData without the "oldKey"
        let queue_msg = QueueMessageData {
            data: StreamId {
                id: "test_id".to_string(),
                map: HashMap::new(),
            },
        };

        // Prepare the SuanpanStreamSendData
        let mut ssd = SuanpanStreamSendData::new();

        // Use the function
        let result = ssd.insert_data_from_queue_with_specific_key(
            &"oldKey".to_string(),
            &"newKey".to_string(),
            &queue_msg,
        );

        // Check for error
        assert!(result.is_err());
        let _ = result.unwrap_err();
        // Here, you might want to further check the error message or type
    }
}
