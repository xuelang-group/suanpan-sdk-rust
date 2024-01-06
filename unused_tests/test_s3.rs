#![cfg(feature = "storage")]
use suanpan::storage::s3::Client;
use tokio;

#[tokio::main]
async fn main() {
    //test_node_id_with_complax_extra().await;
    //test_multi_send_func().await;
    let s3_client = Client::new(
        "http://10.130.9.2:9000".into(),
        "public".into(),
        "".into(),
        "".into(),
    )
    .await
    .unwrap();
    test_list_objects(&s3_client).await;
    let start = std::time::Instant::now();
    recv_object(&s3_client).await;
    println!("recv_object cost: {:?}", start.elapsed());
}

async fn test_list_objects(c: &Client) {
    let res = c.list_objects("").await.unwrap();
    res.iter().for_each(|x| println!("{}", x));
}

async fn recv_object(c: &Client) {
    c.fget_object("mc.pptx", "./mc.pptx").await.unwrap();
}
