use rhai::plugin::*;
use strum_macros::AsRefStr;
use crate::util::error::EvalError;

#[derive(Clone, Debug, AsRefStr)]
pub enum EvalResult {
    String(String),
    MaybeString(Result<String, EvalError>),
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
