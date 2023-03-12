use std::path::PathBuf;

use cursive::reexports::log::info;
use rhai::module_resolvers::{FileModuleResolver, ModuleResolversCollection};
use rhai::{exported_module, Engine};

use crate::eval::eval_result::EvalResult;
use crate::util::ui::string_ago;

pub fn build_engine(paths: &[PathBuf]) -> Engine {
    let mut engine = Engine::new();
    let collection_resolver = prepare_resolvers(paths);
    engine
        .register_fn("compute_age", string_ago)
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
        .set_module_resolver(collection_resolver);

    engine
}

fn prepare_resolvers(paths: &[PathBuf]) -> ModuleResolversCollection {
    let mut collection = ModuleResolversCollection::new();

    for path in paths {
        let mut module_resolver = FileModuleResolver::new();
        module_resolver.set_base_path(path).set_extension("rhai");
        collection.push(module_resolver);
    }

    collection
}

#[cfg(test)]
mod tests {
    use rhai::{Scope, AST};

    use super::*;

    #[test]
    fn test_modules_loading() {
        let dir = tempfile::tempdir().unwrap();
        let dir = dir.into_path();
        let engine = build_engine(&[dir.clone()]);

        std::fs::write(
            dir.join("pod.rhai"),
            r##"
        fn calculate(x) {
            x + 5
        }
        "##,
        )
        .unwrap();

        let ast: AST = engine
            .compile(
                r#"import "pod" as pod;
                pod::calculate(value)
        "#,
            )
            .unwrap();

        let mut scope = Scope::new();
        scope.push("value", 5_i64);

        let result: i64 = engine.eval_ast_with_scope(&mut scope, &ast).unwrap();
        assert_eq!(result, 10);
    }
}
