use std::env;

use std::str::FromStr;
use std::time::Instant;

use futures_util::StreamExt;
// use futures_util::StreamExt;
// use hex_literal::hex;
// use jotta_fs::{
//     auth::{provider, TokenStore},
//     files::{AllocReq, ConflictHandler},
//     path::{AbsolutePath, PathOnDevice},
//     Fs, OptionalByteRange,
// };
// use reqwest::Body;
// use tokio::{
//     fs::File,
//     io::{AsyncSeekExt, AsyncWriteExt, BufReader},
// };
// use tokio_util::io::ReaderStream;
use jotta::auth::{provider, TokenStore};

use jotta::object::{open_range, ObjectName};
use jotta::Fs;
use jotta::{Config, Context};

use tokio::fs::File;
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt::init();

    let refresh_token = env::var("REFRESH_TOKEN").expect("REFRESH_TOKEN not set");
    let session_id = env::var("SESSION_ID").expect("SESSION_ID not set");

    let store = TokenStore::<provider::Jottacloud>::new(refresh_token, session_id);

    let fs = Fs::new(store);
    let ctx = Context::new(fs, Config::new("s3-test"));

    let bucket = "bucket";
    let object_name = ObjectName::from_str("bbb.mp4").unwrap();

    // let res = create_object(&ctx, bucket, &object_name, Some("video/mp4".parse().unwrap())).await.unwrap();

    // dbg!(res);

    // let offset = 0;
    // // let total_bytes = 10_000_000;

    // // let file = File::open("/dev/urandom").await.unwrap().take(total_bytes);
    // let mut file = File::open("bbb.mp4").await.unwrap();
    // file.seek(SeekFrom::Start(offset)).await.unwrap();
    // // let file = file.take(total_bytes);
    // let file = BufReader::new(file);
    // // let stream = ReaderStream::new(file);

    // upload_range(&ctx, bucket, &object_name, offset, file)
    //     .await
    //     .unwrap();

    // dbg!(res);

    let mut file = File::create("bbb2.mp4").await.unwrap();

    let before = Instant::now();

    let (meta, mut stream) = open_range(&ctx, bucket, &object_name, 20).await.unwrap();

    dbg!(meta);

    let mut num_bytes = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.unwrap();
        num_bytes += chunk.len();
        file.write_all(&chunk).await.unwrap();
    }

    let elapsed = before.elapsed();

    let bps = (num_bytes as f32) / elapsed.as_secs_f32();

    println!(
        "downloaded {} bytes in {:.02?} ({}Mb/s)",
        num_bytes,
        elapsed,
        bps * 8.0 / 1000000.0
    )
}
