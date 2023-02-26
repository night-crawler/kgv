use anyhow::Context;
use chrono::{DateTime, Utc};
use handlebars::Handlebars;
use k8s_openapi::api::core::v1::Pod;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Time;
use k8s_openapi::serde_json;
use k8s_openapi::serde_json::json;
use rhai::{exported_module, Dynamic, Engine, Scope, AST};

use crate::eval::eval_result::EvalResult;
use crate::util::error::EvalError;
use crate::util::ui::ago;

pub mod eval_result;
pub mod evaluator;

pub fn build_engine() -> Engine {
    let mut engine = Engine::new();
    engine
        .set_max_expr_depths(1024, 1024)
        .register_type_with_name::<EvalResult>("Result")
        .register_static_module(
            "Result",
            exported_module!(crate::eval::eval_result::eval_result_module).into(),
        );

    engine
}

pub fn string_ago(s: &str) -> Result<String, EvalError> {
    let dt = DateTime::parse_from_rfc3339(s)?;
    let then_utc: DateTime<Utc> = dt.with_timezone(&Utc);
    let now = Utc::now();
    Ok(ago(now - then_utc))
}

pub fn sample_pod_extract() -> anyhow::Result<()> {
    let mut pod: Pod = serde_json::from_value(json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": { "name": "example" },
        "spec": {
            "containers": [{
                "name": "example",
                "image": "alpine",
                // Do nothing
                "command": ["tail", "-f", "/dev/null"],
            }],
        }
    }))?;
    pod.metadata.creation_timestamp = Some(Time(Utc::now()));
    let serialized_pod = serde_json::to_string(&pod)?;

    let engine = Engine::new();

    let qqq: AST = engine.compile(
        r##"
        debug(resource.metadata.creationTimestamp);
        let x = resource.spec;
        let y = x + "lol";
        let qwe = ago(resource.metadata.creationTimestamp);
        Result::MaybeString(qwe)
        "##,
    )?;

    let mut engine = Engine::new();

    let map = engine.parse_json(serialized_pod, false)?;
    engine.register_fn("ago", string_ago);
    engine
        .register_type_with_name::<EvalResult>("Result")
        .register_static_module(
            "Result",
            exported_module!(crate::eval::eval_result::eval_result_module).into(),
        );

    // engine.register_type::<Pod>();
    engine.on_debug(|x, src, pos| {
        let src = src.unwrap_or("unknown");
        println!("DEBUG of {src} at {pos:?}: {x}")
    });

    let mut scope = Scope::new();

    scope.push("resource", map);

    let bla: Dynamic = engine.eval_ast_with_scope(&mut scope, &qqq)?;
    let result: EvalResult = bla.try_cast().context("no")?;

    println!("{:?}", result);

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        sample_pod_extract().unwrap();
    }
}
