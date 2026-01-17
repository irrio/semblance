use std::{cell::RefCell, collections::VecDeque};

use sdl2::keyboard::Keycode;

pub struct QueuedKeyEvent {
    pub pressed: bool,
    pub keycode: Keycode,
}

thread_local! {
    static KEY_QUEUE: RefCell<VecDeque<QueuedKeyEvent>> = RefCell::new(VecDeque::new());
}

pub fn enqueue_key(ev: QueuedKeyEvent) {
    KEY_QUEUE.with_borrow_mut(|q| q.push_back(ev))
}

pub fn dequeue_key() -> Option<QueuedKeyEvent> {
    KEY_QUEUE.with_borrow_mut(|q| q.pop_front())
}
