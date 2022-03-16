//! API client utilities.
use std::str::FromStr;

use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::Response;
use serde::{de::DeserializeOwned, Deserialize};
use strum::EnumString;
use tracing::{trace, warn};

/// An exception thrown by the upstream API.
#[derive(Debug, Deserialize, PartialEq, EnumString)]
pub enum Exception {
    /// Thrown when a file name conflict occurrs.
    UniqueFileException,
    /// Missing or invalid credentials.
    BadCredentialsException,
    /// Checksum mismatch.
    CorruptUploadOpenApiException,
    /// File or folder doesn't exist.
    NoSuchFileException,
    /// Unsure how this is different from `NoSuchFileException`.
    NoSuchPathException,
    /// Invalid parameters (JSON body, query parameters, etc.).
    InvalidArgumentException,
    /// This exception is thrown when uploading chunked data
    /// for some reason, along with an `HTTP 420` status.
    IncompleteUploadOpenApiException,
}

/// A JSON error body returned by the JSON API on errors.
#[derive(Debug, Deserialize)]
pub struct JsonErrorBody {
    /// Error code. Often valid HTTP status codes, but not always.
    pub code: Option<u16>,
    /// Error message.
    pub message: Option<String>,
    /// Error cause.
    pub cause: Option<String>,
    /// Optiona error id.
    pub error_id: Option<MaybeUnknown<Exception>>,
    /// Some kind of tracing id maybe?
    #[serde(rename(deserialize = "x-id"))]
    pub x_id: Option<String>,
}
/// Error message, often in the form of `no.jotta.backup.errors.<exception>: <human-readable message>`.
#[derive(Debug, Deserialize)]
pub struct JavaErrorMessage(pub String);

impl JavaErrorMessage {
    /// Attempt to extract an [`Exception`].
    ///
    /// ```
    /// use jotta_fs::api::{Exception, JavaErrorMessage};
    ///
    /// let exceptions = &[
    ///     ("no.jotta.backup.errors.NoSuchPathException: Directory /user69420/Jotta/Archive/s3-test", Some(Exception::NoSuchPathException)),
    ///     ("OH NO AN INTERNAL ERROR", None),
    ///     ("ArrayIndexOutOfBoundsException", None),
    /// ];
    ///
    /// for (msg, expected) in exceptions {
    ///     let msg = JavaErrorMessage(msg.to_string());
    ///
    ///     assert_eq!(msg.exception_opt(), *expected);
    /// }
    /// ```
    #[must_use]
    pub fn exception_opt(&self) -> Option<Exception> {
        static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(?:\w+\.)*(?P<except>\w+):").unwrap());

        match Exception::from_str(RE.captures(&self.0)?.name("except")?.as_str()) {
            Ok(exception) => Some(exception),
            Err(err) => {
                warn!(
                    "parse upstream exception failed: {:?} (from {})",
                    err, &self.0
                );
                None
            }
        }
    }
}

/// Error body similar to [`JsonErrorBody`] but for the JFS.
#[derive(Debug, Deserialize)]
pub struct XmlErrorBody {
    /// Error code.
    pub code: u16,
    /// Error message.
    pub message: Option<JavaErrorMessage>,
    /// Error reason.
    pub reason: String,
    /// Error cause.
    pub cause: Option<String>,
    /// Hostname of the remote node.
    pub hostname: Option<String>,
    /// Some kind of tracing id maybe?
    #[serde(rename(deserialize = "x-id"))]
    pub x_id: Option<String>,
}

impl XmlErrorBody {
    /// Attempt to extract an [`Exception`].
    #[must_use]
    pub fn exception_opt(&self) -> Option<Exception> {
        self.message.as_ref()?.exception_opt()
    }
}

/// Parse JSON as the associated type if the response has a 2xx status
/// code, otherwise parse it as [`JsonErrorBody`].
///
/// # Errors
///
/// - invalid json
/// - malformed json
pub(crate) async fn read_json<T: DeserializeOwned>(
    res: Response,
) -> reqwest::Result<Result<T, JsonErrorBody>> {
    if res.status().is_success() {
        res.json().await.map(Ok)
    } else {
        res.json().await.map(Err)
    }
}

/// Parse XML as the associated type if the response has a 2xx status
/// code, otherwise parse it as [`XmlErrorBody`].
///
/// # Errors
///
/// - invalid utf-8 response body
/// - invalid xml
pub(crate) async fn read_xml<T: DeserializeOwned>(res: Response) -> crate::Result<T> {
    let status = res.status();
    let xml = res.text().await?;

    trace!("{}", xml);

    if status.is_success() {
        let data = serde_xml_rs::from_str(&xml)?;
        Ok(data)
    } else {
        let e: XmlErrorBody = serde_xml_rs::from_str(&xml)?;
        Err(e.into())
    }
}

/// A serde wrapper for handling unknown enum variants.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum MaybeUnknown<T> {
    /// A known type.
    Known(T),
    /// An unknown type.
    Unknown(String),
}
