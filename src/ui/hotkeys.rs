use std::sync::Arc;

use cursive::{event, Cursive};

use crate::traits::ext::kanal_sender::KanalSenderExt;
use crate::ui::signals::ToUiSignal;

pub fn register_hotkeys(ui: &mut Cursive, ui_to_ui_sender: kanal::Sender<ToUiSignal>) {
    ui.add_global_callback(event::Key::F10, |siv| siv.select_menubar());

    // I personally hate FnMut and this stuff here
    let hotkeys: Vec<(event::Event, Arc<dyn Fn() -> ToUiSignal>)> = vec![
        (
            event::Event::from('~'),
            Arc::new(|| ToUiSignal::ShowDebugLog),
        ),
        (
            event::Event::from(event::Key::Esc),
            Arc::new(|| ToUiSignal::EscPressed),
        ),
        (
            event::Event::CtrlChar('s'),
            Arc::new(|| ToUiSignal::CtrlSPressed),
        ),
        (
            event::Event::AltChar('='),
            Arc::new(|| ToUiSignal::AltPlusPressed),
        ),
        (
            event::Event::CtrlChar('p'),
            Arc::new(|| ToUiSignal::CtrlPPressed),
        ),
        (
            event::Event::from(event::Key::F5),
            Arc::new(|| ToUiSignal::F5Pressed),
        ),
        (
            event::Event::CtrlChar('y'),
            Arc::new(|| ToUiSignal::CtrlYPressed),
        ),
        (
            // how is it 7?
            event::Event::CtrlChar('7'),
            Arc::new(|| ToUiSignal::CtrlSlashPressed),
        ),
        (
            event::Event::CtrlChar('k'),
            Arc::new(|| ToUiSignal::CtrlKPressed),
        ),
        (
            event::Event::CtrlChar('l'),
            Arc::new(|| ToUiSignal::CtrlLPressed),
        ),
    ];

    hotkeys.into_iter().for_each(|(event, signal)| {
        register(ui, ui_to_ui_sender.clone(), event, signal);
    })
}

fn register(
    ui: &mut Cursive,
    ui_to_ui_sender: kanal::Sender<ToUiSignal>,
    event: event::Event,
    signal: Arc<dyn Fn() -> ToUiSignal>,
) {
    ui.add_global_callback(event, move |_| {
        ui_to_ui_sender.send_unwrap(signal());
    });
}
