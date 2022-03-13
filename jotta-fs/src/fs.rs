//! A higher-level but still pretty low-level Jottacloud client with
//! basic filesystem capabilities.
use std::{
    fmt::{Debug, Display},
    ops::{RangeBounds, RangeInclusive},
};

use bytes::Bytes;
use futures::{Stream, TryStreamExt};

use once_cell::sync::Lazy;

use reqwest::{
    header::{self, HeaderValue},
    Body, Client, IntoUrl, Method, RequestBuilder, Url,
};
use tracing::{debug, instrument};

use crate::{
    api::{read_json, read_xml, Exception, MaybeUnknown, XmlErrorBody},
    auth::{self, TokenStore},
    files::{AllocReq, AllocRes, CompleteUploadRes, IncompleteUploadRes, UploadRes},
    jfs::{FileMeta, Index},
    path::UserScopedPath,
};

/// A Jottacloud "filesystem".
#[derive(Debug, Clone)]
pub struct Fs<P: auth::Provider> {
    client: Client,
    token_store: TokenStore<P>,
}

impl<P: auth::Provider> Fs<P> {
    /// Create a new filesystem.
    #[must_use]
    pub fn new(token_store: TokenStore<P>) -> Self {
        Self {
            client: Client::new(),
            token_store,
        }
    }

    async fn req_with_token(
        &self,
        method: Method,
        url: impl IntoUrl,
    ) -> crate::Result<RequestBuilder> {
        let access_token = self.token_store.get_access_token(&self.client).await?;

        Ok(self.client.request(method, url).bearer_auth(access_token))
    }

    async fn jfs_req(&self, method: Method, path: &str) -> crate::Result<RequestBuilder> {
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

        self.req_with_token(method, url).await
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
            .req_with_token(Method::POST, upload_url)
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

        // let pool = res.headers().get("pool");

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
    pub async fn index(&self, path: &UserScopedPath) -> crate::Result<Index> {
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
    pub async fn file_meta(&self, path: &UserScopedPath) -> crate::Result<FileMeta> {
        let res = self.jfs_req(Method::GET, path).await?.send().await?;

        read_xml(res).await
    }

    /// Open a stream to a file.
    ///
    /// # Errors
    ///
    /// - file doesn't exist
    /// - range is larger than the file itself
    /// - network errors
    /// - jottacloud errors
    #[instrument(skip(self, range))]
    pub async fn open(
        &self,
        path: &UserScopedPath,
        range: impl Into<OptionalByteRange>,
    ) -> crate::Result<impl Stream<Item = crate::Result<Bytes>>> {
        debug!("opening stream to file");

        let range: OptionalByteRange = range.into();

        let res = self
            .jfs_req(Method::GET, &format!("{}?mode=bin", path))
            .await?
            // status will be `206 Partial Content` even if the whole body is returned
            .header(header::RANGE, range)
            .send()
            .await?;

        if !res.status().is_success() {
            let err_xml = res.text().await?;
            let err: XmlErrorBody = serde_xml_rs::from_str(&err_xml)?;
            return Err(err.into());
        }

        Ok(res.bytes_stream().map_err(Into::into))
    }
}

/// Simplified abstraction of the `Range` header value in the sense that
/// only one range is allowed.
#[derive(Debug)]
pub struct OptionalByteRange {
    start: Option<u64>,
    end: Option<u64>,
}

#[allow(clippy::len_without_is_empty)]
impl OptionalByteRange {
    /// Total length of the range.
    #[must_use]
    pub fn len(&self) -> Option<u64> {
        self.end.map(|end| end + 1 - self.start.unwrap_or(0))
    }

    /// Start of the range.
    #[must_use]
    pub fn start(&self) -> u64 {
        self.start.unwrap_or(0)
    }

    /// End of the range.
    #[must_use]
    pub fn end(&self) -> Option<u64> {
        self.end
    }

    /// Construct a full range.
    ///
    /// ```
    /// use jotta_fs::OptionalByteRange;
    ///
    /// assert_eq!(OptionalByteRange::full().to_string(), "bytes=0-");
    /// ```
    #[must_use]
    pub fn full() -> Self {
        Self {
            start: None,
            end: None,
        }
    }

    /// Convert a standard [`std::ops::Range`] to [`OptionalByteRange`]:
    ///
    /// ```
    /// use jotta_fs::OptionalByteRange;
    ///
    /// assert_eq!(OptionalByteRange::try_from_bounds(..5).unwrap().to_string(), "bytes=0-4");
    /// assert_eq!(OptionalByteRange::try_from_bounds(..).unwrap().to_string(), "bytes=0-");
    /// assert_eq!(OptionalByteRange::try_from_bounds(3..=4).unwrap().to_string(), "bytes=3-4");
    /// assert!(OptionalByteRange::try_from_bounds(10..7).is_err()); // reversed
    /// ```
    ///
    /// # Errors
    ///
    /// This function returns an `Error` if the range is reversed, i.e. `start` comes after `end`.
    pub fn try_from_bounds(bounds: impl RangeBounds<u64>) -> Result<Self, InvalidRange> {
        use std::ops::Bound::{Excluded, Included, Unbounded};

        let start = match bounds.start_bound() {
            Included(i) => Some(*i),
            Excluded(e) => Some(e + 1),
            Unbounded => None,
        };

        let end = match bounds.end_bound() {
            Included(i) => Some(*i),
            Excluded(e) => Some(e - 1),
            Unbounded => None,
        };

        if start.unwrap_or(0) > end.unwrap_or(u64::MAX) {
            return Err(InvalidRange::Reversed);
        }

        Ok(Self { start, end })
    }
}

/// Invalid range error.
#[derive(Debug, thiserror::Error)]
pub enum InvalidRange {
    /// Range is reversed.
    #[error("range is wrong direction")]
    Reversed,
}

impl Display for OptionalByteRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bytes={}-", self.start())?;

        if let Some(end) = self.end() {
            write!(f, "{}", end)?;
        }

        Ok(())
    }
}

impl From<OptionalByteRange> for HeaderValue {
    fn from(value: OptionalByteRange) -> Self {
        HeaderValue::from_str(&value.to_string()).unwrap()
    }
}
