pub mod common;
pub mod env;
pub mod graph;
pub mod redis;
pub mod storage;
pub mod types;
pub mod utils;

pub mod app {
    use crate::env::get_sp_param;
    use crate::redis::mq::RedisSubscriber;
    use crate::redis::types::QueueMessageRaw;
    use crate::types::SuanpanResult;
    use std::future::Future;
    const REDIS_HOST_SPARAM_KEY: &'static str = "mq-redis-host";
    const REDIS_PORT_SPARAM_KEY: &'static str = "mq-redis-port";
    const SDK_RECV_QUEUE: &'static str = "stream-recv-queue";

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
    }

    fn prepare() -> (RedisSubscriber, tokio::runtime::Runtime) {
        let redis = {
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

        (redis, worker_runtime)
    }

    fn get_redis_recv_queue_name() -> String {
        let recv_queue_name = {
            let nodeid = crate::env::get_env().config_sp_node_id.clone();
            let appid = crate::env::get_env().config_sp_app_id.clone();
            let userid = crate::env::get_env().config_sp_user_id.clone();
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
        let (redis, worker_runtime) = prepare();
        let redis_multi = std::sync::Arc::new(redis);

        let recv_queue_name = get_redis_recv_queue_name();
        log::debug!("recv_queue_name:{}", recv_queue_name);

        worker_runtime.block_on(async move {
            loop {
                let msgs = redis_multi.subscribe_queue_async(&recv_queue_name).await;
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

    pub fn async_run<F, Fut>(aysnc_handler: F)
    where
        F: Fn(StreamData) -> Fut + Send,
        Fut: Future<Output = SuanpanResult<()>> + Send,
    {
        let (redis, worker_runtime) = prepare();
        let redis_multi = std::sync::Arc::new(redis);

        let recv_queue_name = get_redis_recv_queue_name();
        log::debug!("recv_queue_name:{}", recv_queue_name);

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
