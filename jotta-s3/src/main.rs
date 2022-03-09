use std::{env, slice::SliceIndex, str::FromStr};

use jotta::{
    auth::{provider, TokenStore},
    fs::Fs,
    Path,
};
use reqwest::Client;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt::init();

    let refresh_token = env::var("REFRESH_TOKEN").expect("REFRESH_TOKEN not set");
    let session_id = env::var("SESSION_ID").expect("SESSION_ID not set");

    let mut store = TokenStore::<provider::Jottacloud>::new(refresh_token, session_id);
    let client = Client::new();

    let access_token = store.get_access_token(&client).await.unwrap();

    let fs = Fs::new(access_token);

    // let user = get_user(&client, &access_token).await.unwrap();

    // dbg!(user);

    let res = fs
        .index(&Path::from_str("jotta/archive").unwrap())
        .await
        .unwrap();

    dbg!(res);

    // let mut file = File::open("rand").await.unwrap();
    // let total = file.metadata().await.unwrap().len() as usize;
    // let digest = md5::Digest(hex!("41e09dc705b115dc14d8b5606666a9c3"));

    // let res = fs
    //     .allocate(&AllocReq {
    //         path: &Path::from_str("Archive/s3-test/rand2").unwrap(),
    //         bytes: total,
    //         md5: digest,
    //         conflict_handler: ConflictHandler::RejectConflicts,
    //         created: None,
    //         modified: None,
    //     })
    //     .await
    //     .unwrap();

    // dbg!(&res);

    // file.seek(SeekFrom::Start(res.resume_pos as _))
    //     .await
    //     .unwrap();

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
