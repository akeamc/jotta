use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Deserializer};
use serde_with::DeserializeAs;

/// The folks at Jottacloud screwed up and added an extra dash in some of their dates:
///
/// ```txt
/// # api responses
/// 2022-02-24-T04:20:00Z
///
/// # iso8601
/// 2022-02-24T04:20:00Z
/// ```
fn parse_typo_datetime(s: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
    dbg!(s);

    let dt = NaiveDateTime::parse_from_str(s, "%Y-%m-%d-T%H:%M:%SZ")?;
    let dt = Utc.from_local_datetime(&dt).unwrap();

    dbg!(dt);

    Ok(dt)
}

pub struct OptTypoDateTime;

impl<'de> DeserializeAs<'de, Option<DateTime<Utc>>> for OptTypoDateTime {
    fn deserialize_as<D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = Option::<String>::deserialize(deserializer)?.filter(|s| !s.is_empty());

        match s {
            Some(s) => parse_typo_datetime(&s)
                .map_err(serde::de::Error::custom)
                .map(Some),
            None => Ok(None),
        }
    }
}

pub mod md5_hex {
    use hex::FromHexError;
    use md5::Digest;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(digest: &Digest, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{:x}", digest))
    }

    pub fn hex_to_digest(str: &str) -> Result<Digest, FromHexError> {
        let mut bytes = [0; 16];
        hex::decode_to_slice(str, &mut bytes)?;
        Ok(Digest(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Digest, D::Error> {
        let str = String::deserialize(deserializer)?;
        hex_to_digest(&str).map_err(serde::de::Error::custom)
    }
}
