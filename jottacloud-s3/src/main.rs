use std::env;

use jottacloud::{jfs::ls, AccessToken};
use surf::Client;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt::init();

    let client = Client::new().with(surf::middleware::Logger::default());
    let token = AccessToken::new(
        env::var("JOTTACLOUD_ACCESS_TOKEN").expect("JOTTACLOUD_ACCESS_TOKEN not set"),
    );

    // let user_info = user_info(&client, &token).await.unwrap();

    // dbg!(user_info);

    let items = ls(&client, &token, "Jotta/Archive").await.unwrap();

    dbg!(items);

    // let files = list(&client, &token).await.unwrap();

    // dbg!(files);

    // let req = AllocReq {
    //     path: FilePath("/archive/s3-test/helloworld4.txt".into()),
    //     bytes: 4,
    //     md5: md5::compute(b"bruh"),
    //     conflict_handler: ConflictHandler::RejectConflicts,
    //     created: None,
    //     modified: None,
    // };

    // let alloc_res = allocate(&client, &token, &req).await.unwrap();

    // dbg!(&alloc_res);

    // let upload_res = upload(&client, &token, alloc_res.upload_url).await.unwrap();

    // dbg!(upload_res);
}
