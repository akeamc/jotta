//! Listen for Jottacloud events via the websocket API.
//!
//! The websocket API has a very wacky naming convention, mixing `camelCase`,
//! `SCREAMING_SNAKE_CASE`, `PascalCase` and `snake_case` seemingly randomly.
//! Therefore, most the structs and enums in this module are polluted with lots
//! and lots of [Serde](https://serde.rs/) attributes.
//!
//! A somewhat outdated (but nonetheless very helpful) mapping of the API has
//! been written by [ttyridal](https://github.com/ttyridal):
//! [Jotta protocol](https://github.com/ttyridal/jottalib/wiki/Jotta-protocol).
//!
//! I initially tried to cover all events (including file sharing, photo albums
//! and such) but realized that there are too many events and most of them will
//! probably never be used (open an issue otherwise). So, only basic filesystem
//! operations are covered by this API. Other events *will* yield a stream item,
//! but the item will be an `Err(..)` unless I screwed up real bad.

use std::str::FromStr;

use crate::{auth::Provider, serde::OptTypoDateTime, USER_AGENT};
use futures::{future, Sink, SinkExt, Stream, StreamExt};
use reqwest::{Method, Url};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use time::OffsetDateTime;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{self, Message},
};
use tracing::trace;
use uuid::Uuid;

use crate::{api::read_xml, path::AbsolutePath, Fs};

async fn create_ws_token<P: Provider>(fs: &Fs<P>) -> crate::Result<String> {
    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct TokenResponse {
        // username: String,
        auth_token: String,
    }

    let res = fs
        .authed_req(
            Method::GET,
            format!(
                "https://jfs.jottacloud.com/rest/token/{}/createToken",
                fs.username()
            ),
        )
        .await?
        .send()
        .await?;

    let data: TokenResponse = read_xml(res).await?;

    Ok(data.auth_token)
}

/// A message sent from the client to the server.
#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClientMessage {
    /// Subscribe to events.
    Subscribe {
        /// Path to watch. `"ALL"` watches all paths.
        path: String,

        /// User agent.
        #[serde(rename = "UA")]
        user_agent: String,
    },

    /// Send a ping to keep the connection open.
    Ping,
}

impl TryFrom<ClientMessage> for Message {
    type Error = serde_json::Error;

    fn try_from(value: ClientMessage) -> Result<Self, Self::Error> {
        serde_json::to_string(&value).map(Message::text)
    }
}

/// A file sent in some of the websocket messages.
///
/// This is an attempt to make serde parse JSON objects such as this one:
///
/// ```json
/// {
///   "FROM":"/{username}/Jotta/Sync/blabla",
///   "actorDevice":"WEBAPP",
///   "created":"2016-02-04-T07:56:43Z",
///   "dfs":"04KZFaGU",
///   "fileuuid":"da635047-34dd-46e2-99c3-091762fe20d0",
///   "md5":"02588fb184ae4930cf998b8af2e613e7",
///   "mimeType":"APPLICATION_OCTET_STREAM",
///   "modified":"2016-02-04-T07:56:43Z",
///   "revision":"1",
///   "size":"17",
///   "updated":"2016-02-04-T07:58:46Z",
///   "uuid":"a2f5e550-cb15-11e5-b530-002590c0b00c"}
/// }
/// ```
#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsFile {
    /// Path.
    #[serde(rename = "FROM")]
    pub from: AbsolutePath,

    /// Destination (only used for events such as `"MOVE"`).
    #[serde(rename = "TO")]
    pub to: Option<AbsolutePath>,

    /// Device that triggered this event.
    pub actor_device: String,

    /// Creation date.
    #[serde_as(as = "OptTypoDateTime")]
    pub created: Option<OffsetDateTime>,

    /// Node that fulfilled the request.
    pub dfs: String,

    /// UUID of the file.
    #[serde(rename = "fileuuid")]
    pub file_uuid: Uuid,

    /// MD5 digest.
    #[serde(with = "crate::serde::md5_hex")]
    pub md5: md5::Digest,

    /// Media type of the file.
    pub mime_type: String,

    /// Modification date.
    #[serde_as(as = "OptTypoDateTime")]
    pub modified: Option<OffsetDateTime>,

    /// Revision number (starts at one).
    #[serde(with = "serde_with::rust::display_fromstr")]
    pub revision: u32,

    /// Size of the file (bytes).
    #[serde(with = "serde_with::rust::display_fromstr")]
    pub size: u64,

    /// Update time.
    #[serde_as(as = "OptTypoDateTime")]
    pub updated: Option<OffsetDateTime>,
}

