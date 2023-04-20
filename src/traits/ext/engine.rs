use std::path::Path;

use rhai::{Engine, EvalAltResult, AST};

pub(crate) trait EngineExt {
    fn compile_file_with_imports(
        &self,
        file: &Path,
        imports: &[String],
    ) -> Result<AST, EvalAltResult>;
    fn compile_content_with_imports(
        &self,
        content: &str,
        imports: &[String],
    ) -> Result<AST, EvalAltResult>;
}

impl EngineExt for Engine {
    fn compile_file_with_imports(
        &self,
        file: &Path,
        imports: &[String],
    ) -> Result<AST, EvalAltResult> {
        let content = std::fs::read_to_string(file).map_err(|err| {
            EvalAltResult::ErrorSystem(
                format!("Cannot open script file '{}'", file.display()),
                err.into(),
            )
        })?;
        self.compile_content_with_imports(&content, imports)
    }

    fn compile_content_with_imports(
        &self,
        content: &str,
        imports: &[String],
    ) -> Result<AST, EvalAltResult> {
        let mut final_content = imports.join("\n");
        final_content.push_str("\n\n");
        final_content.push_str(content);

        let mut ast: AST = self.compile(&final_content)?;
        ast.set_source(final_content);
        Ok(ast)
    }
}
