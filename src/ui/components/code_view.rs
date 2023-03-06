use cursive::traits::*;
use cursive::utils::markup::StyledString;
use cursive::views::{Dialog, TextView};

pub fn build_code_view(styled_string: StyledString) -> Dialog {
    let tv = TextView::new(styled_string).full_screen().scrollable();
    Dialog::around(tv)
}