/// A directory.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsDir {
    /// Path.
    #[serde(rename = "FROM")]
    pub from: AbsolutePath,

    /// No idea what this is.
    pub actor_device: String,

    /// Uuid.
    pub uuid: Uuid,
}

/// An event that happened in the cloud.
#[derive(Debug, Deserialize)]
#[serde(tag = "ST", content = "D", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ServerEvent {
    /// (Hopefully) returned by [`ClientMessage::Ping`].
    Pong(String),

    /// New file uploaded.
    NewUpload(WsFile),

    /// File deleted.
    Delete(WsFile),

    /// File restored.
    Restore(WsFile),

    /// File was moved.
    Move(WsFile),

    /// Create directory.
    CreateDir(WsDir),

    /// Permanently delete a directory.
    HardDeleteDir(WsDir),
}

/// Server event kinds.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventKind {
    /// File-related events are named `"PATH"` for some reason.
    Path,
}

/// A message sent by the server to the client (us).
#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ServerMessage {
    /// Subscription confirmation.
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    Subscribe {
        /// Path.
        path: String,

        /// Last file that was uploaded, deleted, etc.
        last_uuid: Uuid,
    },

    /// An event.
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    Event {
        /// What kind of event.
        #[serde(rename = "T")]
        kind: EventKind,

        /// Timestamp of the event.
        #[serde_as(as = "crate::serde::UnixMillis")]
        ts: OffsetDateTime,

        /// Inner event data.
        #[serde(flatten)]
        inner: ServerEvent,
    },
}

impl FromStr for ServerMessage {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

impl TryFrom<Message> for ServerMessage {
    type Error = ParseServerMessageError;

    fn try_from(value: Message) -> Result<Self, Self::Error> {
        if let Message::Text(json) = value {
            trace!("{}", json);

            Self::from_str(&json).map_err(Into::into)
        } else {
            Err(ParseServerMessageError::WrongType)
        }
    }
}

/// Server message parse error.
#[derive(Debug, thiserror::Error)]
pub enum ParseServerMessageError {
    /// JSON error.
    #[error("{0}")]
    Json(#[from] serde_json::Error),

    /// The websocket message type must be text.
    #[error("wrong message type (must be text)")]
    WrongType,
}

/// Event error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The message could not be parsed.
    #[error("message could not be parsed: {0}")]
    ParseMessageError(#[from] ParseServerMessageError),

    /// An error occurred with the underlying websocket.
    #[error("websocket error: {0}")]
    WsError(#[from] tungstenite::Error),

    /// JSON error.
    #[error("json error: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// Subscribe to remote events.
///
/// # Errors
///
/// Might error due to authentication errors. Also, it is not 100% certain that
/// we will be able to connect to the websocket.
pub async fn subscribe<P: Provider>(
    fs: &Fs<P>,
) -> crate::Result<impl Stream<Item = Result<ServerMessage, Error>> + Sink<ClientMessage>> {
    let token = create_ws_token(fs).await?;

    let (stream, _) = connect_async(Url::parse(&format!(
        "wss://websocket.jottacloud.com/ws/{}/{}",
        fs.username(),
        token
    ))?)
    .await
    .map_err(Error::from)?;

    let mut stream = stream
        .with::<_, _, _, Error>(|msg: ClientMessage| {
            future::ready(msg.try_into().map_err(Into::into))
        })
        .map::<Result<ServerMessage, Error>, _>(|result| result?.try_into().map_err(Into::into));

    stream
        .send(ClientMessage::Subscribe {
            path: "ALL".into(),
            user_agent: USER_AGENT.into(),
        })
        .await?;

    Ok(stream)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use uuid::Uuid;

    use crate::events::ServerMessage;

    #[test]
    fn deserialize() {
        let msg = ServerMessage::from_str(
            r#"{"SUBSCRIBE":{"PATH":"ALL","LAST_UUID":"40660078-abab-11ec-881d-90e2bae6bf68"}}"#,
        )
        .unwrap();

        match msg {
            ServerMessage::Subscribe { path, last_uuid } => {
                assert_eq!(path, "ALL");
                assert_eq!(
                    last_uuid,
                    Uuid::parse_str("40660078-abab-11ec-881d-90e2bae6bf68").unwrap()
                );
            }
            _ => panic!("wrong type"),
        }
    }
}
