use std::env;

use futures::StreamExt;
use jotta_fs::{auth::LegacyTokenStore, events, Fs};

#[tokio::main]
async fn main() -> Result<(), jotta_fs::Error> {
    tracing_subscriber::fmt::init();

    let username = env::var("USERNAME").unwrap();
    let password = env::var("PASSWORD").unwrap();

    let fs = Fs::new(LegacyTokenStore::try_from_username_password(username, &password).await?);

    let mut events = events::subscribe(&fs).await?;

    while let Some(ev) = events.next().await {
        match ev {
            Ok(ev) => println!("{:?}", ev),
            Err(err) => eprintln!("{}", err),
        }
    }

    Ok(())
}
