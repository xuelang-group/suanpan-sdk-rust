pub mod common;
pub mod env;
#[cfg(feature = "graph")]
pub mod graph;
#[cfg(feature = "logkit")]
pub mod logkit;
#[cfg(feature = "redis-stream")]
pub mod redis;
#[cfg(feature = "storage")]
pub mod storage;
pub mod types;
pub mod utils;

#[cfg(feature = "redis-stream")]
pub mod app {
    use crate::env::get_sp_param;
    use crate::redis::{
        mq::RedisSubscriber,
        types::{QueueMessageRaw, SuanpanStreamSendData},
    };
    use crate::types::SuanpanResult;
    use std::future::Future;
    use std::sync::{Arc, Once};
    static INIT: Once = Once::new();
    static mut E: Option<(Arc<RedisSubscriber>, Arc<tokio::runtime::Runtime>)> = None;

    pub const REDIS_HOST_SPARAM_KEY: &'static str = "mq-redis-host";
    pub const REDIS_PORT_SPARAM_KEY: &'static str = "mq-redis-port";
    pub const SDK_RECV_QUEUE: &'static str = "stream-recv-queue";

    #[derive(Debug)]
    pub struct StreamData {
        msg_raw: QueueMessageRaw,
    }

    impl StreamData {
        pub fn new(raw: QueueMessageRaw) -> StreamData {
            StreamData { msg_raw: raw }
        }

        //TODO: add tyep support for stream data input
        //eg: string,csv, etc..
        pub fn get_input_data(&self, input_num: usize) -> SuanpanResult<String> {
            let payload_input_key = format!("in{}", input_num);
            self.msg_raw.data.get_payload(&payload_input_key)
        }

        pub fn get_msg_id(&self) -> SuanpanResult<String> {
            self.msg_raw.data.get_payload("id") //msg from upstram use id for request_id
        }
    }

    fn get_prepare() -> (
        std::sync::Arc<RedisSubscriber>,
        std::sync::Arc<tokio::runtime::Runtime>,
    ) {
        unsafe {
            INIT.call_once(|| {
                // Assuming prepare() is a function that returns a tuple (RedisSubscriber, Runtime)
                let (r, w) = prepare();
                E = Some((Arc::new(r), Arc::new(w)));
            });
            E.clone().unwrap()
        }
    }

    fn prepare() -> (RedisSubscriber, tokio::runtime::Runtime) {
        let mut redis = {
            let redis_uri = format!(
                "redis://{}:{}",
                get_sp_param(REDIS_HOST_SPARAM_KEY).expect("redis host not found"),
                get_sp_param(REDIS_PORT_SPARAM_KEY).expect("redis port not found"),
            );

            log::debug!(
                "prepare redis subscriber, uri:{} group:{}, consumer:{}",
                redis_uri,
                crate::env::get_env().sp_node_group,
                crate::env::get_env().sp_node_id,
            );

            let redis = crate::redis::mq::RedisSubscriber::new(
                &redis_uri,
                &crate::env::get_env().sp_node_group,
                &crate::env::get_env().sp_node_id,
                1,
            )
            .unwrap();
            redis
        };

        //tokio_runtime
        let worker_runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("worker-runtime")
            .build()
            .unwrap();

        worker_runtime.block_on(redis.async_init(1));
        (redis, worker_runtime)
    }

    pub fn get_right_panel_value(key: &str) -> Option<String> {
        get_sp_param(key)
    }

    fn get_redis_recv_queue_name() -> String {
        let recv_queue_name = {
            let nodeid = crate::env::get_env().sp_node_id.clone();
            let appid = crate::env::get_env().sp_app_id.clone();
            let userid = crate::env::get_env().sp_user_id.clone();
            if nodeid == "" || appid == "" || userid == "" {
                panic!(
                    "invalid parameter from env config, {} {} {}",
                    nodeid, appid, userid
                );
            }
            get_sp_param(SDK_RECV_QUEUE)
                .unwrap_or_else(|| crate::common::get_node_queue_name(&userid, &appid, &nodeid))
        };
        recv_queue_name
    }

    pub fn run<F>(handler: F)
    where
        F: Fn(StreamData) -> SuanpanResult<()>,
    {
        let (redis, worker_runtime) = get_prepare();
        let recv_queue_name = get_redis_recv_queue_name();
        redis.init_queue(&recv_queue_name);

        log::debug!("recv_queue_name:{}", recv_queue_name);

        worker_runtime.block_on(async move {
            loop {
                let msgs = redis.subscribe_queue_async(&recv_queue_name).await;
                match msgs {
                    Ok(msgs) => {
                        for msg in msgs {
                            if let Err(e) = handler(StreamData::new(msg)) {
                                log::error!("handler error:{}", e);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("recv msg error:{}", e);
                    }
                }
            }
        });
    }

    pub async fn async_send_to(
        port: usize,
        data: String,
        msg_id: Option<String>,
    ) -> SuanpanResult<()> {
        let send_queue_name = {
            let appid = crate::env::get_env().sp_app_id.clone();
            let userid = crate::env::get_env().sp_user_id.clone();
            if appid == "" || userid == "" {
                panic!("invalid parameter from env config, {} {} ", appid, userid);
            }
            crate::common::get_master_queue_name(&userid, &appid)
        };

        let mut suanpan_stream_data = SuanpanStreamSendData::new();
        let out_index = format!("out{}", port);
        if msg_id.is_some() {
            suanpan_stream_data.set_payload("request_id", msg_id.unwrap()); //msg to downstream use request_id to "master/approuter"
        }

        suanpan_stream_data.set_payload("node_id", crate::env::get_env().sp_node_id.clone());
        suanpan_stream_data.set_payload(&out_index, data);
        log::debug!("send data to {send_queue_name}, with out_idx:{out_index}");

        let (redis, _) = get_prepare();
        redis
            .send_message_async(&send_queue_name, suanpan_stream_data)
            .await
    }

    pub fn async_init<F, Fut>(init_func: F)
    where
        F: Fn() -> Fut + Send,
        Fut: Future<Output = ()> + Send,
    {
        let (_, worker_runtime) = get_prepare();
        worker_runtime.block_on(init_func());
    }

    pub fn async_run<F, Fut>(aysnc_handler: F)
    where
        F: Fn(StreamData) -> Fut + Send,
        Fut: Future<Output = SuanpanResult<()>> + Send,
    {
        let (redis, worker_runtime) = get_prepare();
        let recv_queue_name = get_redis_recv_queue_name();
        redis.init_queue(&recv_queue_name);
        let redis_multi = std::sync::Arc::new(redis);

        log::debug!("recv_queue_name:{}", recv_queue_name);

        //init

        worker_runtime.block_on(async move {
            loop {
                let msgs = redis_multi.subscribe_queue_async(&recv_queue_name).await;
                match msgs {
                    Ok(msgs) => {
                        for msg in msgs {
                            let msg = StreamData::new(msg);
                            if let Err(e) = aysnc_handler(msg).await {
                                log::error!("handler error:{}", e);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("recv msg error:{}", e);
                    }
                }
            }
        });
    }
}
