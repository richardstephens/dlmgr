use crate::args::{Args, ServerType};
use range_server::zero_server::start_zero_server;
mod args;
use clap::Parser;
use range_server::simple::start_temp_server;

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let join_handle = match args.command {
        ServerType::Simple => {
            let mut range_server = start_temp_server().await;
            eprintln!("Test file available on url: {}", range_server.tmpfile_url);
            eprintln!(
                "Expected sha256sum: {}",
                hex::encode(&range_server.tmpfile_sha256)
            );

            range_server.join_handle.take().unwrap()
        }
        ServerType::ZeroServer(_) => {
            let mut zero_server = start_zero_server(100 * 1024 * 1024 * 1024).await;
            eprintln!("URL: {:?}", zero_server.url);
            zero_server.join_handle.take().unwrap()
        }
    };
    join_handle.await.unwrap();
}
