use chrono::{DateTime, Utc};
use handlebars::Handlebars;
use k8s_openapi::api::core::v1::Pod;
use k8s_openapi::serde_json;
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
    let engine = engine_factory::build_engine(&["/home/user/.kgv/modules".into()]);
    handlebars.set_engine(engine);
    handlebars.register_template_file("tpl", "./templates/template.hbs")?;
    handlebars.register_script_helper_file("len_getter", "./templates/goals.rhai")?;

    let pod: Pod = serde_json::from_value(json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": { "name": "example" },
        "spec": {
            "containers": [{
                "name": "example",
                "image": "alpine",
                "command": ["tail", "-f", "/dev/null"],
            }],
        }
    }))
    .unwrap();

    println!("{}", handlebars.render("tpl", &pod)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::eval::sample_hbs;

    #[test]
    fn test() {
        sample_hbs().unwrap();
    }
}
