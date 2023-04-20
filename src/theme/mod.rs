use cursive::theme::{BorderStyle, Color, Palette, Theme};

pub(crate) fn get_theme() -> Theme {
    Theme {
        palette: {
            use cursive::theme::PaletteColor::*;

            let mut palette = Palette::default();

            palette[Background] = Color::parse("#20222d").unwrap();
            palette[Shadow] = Color::parse("#000000").unwrap();
            palette[View] = Color::parse("#282a36").unwrap();
            palette[Primary] = Color::parse("#f8f8f2").unwrap();
            palette[Secondary] = Color::parse("#6272a4").unwrap();
            palette[Tertiary] = Color::parse("#f1fa8c").unwrap();
            palette[TitlePrimary] = Color::parse("#50fa7b").unwrap();
            palette[TitleSecondary] = Color::parse("#ff79c6").unwrap();
            palette[Highlight] = Color::parse("#8be9fd").unwrap();
            palette[HighlightInactive] = Color::parse("#6272a4").unwrap();
            palette[HighlightText] = Color::parse("#282a36").unwrap();

            // {
            //     use cursive::theme::Effect::*;
            //     use cursive::theme::PaletteStyle::*;
            //     use cursive::theme::Style;
            //     palette[Highlight] = Style::from(Yellow.dark()).combine(Bold);
            // }

            palette
        },
        shadow: true,
        borders: BorderStyle::Outset,
    }
}
