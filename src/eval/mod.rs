use chrono::{DateTime, Utc};
use handlebars::Handlebars;
use k8s_openapi::serde_json::json;

use crate::util::error::EvalError;
use crate::util::ui::ago;

pub mod engine_factory;
pub mod eval_result;
pub mod evaluator;

pub fn string_ago(s: &str) -> Result<String, EvalError> {
    let dt = DateTime::parse_from_rfc3339(s)?;
    let then_utc: DateTime<Utc> = dt.with_timezone(&Utc);
    let now = Utc::now();
    Ok(ago(now - then_utc))
}

pub fn sample_hbs() -> anyhow::Result<()> {
    let mut handlebars = Handlebars::new();
    handlebars.register_template_file("tpl", "./templates/template.hbs")?;
    handlebars.register_script_helper_file("len_getter", "./templates/goals.rhai")?;

    let data = json! {[
        [{
            "name": "Dortmund",
            "goals": ["Haaland", "Guerreiro", "Hazard", "Guerreiro"]
        }, {
            "name": "Schalke",
            "goals": []
        }],
        [{
            "name": "RB Leipzig",
            "goals": ["Poulsen"]
        }, {
            "name": "SC Feriburg",
            "goals": ["Gulde"]
        }]
    ]};
    println!("{}", handlebars.render("tpl", &data)?);
    Ok(())
}
