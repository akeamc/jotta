use std::{
    fmt::Display,
    ops::{RangeBounds, RangeInclusive},
};

use bytes::Bytes;
use futures::Stream;

use once_cell::sync::Lazy;

use reqwest::{
    header::{self, HeaderValue},
    Body, Client, IntoUrl, Method, RequestBuilder, Url,
};

use crate::{
    api::{read_json, read_xml, Exception, MaybeUnknown, XmlErrorBody},
    auth::AccessToken,
    files::{AllocReq, AllocRes, CompleteUploadRes, IncompleteUploadRes, UploadRes},
    jfs::{FileMeta, Index},
    Path,
};

#[derive(Debug)]
pub struct Fs {
    client: Client,
    access_token: AccessToken,
}

impl Fs {
    #[must_use]
    pub fn new(access_token: AccessToken) -> Self {
        Self {
            client: Client::new(),
            access_token,
        }
    }

    fn req_with_token(&self, method: Method, url: impl IntoUrl) -> RequestBuilder {
        self.client.request(method, url).header(
            header::AUTHORIZATION,
            format!("Bearer {}", self.access_token),
        )
    }

    fn jfs_req(&self, method: Method, path: &str) -> crate::Result<RequestBuilder> {
        static JFS_BASE: Lazy<Url> =
            Lazy::new(|| Url::parse("https://jfs.jottacloud.com/jfs/").unwrap());

        let url = JFS_BASE.join(path)?;

        Ok(self.req_with_token(method, url))
    }

    fn files_v1_req_builder(&self, method: Method, path: &str) -> crate::Result<RequestBuilder> {
        static FILES_V1_BASE: Lazy<Url> =
            Lazy::new(|| Url::parse("https://api.jottacloud.com/files/v1/").unwrap());

        let url = FILES_V1_BASE.join(path)?;

        Ok(self.req_with_token(method, url))
    }

    pub async fn allocate(&self, req: &AllocReq<'_>) -> crate::Result<AllocRes> {
        let res = self
            .files_v1_req_builder(Method::POST, "allocate")?
            .json(req)
            .send()
            .await?;

        Ok(read_json(res).await??)
    }

    pub async fn put_data(
        &self,
        upload_url: &str,
        body: impl Into<Body>,
        range: RangeInclusive<usize>,
    ) -> crate::Result<UploadRes> {
        let res = self
            .req_with_token(Method::POST, upload_url)
            .body(body)
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .header(header::CONTENT_LENGTH, range.end() + 1 - range.start())
            .header(
                header::RANGE,
                format!("bytes={}-{}", range.start(), range.end()),
            )
            .send()
            .await?;

        let pool = res.headers().get("pool");
        dbg!(pool);

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

    pub async fn index(&self, path: &Path) -> crate::Result<Index> {
        let res = self
            .jfs_req(
                Method::GET,
                &format!("{}/{}", self.access_token.username(), path),
            )?
            .send()
            .await?;

        read_xml(res).await
    }

    // pub async fn open(&self, path: &Path) -> crate::Result<()> {

    // }

    pub async fn file_meta(&self, path: &Path) -> crate::Result<FileMeta> {
        let res = self
            .jfs_req(
                Method::GET,
                &format!("{}/{}", self.access_token.username(), path),
            )?
            .send()
            .await?;

        read_xml(res).await
    }

    pub async fn open(
        &self,
        path: &Path,
        range: impl Into<OptionalByteRange>,
    ) -> crate::Result<impl Stream<Item = reqwest::Result<Bytes>>> {
        let range: OptionalByteRange = range.into();

        let res = self
            .jfs_req(
                Method::GET,
                &format!("{}/{}?mode=bin", self.access_token.username(), path),
            )?
            // status will be `206 Partial Content` even if the whole body is returned
            .header(header::RANGE, range)
            .send()
            .await?;

        if !res.status().is_success() {
            let err_xml = res.text().await?;
            let err: XmlErrorBody = serde_xml_rs::from_str(&err_xml)?;
            return Err(err.into());
        }

        Ok(res.bytes_stream())
    }
}

#[derive(Debug)]
pub enum OptionalByteRange {
    Full,
    From { start: usize },
    To { end: usize },
    Inclusive { start: usize, end: usize },
}

impl OptionalByteRange {
    #[must_use]
    fn len(&self) -> Option<usize> {
        match self {
            Self::Full | Self::From { .. } => None,
            Self::To { end } => Some(end + 1),
            Self::Inclusive { start, end } => Some(end - start + 1),
        }
    }

    #[must_use]
    fn start(&self) -> usize {
        match self {
            Self::Full | Self::To { .. } => 0,
            Self::Inclusive { start, .. } | Self::From { start } => *start,
        }
    }

    #[must_use]
    fn end(&self) -> Option<usize> {
        match self {
            Self::Full | Self::From { .. } => None,
            Self::Inclusive { end, .. } | Self::To { end } => Some(*end),
        }
    }
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

impl<R> From<R> for OptionalByteRange
where
    R: RangeBounds<usize>,
{
    fn from(bounds: R) -> Self {
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

        assert!(
            start.unwrap_or(0) <= end.unwrap_or(usize::MAX),
            "range end must not be lower than start"
        );
        assert_ne!(end, Some(0), "range must not end at 0");

        match (start, end) {
            (None, None) => Self::Full,
            (Some(start), None) => Self::From { start },
            (None, Some(end)) => Self::To { end },
            (Some(start), Some(end)) => Self::Inclusive { start, end },
        }
    }
}
