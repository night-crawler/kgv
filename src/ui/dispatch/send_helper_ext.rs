use std::fmt::Debug;

use cursive::{Cursive, View};

use crate::traits::ext::cursive::SivExt;
use crate::traits::ext::kanal_sender::KanalSenderExt;
use crate::traits::ext::mutex::MutexExt;
use crate::ui::dispatcher::DispatchContext;
use crate::ui::ui_store::UiStore;
use crate::util::panics::ResultExt;

pub(crate) trait DispatchContextSendHelperExt {
    fn call_on_name<V, F, R>(&self, name: &str, callback: F)
    where
        V: View,
        F: Send + FnOnce(&mut V) -> R + 'static,
        R: 'static;

    fn call_on_name_wait<V, F, R>(&self, name: &str, callback: F) -> R
    where
        V: View,
        F: Send + 'static + FnOnce(&mut V) -> R + Send,
        R: 'static + Debug;

    fn send<F>(&self, callback: F)
    where
        F: FnOnce(&mut Cursive) + Send + 'static;

    fn send_wait<F, R>(&self, callback: F) -> R
    where
        F: FnOnce(&mut Cursive) -> R + Send + 'static,
        R: 'static;
}

impl<'a, S> DispatchContextSendHelperExt for DispatchContext<'a, UiStore, S> {
    fn call_on_name<V, F, R>(&self, name: &str, callback: F)
    where
        V: View,
        F: Send + 'static + FnOnce(&mut V) -> R,
        R: 'static,
    {
        let sink = self.data.lock_unwrap().sink.clone();
        self.num_callbacks
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        sink.channel_call_on_name(self.sender.clone(), name, callback);
    }

    fn call_on_name_wait<V, F, R>(&self, name: &str, callback: F) -> R
    where
        V: View,
        F: Send + 'static + FnOnce(&mut V) -> R,
        R: 'static + Debug,
    {
        let sink = self.data.lock_unwrap().sink.clone();
        let sender = self.sender.clone();

        sink.call_on_name(name, move |view: &mut V| {
            let result = callback(view);
            sender.send_unwrap(Box::new(result));
        });

        let result = self.receiver.recv().unwrap_or_log();
        match result.downcast::<R>() {
            Ok(result) => *result,
            Err(err) => panic!("Failed to downcast result: {:?}", err),
        }
    }

    fn send<F>(&self, callback: F)
    where
        F: FnOnce(&mut Cursive) + Send + 'static,
    {
        let sink = self.data.lock_unwrap().sink.clone();
        self.num_callbacks
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        sink.channel_box(self.sender.clone(), callback);
    }

    fn send_wait<F, R>(&self, callback: F) -> R
    where
        F: FnOnce(&mut Cursive) -> R + Send + 'static,
        R: 'static,
    {
        let sink = self.data.lock_unwrap().sink.clone();
        let sender = self.sender.clone();
        sink.send_box(move |siv| {
            let result = callback(siv);
            sender.send_unwrap(Box::new(result));
        });
        let result = self.receiver.recv().unwrap_or_log();
        match result.downcast::<R>() {
            Ok(result) => *result,
            Err(err) => panic!("Failed to downcast result: {:?}", err),
        }
    }
}
