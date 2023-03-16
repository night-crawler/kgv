use std::sync::{Arc, Mutex};

use cursive::reexports::log::{error, info};
use cursive::CursiveRunnable;

use crate::traits::ext::mutex::MutexExt;
use crate::ui::backend::init_cursive_backend;
use crate::ui::ui_store::UiStore;

pub fn enter_command_handler_loop(
    ui: &mut CursiveRunnable,
    store: Arc<Mutex<UiStore>>,
) -> anyhow::Result<()> {
    loop {
        ui.try_run_with(init_cursive_backend)?;

        let mut store = store.lock_unwrap();
        if let Some(command) = store.interactive_command.take() {
            match command.run() {
                Ok(status) => {
                    if !status.success() {
                        error!("Failed to exec: {}", status);
                    } else {
                        info!("Executed: {}", status);
                    }
                }
                Err(err) => {
                    error!("Error executing command: {}", err);
                }
            }
        } else {
            break;
        }
    }

    Ok(())
}
