use std::cmp::Ordering;

use chrono::Utc;
use rhai::plugin::*;
use strum_macros::AsRefStr;

use crate::util::error::{EvalError, KgvError};
use crate::util::ui::{ago, duration_since, string_ago};

#[derive(Clone, Debug)]
pub struct RhaiPseudoResource {
    pub id: String,
    pub resource: Dynamic,
}

#[derive(Clone, Debug, AsRefStr)]
pub enum EvalResult {
    Error(String),
    AgoSince(chrono::DateTime<Utc>),
    String(String),
    Int(i64),
    Ago(String),
    MaybeString(Result<String, EvalError>),
    Vec(Vec<Dynamic>),
}

impl TryFrom<Dynamic> for EvalResult {
    type Error = KgvError;

    fn try_from(value: Dynamic) -> Result<Self, <EvalResult as TryFrom<Dynamic>>::Error> {
        let type_name = value.type_name();
        if value.is_string() {
            let value = value.into_string()?;
            return Ok(EvalResult::String(value));
        } else if value.is_int() {
            return Ok(EvalResult::Int(value.as_int()?));
        } else if value.is_array() {
            let array = value.into_typed_array::<Dynamic>()?;
            return Ok(EvalResult::Vec(array));
        }

        value
            .try_cast::<EvalResult>()
            .ok_or_else(|| KgvError::TypeConversionError(type_name.to_string()))
    }
}

impl Eq for EvalResult {}

impl PartialEq<Self> for EvalResult {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Error(left), Self::Error(right)) => left == right,
            (Self::AgoSince(left), Self::AgoSince(right)) => left == right,
            (Self::String(left), Self::String(right)) => left == right,
            (Self::Int(left), Self::Int(right)) => left == right,
            (Self::MaybeString(left), Self::MaybeString(right)) => match (left, right) {
                (Ok(left), Ok(right)) => left == right,
                _ => false,
            },
            _ => false,
        }
    }
}

impl PartialOrd<Self> for EvalResult {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Self::Error(left), Self::Error(right)) => left.partial_cmp(right),
            (Self::AgoSince(left), Self::AgoSince(right)) => left.partial_cmp(right),
            (Self::String(left), Self::String(right)) => left.partial_cmp(right),
            (Self::Int(left), Self::Int(right)) => left.partial_cmp(right),
            (Self::MaybeString(left), Self::MaybeString(right)) => match (left, right) {
                (Ok(left_string), Ok(right_string)) => left_string.partial_cmp(right_string),
                _ => None,
            },
            (Self::Ago(left), Self::Ago(right)) => {
                match (duration_since(left), duration_since(right)) {
                    (Ok(left), Ok(right)) => left.partial_cmp(&right),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

impl Ord for EvalResult {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

impl ToString for EvalResult {
    fn to_string(&self) -> String {
        match self {
            EvalResult::Error(err) => err.to_string(),
            EvalResult::AgoSince(duration) => {
                let now = Utc::now();
                ago(now - *duration)
            }
            EvalResult::String(str) => str.to_string(),
            EvalResult::Int(val) => val.to_string(),
            EvalResult::MaybeString(value) => match value {
                Ok(value) => value.clone(),
                Err(err) => format!("{}", err),
            },
            EvalResult::Vec(v) => format!("{:?}", v),
            EvalResult::Ago(ts) => string_ago(ts),
        }
    }
}

#[export_module]
pub mod eval_result_module {
    use crate::eval::eval_result::EvalResult;
    use crate::util::error::EvalError;

    #[allow(non_snake_case)]
    pub fn String(value: String) -> EvalResult {
        EvalResult::String(value)
    }

    #[allow(non_snake_case)]
    pub fn MaybeString(value: Result<String, EvalError>) -> EvalResult {
        EvalResult::MaybeString(value)
    }

    #[allow(non_snake_case)]
    pub fn Ago(value: String) -> EvalResult {
        EvalResult::Ago(value)
    }
}

#[allow(non_snake_case)]
pub fn PseudoResource(id: String, resource: Dynamic) -> RhaiPseudoResource {
    RhaiPseudoResource { id, resource }
}
