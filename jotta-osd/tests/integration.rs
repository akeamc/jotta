use std::sync::Arc;

use async_once::AsyncOnce;
use bytes::{BufMut, BytesMut};
use futures_util::StreamExt;
use jotta::{auth::LegacyAuth, path::UserScopedPath, range::ClosedByteRange, Fs};
use jotta_osd::{
    bucket::{self, Bucket},
    object::{self, meta::Patch},
    Config, Context,
};
use lazy_static::lazy_static;
use rand::{rngs::OsRng, RngCore};

lazy_static! {
    /// Use a lazily evaluated, thread-safe token store so we don't need
    /// to login for every test.
    static ref TOKEN_STORE: AsyncOnce<LegacyAuth> = AsyncOnce::new(async {
                println!("logging in ...");

                LegacyAuth::init(env("USERNAME"), &env("PASSWORD"))
                        .await
                        .unwrap()
    });
}

pub fn env(key: &str) -> String {
    dotenv::var(key).unwrap_or_else(|_| panic!("`{key}` is not defined"))
}

async fn test_context(test_id: &str) -> Context<LegacyAuth> {
    let token_store = (*TOKEN_STORE.get().await).clone();
    let fs = Fs::new(token_store);
    let root = format!("jotta-osd-test/{test_id}");

    let path = UserScopedPath(format!("Jotta/Archive/{root}"));

    match fs.remove_folder(&path).await {
        Ok(_) => println!("removed `{path}`"),
        Err(_) => println!("failed to remvoe `{path}` -- assuming that it never existed"),
    }

    Context::initialize(fs, Config::new(root)).await.unwrap()
}

#[tokio::test]
async fn create_bucket() {
    let ctx = test_context("create_bucket").await;

    assert!(bucket::list(&ctx).await.unwrap().is_empty());

    let name = "my-bucket".parse().unwrap();
    let bucket = bucket::create(&ctx, &name).await.unwrap();
    assert_eq!(bucket, Bucket { name });

    assert_eq!(bucket::list(&ctx).await.unwrap(), vec![bucket]);
}

#[tokio::test]
async fn simple_upload() {
    let ctx = test_context("simple_upload").await;

    let bucket = bucket::create(&ctx, &"can".parse().unwrap()).await.unwrap();

    let name = "random".parse().unwrap();
    object::create(&ctx, &bucket.name, &name, Patch::default())
        .await
        .unwrap();

    let filesize = 4_000_000;
    let mut data = BytesMut::new();
    data.resize(filesize, 0);
    OsRng.fill_bytes(&mut data[..]);

    object::upload_range(&ctx, &bucket.name, &name, 0, data.as_ref(), 2)
        .await
        .unwrap();

    let meta = object::meta::get(&ctx, &bucket.name, &name).await.unwrap();

    assert_eq!(meta.size, filesize as u64);

    let mut stream = object::stream_range(
        Arc::new(ctx),
        bucket.name,
        name,
        ClosedByteRange::new_to_including(filesize as u64 - 1),
        2,
    );

    let mut remote = BytesMut::with_capacity(filesize);

    while let Some(chunk) = stream.next().await {
        remote.put(chunk.unwrap());
    }

    if data != remote {
        panic!("uploaded file does not match local copy")
    }
}
