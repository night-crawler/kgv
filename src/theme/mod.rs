use cursive::theme::{BorderStyle, Palette, Theme};

pub fn get_theme() -> Theme {
    Theme {
        palette: {
            use cursive::theme::BaseColor::*;
            use cursive::theme::PaletteColor::*;

            let mut palette = Palette::default();

            palette[Background] = Black.dark();
            palette[Shadow] = Black.light();
            palette[View] = Black.dark();

            palette[Primary] = White.dark();
            palette[Secondary] = Black.light();
            palette[Tertiary] = Black.light();

            palette[TitlePrimary] = Cyan.light();
            palette[TitleSecondary] = Yellow.light();

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
