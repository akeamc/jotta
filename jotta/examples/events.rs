use std::env;

use futures::StreamExt;
use jotta::{auth::LegacyAuth, events, Client};

#[tokio::main]
async fn main() -> Result<(), jotta::Error> {
    tracing_subscriber::fmt::init();

    let username = env::var("USERNAME").unwrap();
    let password = env::var("PASSWORD").unwrap();

    let fs = Client::new(LegacyAuth::init(username, &password).await?);

    let mut events = events::subscribe(&fs).await?;

    while let Some(ev) = events.next().await {
        match ev {
            Ok(ev) => println!("{:?}", ev),
            Err(err) => eprintln!("{}", err),
        }
    }

    Ok(())
}
