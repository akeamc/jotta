use std::{env, str::FromStr};

use futures_util::StreamExt;
// use tokio::io::{AsyncReadExt};
use jottacloud::{fs::Fs, AccessToken, Path};
use tokio::{fs::File, io::AsyncWriteExt};

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt::init();

    let token = AccessToken::new(
        env::var("JOTTACLOUD_ACCESS_TOKEN").expect("JOTTACLOUD_ACCESS_TOKEN not set"),
    );
    let fs = Fs::new(token);

    let mut file = File::create("example").await.unwrap();

    let mut stream = fs
        .open(&Path::from_str("Jotta/Archive/s3-test/rand").unwrap(), ..)
        .await
        .unwrap();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.unwrap();
        file.write_all(&chunk).await.unwrap();
    }
}
