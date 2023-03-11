use cursive::{event, Cursive};
use cursive_flexi_logger_view::toggle_flexi_logger_debug_console;

use crate::ui::signals::ToUiSignal;
use crate::util::panics::ResultExt;

pub fn register_hotkeys(ui: &mut Cursive, ui_to_ui_sender: kanal::Sender<ToUiSignal>) {
    ui.add_global_callback('~', toggle_flexi_logger_debug_console);
    ui.add_global_callback(event::Key::F10, |siv| siv.select_menubar());

    {
        let ui_to_ui_sender = ui_to_ui_sender.clone();
        ui.add_global_callback(event::Key::Esc, move |_| {
            ui_to_ui_sender.send(ToUiSignal::EscPressed).unwrap_or_log();
        });
    }
    {
        let ui_to_ui_sender = ui_to_ui_sender.clone();
        ui.add_global_callback(event::Event::CtrlChar('s'), move |_| {
            ui_to_ui_sender.send(ToUiSignal::CtrlSPressed).unwrap();
        });
    }
    {
        let ui_to_ui_sender = ui_to_ui_sender.clone();
        ui.add_global_callback(event::Key::F5, move |_| {
            ui_to_ui_sender.send(ToUiSignal::F5Pressed).unwrap();
        });
    }
    ui.add_global_callback(event::Event::CtrlChar('y'), move |_| {
        ui_to_ui_sender.send(ToUiSignal::CtrlYPressed).unwrap();
    });
}
