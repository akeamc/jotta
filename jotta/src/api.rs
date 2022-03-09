use reqwest::Response;
use serde::{de::DeserializeOwned, Deserialize};

/// An exception thrown by the upstream API.
#[derive(Debug, Deserialize)]
pub enum Exception {
    /// Thrown when a file name conflict occurrs.
    UniqueFileException,
    /// Missing or invalid credentials.
    BadCredentialsException,
    /// Checksum mismatch.
    CorruptUploadOpenApiException,
    /// File or folder doesn't exist.
    NoSuchFileException,
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

/// Error body similar to [`JsonErrorBody`] but for the JFS.
#[derive(Debug, Deserialize)]
pub struct XmlErrorBody {
    /// Error code.
    pub code: u16,
    /// Error message, often starting with `no.jotta.backup.errors.<exception>`.
    ///
    /// TODO: Implement parser for exceptions specified here.
    pub message: Option<String>,
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

/// Parse JSON as the associated type if the response has a 2xx status
/// code, otherwise parse it as [`JsonErrorBody`].
///
/// # Errors
///
/// - invalid json
/// - malformed json
pub async fn read_json<T: DeserializeOwned>(
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
pub async fn read_xml<T: DeserializeOwned>(res: Response) -> crate::Result<T> {
    let status = res.status();
    let xml = res.text().await?;

    println!("{}", xml);

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
