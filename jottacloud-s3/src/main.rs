use std::env;

use jottacloud::{
    files::{allocate, upload, AllocReq, ConflictHandler},
    AccessToken,
};
use surf::Client;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt::init();

    let req = AllocReq {
        path: "/archive/helloworld2.txt".into(),
        bytes: 4,
        md5: md5::compute(b"bruh"),
        conflict_handler: ConflictHandler::RejectConflicts,
    };

    let client = Client::new().with(surf::middleware::Logger::default());
    let token = AccessToken::new(
        env::var("JOTTACLOUD_ACCESS_TOKEN").expect("JOTTACLOUD_ACCESS_TOKEN not set"),
    );

    // let alloc_res = allocate(&client, &token, &req).await.unwrap();

    // dbg!(&alloc_res);
    let upload_url = "bruh".into();

    let upload_res = upload(&client, &token, upload_url).await.unwrap();

    dbg!(upload_res);
}
