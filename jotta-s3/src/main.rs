use std::{env, io::SeekFrom, str::FromStr};

// use futures_util::{StreamExt};
use hex_literal::hex;
use jotta::{
    auth::{provider, TokenStore},
    files::{AllocReq, ConflictHandler},
    fs::Fs,
    Path,
};
use reqwest::{Body, Client};
use tokio::{fs::File, io::AsyncSeekExt};
use tokio_util::io::ReaderStream;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt::init();

    let refresh_token = env::var("REFRESH_TOKEN").expect("REFRESH_TOKEN not set");
    let session_id = env::var("SESSION_ID").expect("SESSION_ID not set");

    let mut store = TokenStore::<provider::Tele2>::new(refresh_token, session_id);

    let access_token = store.get_access_token(&Client::new()).await.unwrap();

    let fs = Fs::new(access_token);

    let mut file = File::open("rand").await.unwrap();
    let total = file.metadata().await.unwrap().len() as usize;
    let digest = md5::Digest(hex!("1a84e5236fff4791ed1b2bd76327cd15"));

    let res = fs
        .allocate(&AllocReq {
            path: &Path::from_str("Archive/s3-test/rand").unwrap(),
            bytes: total,
            md5: digest,
            conflict_handler: ConflictHandler::RejectConflicts,
            created: None,
            modified: None,
        })
        .await
        .unwrap();

    dbg!(&res);

    file.seek(SeekFrom::Start(res.resume_pos as _))
        .await
        .unwrap();

    let stream = ReaderStream::new(file);

    let res = fs
        .put_data(
            &res.upload_url,
            Body::wrap_stream(stream),
            res.resume_pos..=total,
        )
        .await
        .unwrap();

    dbg!(res);

    // let mut file = File::create("example").await.unwrap();

    // let mut stream = fs
    //     .open(&Path::from_str("Jotta/Archive/s3-test/rand").unwrap(), ..)
    //     .await
    //     .unwrap();

    // while let Some(chunk) = stream.next().await {
    //     let chunk = chunk.unwrap();
    //     file.write_all(&chunk).await.unwrap();
    // }
}
