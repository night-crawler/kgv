use cursive::theme::{BorderStyle, Palette, Theme};

pub fn get_theme() -> Theme {
    Theme {
        palette: {
            use cursive::theme::BaseColor::*;
            use cursive::theme::PaletteColor::*;

            let mut palette = Palette::default();

            palette[View] = Black.dark();
            palette[Background] = Black.dark();
            palette[Primary] = White.dark();
            palette[TitlePrimary] = Red.dark();
            palette[Highlight] = Red.dark();
            palette[HighlightInactive] = Black.dark();
            palette[HighlightText] = White.dark();

            {
                use cursive::theme::Effect::*;
                use cursive::theme::PaletteStyle::*;
                use cursive::theme::Style;
                palette[Highlight] = Style::from(Red.light()).combine(Bold);
            }

            palette
        },
        shadow: true,
        borders: BorderStyle::Outset,
    }
}
