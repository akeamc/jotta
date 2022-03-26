use std::marker::PhantomData;

use serde::Deserialize;
use serde_with::DeserializeAs;

/// Treat `null` values as defaults.
pub(crate) struct NullAsDefault<T>(PhantomData<T>);

impl<'de, T> DeserializeAs<'de, Option<T>> for NullAsDefault<T>
where
    T: Deserialize<'de> + Default + std::fmt::Debug,
{
    fn deserialize_as<D>(deserializer: D) -> Result<Option<T>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Option::<T>::deserialize(deserializer)?.unwrap_or_default();

        Ok(Some(value))
    }
}

#[cfg(test)]
mod tests {
    use super::NullAsDefault;
    use serde::Deserialize;
    use serde_json::json;
    use serde_with::serde_as;

    #[test]
    fn null_as_default() {
        #[serde_as]
        #[derive(Debug, Deserialize, PartialEq, Eq)]
        struct Params {
            #[serde(default)]
            #[serde_as(as = "NullAsDefault<u32>")]
            score: Option<u32>,
        }

        let cases = vec![
            (json!({}), Params { score: None }),
            (json!({ "score": null }), Params { score: Some(0) }),
            (json!({ "score": 42 }), Params { score: Some(42) }),
        ];

        for (value, expected) in cases {
            let params: Params = serde_json::from_value(value).unwrap();
            assert_eq!(params, expected);
        }
    }
}
