use anyhow::{Context, Error};
use cursive::utils::markup::StyledString;

pub struct Highlighter {
    theme: syntect::highlighting::Theme,
    syntax_set: syntect::parsing::SyntaxSet,
}

impl Highlighter {
    pub fn new(theme_name: &str) -> anyhow::Result<Self> {
        let theme = Self::get_theme(theme_name)?;
        let syntax_set = syntect::parsing::SyntaxSet::load_defaults_newlines();

        Ok(Self { theme, syntax_set })
    }

    fn get_theme(theme_name: &str) -> Result<syntect::highlighting::Theme, Error> {
        let mut theme_set = syntect::highlighting::ThemeSet::load_defaults();
        println!("{:?}", theme_set.themes.keys());
        theme_set
            .themes
            .remove(theme_name)
            .with_context(|| format!("Could not find specified theme {}", theme_name))
    }

    pub fn highlight(&self, text: &str, syntax_extension: &str) -> anyhow::Result<StyledString> {
        let syntax = self
            .syntax_set
            .find_syntax_by_extension(syntax_extension)
            .with_context(|| format!("Could not find syntax by extension {}", syntax_extension))?;
        let mut hl_lines = syntect::easy::HighlightLines::new(syntax, &self.theme);
        let result = cursive_syntect::parse(text, &mut hl_lines, &self.syntax_set)?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let hl = Highlighter::new("base16-eighties.dark").unwrap();
        let styled_string = hl.highlight("sample: true", "yaml");
        assert!(styled_string.is_ok());
    }
}
