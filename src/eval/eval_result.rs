use std::cmp::Ordering;

use rhai::plugin::*;
use strum_macros::AsRefStr;

use crate::util::error::EvalError;
use crate::util::ui::ago;

#[derive(Clone, Debug, AsRefStr)]
pub enum EvalResult {
    Error(String),
    Duration(chrono::Duration),
    String(String),
    Int(i64),
    MaybeString(Result<String, EvalError>),
}

impl Eq for EvalResult {}

impl PartialEq<Self> for EvalResult {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Error(left), Self::Error(right)) => left == right,
            (Self::Duration(left), Self::Duration(right)) => left == right,
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
            (Self::Duration(left), Self::Duration(right)) => left.partial_cmp(right),
            (Self::String(left), Self::String(right)) => left.partial_cmp(right),
            (Self::Int(left), Self::Int(right)) => left.partial_cmp(right),
            (Self::MaybeString(left), Self::MaybeString(right)) => match (left, right) {
                (Ok(left_string), Ok(right_string)) => left_string.partial_cmp(right_string),
                _ => None,
            },
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
            EvalResult::Duration(duration) => ago(*duration),
            EvalResult::String(str) => str.to_string(),
            EvalResult::Int(val) => val.to_string(),
            EvalResult::MaybeString(value) => match value {
                Ok(value) => value.clone(),
                Err(err) => format!("{}", err),
            },
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
}
