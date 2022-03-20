//! A higher-level but still pretty low-level Jottacloud client with
//! basic filesystem capabilities.
use std::{fmt::Debug, ops::RangeInclusive};

use bytes::Bytes;
use futures::{Stream, TryStreamExt};

use once_cell::sync::Lazy;

use reqwest::{
    header::{self},
    Body, Client, IntoUrl, Method, RequestBuilder, Response, Url,
};
use tracing::{debug, instrument};

use crate::{
    api::{read_json, read_xml, Exception, MaybeUnknown, XmlErrorBody},
    auth::TokenStore,
    files::{AllocReq, AllocRes, CompleteUploadRes, IncompleteUploadRes, UploadRes},
    jfs::{FileDetail, FolderDetail},
    path::UserScopedPath,
    range::{ByteRange, OpenByteRange},
};

/// `User-Agent` used in all requests to Jottacloud.
pub static USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " ",
    env!("CARGO_PKG_REPOSITORY")
);

/// A Jottacloud "filesystem".
#[derive(Debug)]
pub struct Fs {
    client: Client,
    token_store: Box<dyn TokenStore>,
}

impl Fs {
    /// Create a new filesystem.
    ///
    /// # Panics
    ///
    /// Panics if the HTTP client fails to initialize.
    #[must_use]
    pub fn new<S: TokenStore + 'static>(token_store: S) -> Self {
        Self {
            client: Client::builder().user_agent(USER_AGENT).build().unwrap(),
            token_store: Box::new(token_store),
        }
    }

    async fn authed_req(&self, method: Method, url: impl IntoUrl) -> crate::Result<RequestBuilder> {
        let access_token = self.token_store.get_access_token(&self.client).await?;

        Ok(self.client.request(method, url).bearer_auth(access_token))
    }

    async fn jfs_req(
        &self,
        method: Method,
        path: &UserScopedPath,
    ) -> crate::Result<RequestBuilder> {
        static JFS_BASE: Lazy<Url> =
            Lazy::new(|| Url::parse("https://jfs.jottacloud.com/jfs/").unwrap());

        let access_token = self.token_store.get_access_token(&self.client).await?;

        let url = JFS_BASE
            .join(&format!("{}/", access_token.username()))?
            .join(path)?;

        Ok(self.client.request(method, url).bearer_auth(access_token))
    }

    async fn files_v1_req_builder(
        &self,
        method: Method,
        path: &str,
    ) -> crate::Result<RequestBuilder> {
        static FILES_V1_BASE: Lazy<Url> =
            Lazy::new(|| Url::parse("https://api.jottacloud.com/files/v1/").unwrap());

        let url = FILES_V1_BASE.join(path)?;

        self.authed_req(method, url).await
    }

    /// Allocate for uploading a new file or a new file revision.
    ///
    /// # Errors
    ///
    /// - network errors
    /// - authentication errors (invalid token)
    /// - jottacloud errors
    /// - too little space left? (not verified)
    pub async fn allocate(&self, req: &AllocReq<'_>) -> crate::Result<AllocRes> {
        let response = self
            .files_v1_req_builder(Method::POST, "allocate")
            .await?
            .json(req)
            .send()
            .await?;

        Ok(read_json(response).await??)
    }

    /// Upload some or all data. `upload_url` is acquired from [`Fs::allocate`].
    ///
    /// # Errors
    ///
    /// - invalid upload url
    /// - premature end of body (smaller `body` than `range`)
    /// - jottacloud erorr
    /// - network error
    pub async fn upload_range(
        &self,
        upload_url: &str,
        body: impl Into<Body>,
        range: RangeInclusive<u64>,
    ) -> crate::Result<UploadRes> {
        let res = self
            .authed_req(Method::POST, upload_url)
            .await?
            .body(body)
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .header(header::CONTENT_LENGTH, range.end() - range.start())
            .header(
                header::RANGE,
                format!("bytes={}-{}", range.start(), range.end()),
            )
            .send()
            .await?;

        let res = match read_json::<CompleteUploadRes>(res).await? {
            Ok(complete) => UploadRes::Complete(complete),
            Err(err) => match err.error_id {
                Some(MaybeUnknown::Known(Exception::IncompleteUploadOpenApiException)) => {
                    UploadRes::Incomplete(IncompleteUploadRes { range })
                }
                _ => return Err(err.into()),
            },
        };

        Ok(res)
    }

    /// List all files and folders at a path. Similar to the UNIX `fs` command.
    ///
    /// # Errors
    ///
    /// - network errors
    /// - jottacloud errors (including auth)
    /// - path doesn't exist
    pub async fn index(&self, path: &UserScopedPath) -> crate::Result<FolderDetail> {
        let res = self.jfs_req(Method::GET, path).await?.send().await?;

        read_xml(res).await
    }

    /// Get metadata associated with a file.
    ///
    /// # Errors
    ///
    /// - network errors
    /// - jottacloud errors
    /// - no such file
    pub async fn file_detail(&self, path: &UserScopedPath) -> crate::Result<FileDetail> {
        let res = self.jfs_req(Method::GET, path).await?.send().await?;

        read_xml(res).await
    }

    /// **Permanently** delete a folder. It must be a folder. It fails if you try to
    /// delete a single file.
    ///
    /// # Errors
    ///
    /// - your usual Jottacloud errors
    /// - trying to delete a file instead of a folder
    pub async fn delete_folder(&self, path: &UserScopedPath) -> crate::Result<FolderDetail> {
        let res = self
            .jfs_req(Method::POST, path)
            .await?
            // switching this to ?dlDir=true will move the folder to trash instead of irreversibly deleting
            .query(&[("rmDir", "true")])
            .send()
            .await?;

        read_xml(res).await
    }

    #[instrument(skip(self, range))]
    async fn file_bin(
        &self,
        path: &UserScopedPath,
        range: impl ByteRange,
    ) -> crate::Result<Response> {
        debug!("requesting file");

        let res = self
            .jfs_req(Method::GET, path)
            .await?
            .query(&[("mode", "bin")])
            .header(header::RANGE, range.to_http())
            .send()
            .await?;

        if !res.status().is_success() {
            let err_xml = res.text().await?;
            let err: XmlErrorBody = serde_xml_rs::from_str(&err_xml)?;
            return Err(err.into());
        }

        Ok(res)
    }

    /// Open a stream to a file.
    ///
    /// # Errors
    ///
    /// - file doesn't exist
    /// - range is larger than the file itself
    /// - network errors
    /// - jottacloud errors
    pub async fn file_to_stream(
        &self,
        path: &UserScopedPath,
        range: impl ByteRange,
    ) -> crate::Result<impl Stream<Item = crate::Result<Bytes>>> {
        let res = self.file_bin(path, range).await?;

        Ok(res.bytes_stream().map_err(Into::into))
    }

    /// Read a file as a string.
    ///
    /// # Errors
    ///
    /// - file doesn't exist
    /// - range is larger than the file itself
    /// - network errors
    /// - jottacloud errors
    pub async fn file_to_string(&self, path: &UserScopedPath) -> crate::Result<String> {
        let text = self
            .file_bin(path, OpenByteRange::full())
            .await?
            .text()
            .await?;

        Ok(text)
    }

    /// Read a file as bytes.
    ///
    /// # Errors
    ///
    /// - file doesn't exist
    /// - range is larger than the file itself
    /// - network errors
    /// - jottacloud errors
    pub async fn file_to_bytes(
        &self,
        path: &UserScopedPath,
        range: impl ByteRange,
    ) -> crate::Result<Bytes> {
        let res = self.file_bin(path, range).await?;

        Ok(res.bytes().await?)
    }
}
