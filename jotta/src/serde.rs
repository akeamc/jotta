use serde::{Deserialize, Deserializer, Serializer};
use serde_with::{DeserializeAs, SerializeAs};
use time::{macros::format_description, OffsetDateTime, PrimitiveDateTime};

/// The folks at Jottacloud screwed up and added an extra dash in some of their dates:
///
/// ```txt
/// # api responses
/// 2022-02-24-T04:20:00Z
///
/// # iso8601
/// 2022-02-24T04:20:00Z
/// ```
fn parse_typo_datetime(s: &str) -> Result<OffsetDateTime, time::error::Parse> {
    let format = format_description!("[year]-[month]-[day]-T[hour]:[minute]:[second]Z");

    let prim = PrimitiveDateTime::parse(s, &format)?;
    Ok(prim.assume_utc())
}

pub(crate) struct OptTypoDateTime;

impl<'de> DeserializeAs<'de, Option<OffsetDateTime>> for OptTypoDateTime {
    fn deserialize_as<D>(deserializer: D) -> Result<Option<OffsetDateTime>, D::Error>
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

pub(crate) struct UnixMillis;

impl SerializeAs<OffsetDateTime> for UnixMillis {
    fn serialize_as<S>(source: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let ms = (source.unix_timestamp_nanos() / 1_000_000)
            .try_into()
            .map_err(serde::ser::Error::custom)?;
        serializer.serialize_i64(ms)
    }
}

impl<'de> DeserializeAs<'de, OffsetDateTime> for UnixMillis {
    fn deserialize_as<D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let ms = i64::deserialize(deserializer)?;
        OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000)
            .map_err(serde::de::Error::custom)
    }
}

pub(crate) mod md5_hex {
    use hex::FromHexError;
    use md5::Digest;
    use serde::{Deserialize, Deserializer, Serializer};

    pub(crate) fn serialize<S: Serializer>(
        digest: &Digest,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{:x}", digest))
    }

    pub(crate) fn hex_to_digest(str: &str) -> Result<Digest, FromHexError> {
        let mut bytes = [0; 16];
        hex::decode_to_slice(str, &mut bytes)?;
        Ok(Digest(bytes))
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Digest, D::Error> {
        let str = String::deserialize(deserializer)?;
        hex_to_digest(&str).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use time::macros::datetime;

    use crate::serde::parse_typo_datetime;

    #[test]
    fn typo_datetime_parsing() {
        assert_eq!(
            parse_typo_datetime("2020-05-16-T10:46:05Z").unwrap(),
            datetime!(2020-05-16 10:46:05 +00:00:00)
        );
    }
}
