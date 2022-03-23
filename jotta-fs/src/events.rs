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

use std::str::FromStr;

use crate::serde::OptTypoDateTime;
use chrono::{DateTime, Utc};
use futures::{Future, SinkExt, StreamExt};
use reqwest::{Method, Url};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

use crate::{api::read_xml, path::AbsolutePath, Fs};

async fn create_ws_token(fs: &Fs) -> crate::Result<String> {
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
pub enum ClientMessage<'a> {
    /// Subscribe to events.
    Subscribe {
        /// Path to watch. `"ALL"` watches all paths.
        path: &'a str,

        /// User agent.
        #[serde(rename = "UA")]
        user_agent: &'a str,
    },

    /// Send a ping to keep the connection open.
    Ping,
}

impl<'a> TryFrom<ClientMessage<'a>> for Message {
    type Error = serde_json::Error;

    fn try_from(value: ClientMessage<'a>) -> Result<Self, Self::Error> {
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

    /// Device that triggered this event.
    pub actor_device: String,

    /// Creation date.
    #[serde_as(as = "OptTypoDateTime")]
    pub created: Option<DateTime<Utc>>,

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
    pub modified: Option<DateTime<Utc>>,

    /// Revision number (starts at one).
    #[serde(with = "serde_with::rust::display_fromstr")]
    pub revision: u32,

    /// Size of the file (bytes).
    #[serde(with = "serde_with::rust::display_fromstr")]
    pub size: u64,

    /// Update time.
    #[serde_as(as = "OptTypoDateTime")]
    pub updated: Option<DateTime<Utc>>,
}

/// A photo.
#[serde_as]
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct WsPhoto {
    /// Capture date.
    #[serde_as(as = "serde_with::TimestampNanoSeconds")]
    pub captured_date: DateTime<Utc>,

    /// MD5 digest.
    #[serde(with = "crate::serde::md5_hex")]
    pub md5: md5::Digest,

    /// Id of the photo (unknown format).
    pub photo_id: String,

    /// UUID of the photo.
    pub uuid: Uuid,
}

/// An event that happened in the cloud.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(tag = "ST", content = "D")]
pub enum ServerEvent {
    /// New file uploaded.
    NewUpload(WsFile),

    /// File deleted.
    Delete(WsFile),

    /// (Hopefully) returned by [`ClientMessage::Ping`].
    Pong(String),
    #[serde(rename = "PhotoAdded")]

    /// Photo added.
    PhotoAdded(WsPhoto),
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
        last_uuid: String,
    },
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]

    /// An event.
    Event {
        /// Timestamp of the event.
        #[serde_as(as = "serde_with::TimestampMilliSeconds")]
        ts: DateTime<Utc>,

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
            println!("{}", json);

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

pub fn subscribe<'a>(fs: &'a Fs) -> impl Future<Output = crate::Result<()>> + 'a {
    async {
        let token = create_ws_token(fs).await?;

        let (mut stream, _) = connect_async(Url::parse(&format!(
            "wss://websocket.jottacloud.com/ws/{}/{}",
            fs.username(),
            token
        ))?)
        .await
        .unwrap();

        stream
            .send(
                ClientMessage::Subscribe {
                    path: "ALL",
                    user_agent: "Helo",
                }
                .try_into()
                .unwrap(),
            )
            .await
            .unwrap();

        while let Some(msg) = stream.next().await {
            let msg = ServerMessage::try_from(msg.unwrap()).unwrap();

            dbg!(msg);
        }

        todo!();
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use hex_literal::hex;
    use std::str::FromStr;
    use uuid::Uuid;

    use crate::events::ServerMessage;

    use super::{ServerEvent, WsPhoto};

    #[test]
    fn deserialize() {
        let msg = ServerMessage::from_str(
        r#"{"EVENT":{"U":"69420","A":"69420","T":"PHOTOS","ST":"PhotoAdded","TS":1648065676008,"D":{"captured_date":1646419475030718700,"md5":"a68184e9a6c263e782fbb40f9c3a3873","photo_id":"aaaaaaaaaaa","uuid":"ff5d0c63-aae3-11ec-881d-90e2bae6bf68"},"uuid":"ff5d0c63-aae3-11ec-881d-90e2bae6bf68","unixnano":1648065676.0228963}}"#
    ).unwrap();

        match msg {
            ServerMessage::Event {
                ts: _,
                inner: ServerEvent::PhotoAdded(photo),
            } => assert_eq!(
                photo,
                WsPhoto {
                    captured_date: Utc.timestamp(1646419475, 30718700),
                    md5: md5::Digest(hex!("a68184e9a6c263e782fbb40f9c3a3873")),
                    photo_id: "aaaaaaaaaaa".into(),
                    uuid: Uuid::parse_str("ff5d0c63-aae3-11ec-881d-90e2bae6bf68").unwrap(),
                }
            ),
            _ => panic!("wrong type"),
        }
    }
}
