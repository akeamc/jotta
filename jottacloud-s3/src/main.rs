use std::{env, str::FromStr};

use futures_util::StreamExt;
// use tokio::io::{AsyncReadExt};
use jottacloud::{auth::get_access_token, fs::Fs, Path};
use reqwest::Client;
use tokio::{fs::File, io::AsyncWriteExt};

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt::init();

    let refresh_token = env::var("REFRESH_TOKEN").expect("REFRESH_TOKEN not set");
    let session_id = env::var("SESSION_ID").expect("SESSION_ID not set");

    let access_token = get_access_token(
        &Client::new(),
        &refresh_token,
        "jottacloud", // other sites: "tele2.se"
        &session_id,
    )
    .await
    .unwrap();

    let fs = Fs::new(access_token);

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
