use serde::Serialize;

pub mod files;
pub mod errors;

#[derive(Debug, Serialize)]
pub struct AccessToken(String);

impl AccessToken {
    pub fn new(value: String) -> Self {
        Self(value)
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
