use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use cursive::reexports::log::{error, info};

use crate::reexports::sync::Mutex;
use crate::traits::ext::kanal_sender::KanalSenderExt;
use crate::util::panics::ResultExt;
use crate::util::ui::ago_std;

pub struct DispatchContext<'a, T, S> {
    pub dispatcher: &'a Dispatcher<T, S>,
    pub data: Arc<Mutex<T>>,
    pub sender: kanal::Sender<Box<dyn std::any::Any>>,
    pub receiver: kanal::Receiver<Box<dyn std::any::Any>>,
    pub num_callbacks: Arc<AtomicUsize>,
}

impl<'a, T, S> Clone for DispatchContext<'a, T, S> {
    fn clone(&self) -> Self {
        Self {
            dispatcher: self.dispatcher,
            data: Arc::clone(&self.data),
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
            num_callbacks: Arc::new(AtomicUsize::new(0)),
        }
    }
}

pub trait Dispatch<T, S> {
    fn dispatch(self, context: DispatchContext<T, S>);
}

pub struct Dispatcher<T, S> {
    data: Arc<Mutex<T>>,
    signal_receiver: kanal::Receiver<S>,
    signal_sender: kanal::Sender<S>,
}

impl<T, S> Dispatcher<T, S>
where
    S: Dispatch<T, S> + AsRef<str> + 'static,
    T: Send + Sync + 'static,
{
    pub fn new(
        signal_sender: kanal::Sender<S>,
        signal_receiver: kanal::Receiver<S>,
        data: Arc<Mutex<T>>,
    ) -> Self {
        Self {
            data,
            signal_receiver,
            signal_sender,
        }
    }

    pub fn send_async(&self, signal: S) {
        self.signal_sender.send_unwrap(signal);
    }

    pub fn dispatch_sync(&self, signal: S) {
        let (sender, receiver) = kanal::bounded(16);
        let signal_name = signal.as_ref().to_string();

        let num_callbacks = Arc::new(AtomicUsize::new(0));

        let context = DispatchContext {
            dispatcher: self,
            data: Arc::clone(&self.data),
            sender,
            receiver: receiver.clone(),
            num_callbacks: Arc::clone(&num_callbacks),
        };
        signal.dispatch(context);

        for _ in 0..num_callbacks.load(std::sync::atomic::Ordering::SeqCst) {
            match receiver.recv() {
                Ok(response) => {
                    if let Ok(result) = response.downcast::<Result<(), anyhow::Error>>() {
                        match result.as_ref() {
                            Ok(_) => {}
                            Err(err) => {
                                error!("Signal {signal_name} failed: {err:?}");
                            }
                        }
                    }
                }
                Err(err) => {
                    error!("Signal {signal_name} did not send a signal back: {err:?}");
                }
            }
        }

        if !receiver.is_empty() {
            error!("Received too many signals for {signal_name}");
        }
    }

    pub fn block(&self) {
        for signal in self.signal_receiver.clone() {
            let now = std::time::Instant::now();
            let signal_name = signal.as_ref().to_string();

            self.dispatch_sync(signal);
            info!("Dispatching {signal_name} took {}", ago_std(now.elapsed()));
        }
    }

    fn spawn(self: Arc<Self>, thread_id: usize, thread_prefix: &str) {
        let dispatcher = Arc::clone(&self);
        std::thread::Builder::new()
            .name(format!("{}-{}", thread_prefix, thread_id))
            .spawn(move || {
                dispatcher.block();
            })
            .unwrap_or_log();
    }

    pub fn spawn_n(self: Arc<Self>, n: usize, thread_prefix: &str) {
        for thread_id in 0..n {
            self.clone().spawn(thread_id, thread_prefix);
        }
    }
}
