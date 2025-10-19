pub mod simple;
#[tokio::main]
async fn main() {
    let mut range_server = simple::start_temp_server().await;
    eprintln!("Test file availble on url: {}", range_server.tmpfile_url);
    eprintln!(
        "Expected sha256sum: {}",
        hex::encode(&range_server.tmpfile_sha256)
    );

    range_server.join_handle.take().unwrap().await.unwrap();
}
