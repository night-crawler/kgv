use std::sync::Arc;

use crate::reexports::RwLock;
use cursive::direction::Direction;
use cursive::event::{AnyCb, Event, EventResult};
use cursive::view::{CannotFocus, Selector, ViewNotFound};
use cursive::{Printer, Rect, Vec2, View};

pub struct ViewWithMeta<T> {
    pub inner: Box<dyn View>,
    pub meta: Arc<RwLock<T>>,
}

impl<T> ViewWithMeta<T> {
    pub fn new<I: View>(inner: I, meta: T) -> Self {
        Self {
            inner: Box::new(inner),
            meta: Arc::new(RwLock::new(meta)),
        }
    }
}

impl<T> View for ViewWithMeta<T>
where
    T: 'static,
{
    fn draw(&self, printer: &Printer) {
        self.inner.draw(printer)
    }

    fn layout(&mut self, size: Vec2) {
        self.inner.layout(size)
    }

    fn needs_relayout(&self) -> bool {
        self.inner.needs_relayout()
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        self.inner.required_size(constraint)
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        self.inner.on_event(event)
    }

    fn call_on_any(&mut self, selector: &Selector, callback: AnyCb) {
        self.inner.call_on_any(selector, callback)
    }

    fn focus_view(&mut self, selector: &Selector) -> Result<EventResult, ViewNotFound> {
        self.inner.focus_view(selector)
    }

    fn take_focus(&mut self, source: Direction) -> Result<EventResult, CannotFocus> {
        self.inner.take_focus(source)
    }

    fn important_area(&self, view_size: Vec2) -> Rect {
        self.inner.important_area(view_size)
    }

    fn type_name(&self) -> &'static str {
        self.inner.type_name()
    }
}
