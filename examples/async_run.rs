use suanpan::{
    app::{self, async_run, StreamData},
    types::SuanpanResult,
};

pub async fn handler(stream_data: StreamData) -> SuanpanResult<()> {
    println!("hello world");
    if let Ok(data) = stream_data.get_input_data(1) {
        println!("recv msg: {:?}", stream_data);
        let msgid = stream_data.get_msg_id().ok();
        if let Err(e) = app::async_send_to(1, "lala".to_string(), msgid).await {
            println!("error {e}");
        }
        println!("data:{}", data);
    }
    Ok(())
}

fn main() {
    env_logger::init();
    async_run(handler);
}
