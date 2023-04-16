use std::sync::Arc;

use cursive::{event, Cursive};

use crate::traits::ext::kanal_sender::KanalSenderExt;
use crate::ui::signals::InterUiSignal;

pub fn register_hotkeys(ui: &mut Cursive, ui_to_ui_sender: kanal::Sender<InterUiSignal>) {
    ui.add_global_callback(event::Key::F10, |siv| siv.select_menubar());

    // I personally hate FnMut and this stuff here
    let hotkeys: Vec<(event::Event, Arc<dyn Fn() -> InterUiSignal>)> = vec![
        (
            event::Event::from('~'),
            Arc::new(|| InterUiSignal::ShowDebugLog),
        ),
        (
            event::Event::from(event::Key::Esc),
            Arc::new(|| InterUiSignal::EscPressed),
        ),
        (
            event::Event::CtrlChar('s'),
            Arc::new(|| InterUiSignal::CtrlSPressed),
        ),
        (
            event::Event::AltChar('='),
            Arc::new(|| InterUiSignal::AltPlusPressed),
        ),
        (
            event::Event::CtrlChar('p'),
            Arc::new(|| InterUiSignal::CtrlPPressed),
        ),
        (
            event::Event::from(event::Key::F5),
            Arc::new(|| InterUiSignal::F5Pressed),
        ),
        (
            event::Event::CtrlChar('y'),
            Arc::new(|| InterUiSignal::CtrlYPressed),
        ),
        (
            // how is it 7?
            event::Event::CtrlChar('7'),
            Arc::new(|| InterUiSignal::CtrlSlashPressed),
        ),
        (
            event::Event::CtrlChar('k'),
            Arc::new(|| InterUiSignal::CtrlKPressed),
        ),
        (
            event::Event::CtrlChar('l'),
            Arc::new(|| InterUiSignal::CtrlLPressed),
        ),
    ];

    hotkeys.into_iter().for_each(|(event, signal)| {
        register(ui, ui_to_ui_sender.clone(), event, signal);
    })
}

fn register(
    ui: &mut Cursive,
    ui_to_ui_sender: kanal::Sender<InterUiSignal>,
    event: event::Event,
    signal: Arc<dyn Fn() -> InterUiSignal>,
) {
    ui.add_global_callback(event, move |_| {
        ui_to_ui_sender.send_unwrap(signal());
    });
}
