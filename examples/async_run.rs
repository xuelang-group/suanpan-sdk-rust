use suanpan::{
    app::{async_run, StreamData},
    types::SuanpanResult,
};

pub async fn handler(stream_data: StreamData) -> SuanpanResult<()> {
    println!("hello world");
    if let Ok(data) = stream_data.get_input_data(1) {
        println!("data:{}", data);
    }
    Ok(())
}

fn main() {
    async_run(handler);
}
