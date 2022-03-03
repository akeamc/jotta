use serde::{Deserialize, Serialize};

pub mod errors;
pub mod files;
pub mod jfs;

#[derive(Debug, Deserialize)]
pub struct AccessTokenClaims {
    pub username: String,
}

#[derive(Debug, Serialize)]
pub struct AccessToken(String);

impl AccessToken {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn jwt_claims(&self) -> AccessTokenClaims {
        let mut segments = self.0.split('.');
        let _header = segments.next();
        let payload = segments.next().expect("malformed token");
        let json = base64::decode_config(payload, base64::URL_SAFE_NO_PAD).unwrap();
        let json = String::from_utf8(json).unwrap();
        let claims: AccessTokenClaims = serde_json::from_str(&json).unwrap();

        claims
    }

    pub fn username(&self) -> String {
        self.jwt_claims().username
    }
}

impl std::fmt::Display for AccessToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
