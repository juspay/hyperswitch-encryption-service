use crate::errors;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
#[serde(tag = "data_identifier", content = "key_identifier")]
pub enum Identifier {
    User(String),
    Merchant(String),
    UserAuth(String),
}

impl Identifier {
    pub fn get_identifier(&self) -> (String, String) {
        match self {
            Self::User(id) => (String::from("User"), id.clone()),
            Self::Merchant(id) => (String::from("Merchant"), id.clone()),
            Self::UserAuth(id) => (String::from("UserAuth"), id.clone()),
        }
    }
}

impl core::fmt::Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User(s) => f.write_str(&format!("User_{}", s)),
            Self::Merchant(s) => f.write_str(&format!("Merchant_{}", s)),
            Self::UserAuth(s) => f.write_str(&format!("UserAuth_{}", s)),
        }
    }
}

impl TryFrom<(String, String)> for Identifier {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(value: (String, String)) -> Result<Self, Self::Error> {
        let did = value.0.as_str();
        let kid = value.1;

        match (did, kid) {
            ("User", kid) => Ok(Self::User(kid)),
            ("Merchant", kid) => Ok(Self::Merchant(kid)),
            ("UserAuth", kid) => Ok(Self::UserAuth(kid)),
            (_, _) => Err(error_stack::Report::from(
                errors::ParsingError::ParsingFailed(String::from("Failed to parse Identifier")),
            )),
        }
    }
}
