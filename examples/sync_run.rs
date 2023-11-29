use suanpan::{
    app::{run, StreamData},
    types::SuanpanResult,
};

pub fn handler(stream_data: StreamData) -> SuanpanResult<()> {
    println!("hello world");
    if let Ok(data) = stream_data.get_input_data(1) {
        println!("data:{}", data);
    }
    Ok(())
}

fn main() {
    env_logger::init();
    run(handler);
}
