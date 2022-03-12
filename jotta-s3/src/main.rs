use std::{env, str::FromStr};

use jotta::{
    auth::{provider, TokenStore},
    fs::Fs,
    path::AbsolutePath,
};

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt::init();

    let refresh_token = env::var("REFRESH_TOKEN").expect("REFRESH_TOKEN not set");
    let session_id = env::var("SESSION_ID").expect("SESSION_ID not set");

    let store = TokenStore::<provider::Jottacloud>::new(refresh_token, session_id);

    let fs = Fs::new(store);

    // let mut file = File::open("rand").await.unwrap();
    // let total = file.metadata().await.unwrap().len();
    // let digest = md5::Digest(hex!("4fe28312fdea186737995086f4edd905"));

    // let res = fs
    //     .allocate(&AllocReq {
    //         path: &PathOnDevice::from_str("Archive/s3-test/rand70").unwrap(),
    //         bytes: total as _,
    //         md5: digest,
    //         conflict_handler: ConflictHandler::RejectConflicts,
    //         created: None,
    //         modified: None,
    //     })
    //     .await
    //     .unwrap();

    // dbg!(&res);

    // file.seek(SeekFrom::Start(res.resume_pos)).await.unwrap();

    // let file = BufReader::new(file);
    // let stream = ReaderStream::new(file);

    // let res = fs
    //     .put_data(
    //         &res.upload_url,
    //         Body::wrap_stream(stream),
    //         res.resume_pos..=total,
    //     )
    //     .await
    //     .unwrap();

    // dbg!(res);

    let files = fs
        .file_meta(&AbsolutePath::from_str("jotta/archive/ship.jpg").unwrap())
        .await
        .unwrap();

    dbg!(files);

    // let mut file = File::create("example").await.unwrap();

    // let mut stream = fs
    //     .open(
    //         &AbsolutePath::from_str("Jotta/Archive/s3-test/rand").unwrap(),
    //         ..,
    //     )
    //     .await
    //     .unwrap();

    // while let Some(chunk) = stream.next().await {
    //     let chunk = chunk.unwrap();
    //     file.write_all(&chunk).await.unwrap();
    // }
}
