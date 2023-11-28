use std::sync::Arc;
use suanpan::redis::mq::RedisSubscriber;
use suanpan::redis::types::SuanpanStreamSendData;
use tokio;

#[tokio::main]
async fn main() {
    test_node_id_with_complax_extra().await;
    //test_multi_send_func().await;
}

#[allow(dead_code)]
async fn test_multi_send_func() {
    //let r = RedisSubscriber::new("redis://app-80544-redis:6379", "testgroup", "123").unwrap();
    let uri = "redis://10.244.1.45:6379";
    println!("{:?}", uri);
    let mut r = RedisSubscriber::new(uri, "testgroup", "123", 100).unwrap();
    r.async_init(3).await;
    let r = Arc::new(r);
    //r.flushall().await.unwrap();
    for _ in 0..100 {
        let mut join_h = Vec::new();
        for _ in 0..60 {
            let r = r.clone();
            let mut suanpan_stream_data = SuanpanStreamSendData::new();
            suanpan_stream_data.set_payload("node_id", "123");
            let data = vec![0u8; 10_000];
            suanpan_stream_data.set_payload("in1", data);
            let tastk = tokio::spawn(async move {
                let start = std::time::Instant::now(); // Start the stopwatch
                let _ = r
                    .send_message_async("test queue", suanpan_stream_data)
                    .await;
                if start.elapsed().as_millis() > 50 {
                    println!("!! elasped over 10ms, {:?}", start.elapsed());
                }
            });
            join_h.push(tastk);
        }
        for h in join_h.into_iter() {
            let _ = h.await;
        }
    }

    //r.flushall().await.unwrap();
}

#[allow(dead_code)]
async fn test_node_id_with_complax_extra() {
    let mut r = RedisSubscriber::new("redis://10.130.9.2:6379", "testgroup", "123", 100).unwrap();
    r.async_init(3).await;
    r.flushall().await.unwrap();
    r.init_queue("testqueue3");
    //let mut data = QueueMessageData::new();
    let mut suanpan_stream_data = SuanpanStreamSendData::new();
    suanpan_stream_data.set_payload("node_id", "123");
    r.send_message_async("testqueue3", suanpan_stream_data)
        .await
        .unwrap();

    let res = r.recv_messages_async("testqueue3").await.unwrap();
    let node_id = res.get(0).unwrap().get_message_data().get_node_id();
    assert_eq!(node_id, Some("123".to_string()));

    r.flushall().await.unwrap();
    r.init_queue("testqueue3");
    let mut data = SuanpanStreamSendData::new();
    data.set_payload("aaa", "123");
    r.send_message_async("testqueue3", data).await.unwrap();
    let res = r.recv_messages_async("testqueue3").await.unwrap();
    let node_id = res.get(0).unwrap().get_message_data().get_node_id();
    assert_eq!(node_id, None);

    r.flushall().await.unwrap();
    r.init_queue("testqueue3");
    data = SuanpanStreamSendData::new();
    data.set_payload("extra", r#"{"node_id": "456"}"#);
    r.send_message_async("testqueue3", data).await.unwrap();
    let res = r.recv_messages_async("testqueue3").await.unwrap();
    let node_id = res.get(0).unwrap().get_message_data().get_node_id();
    assert_eq!(node_id, Some("456".to_string()));
    r.flushall().await.unwrap();
}

#[allow(dead_code)]
async fn test_manual() {
    let r = RedisSubscriber::new("redis://10.130.9.2:6379", "testgroup", "123", 100).unwrap();
    r.flushall().await.unwrap();
    let res = r.recv_messages_async("testqueue").await.unwrap();

    for i in res {
        let mut data = SuanpanStreamSendData::new();
        data.update_by_queue_message(i.get_message_data());
        data.set_payload("key1", 1234);
        data.set_payload("key2", 2345);
        data.set_payload("key3", "abcd");
        r.send_message_async("testqueue1", data).await.unwrap();
    }
    r.flushall().await.unwrap();
}
