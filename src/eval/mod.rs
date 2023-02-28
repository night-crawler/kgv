use chrono::{DateTime, Utc};
use cursive::reexports::log::info;
use handlebars::Handlebars;
use k8s_openapi::serde_json::json;
use rhai::module_resolvers::FileModuleResolver;
use rhai::{exported_module, Engine, Scope, AST};

use crate::eval::eval_result::EvalResult;
use crate::util::error::EvalError;
use crate::util::ui::ago;

pub mod eval_result;
pub mod evaluator;

pub fn build_engine() -> Engine {
    let mut engine = Engine::new();
    let mut mr = FileModuleResolver::new();
    mr.set_base_path("/home/user/.kgv/scripts/modules")
        .set_extension("rhai");
    engine
        .set_max_expr_depths(64, 64)
        .register_type_with_name::<EvalResult>("Result")
        .register_static_module(
            "Result",
            exported_module!(crate::eval::eval_result::eval_result_module).into(),
        )
        .on_debug(|x, src, pos| {
            let src = src.unwrap_or("unknown");
            info!("ENGINE: {src} at {pos:?}: {x}");
        })
        .set_module_resolver(mr);

    engine
}

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

#[cfg(test)]
mod tests {
    use rhai::Dynamic;
    use super::*;

    #[test]
    fn test() {
        let engine = build_engine();
        let ast: AST = engine
            .compile(
                r#"import "pod" as pod;
                pod::ready(resource)
        "#,
            )
            .unwrap();

        let mut scope = Scope::new();
        scope.push("resource", ());

        let qwe: Dynamic = engine.eval_ast_with_scope(&mut scope, &ast).unwrap();

        println!("{}", qwe);
    }
}
