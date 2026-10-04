//! The context-menu opener: the unregistered default, one run per call, and replacement.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::{ContextMenuRequest, open_context_menu, register_context_menu_opener};

fn right_click() -> ContextMenuRequest {
    ContextMenuRequest {
        client_x: 120.0,
        client_y: 340.5,
        hit: Some("s1".to_string()),
        selection: vec!["s1".to_string(), "s2".to_string()],
        world: (6_400.0, -12.25),
    }
}

#[test]
fn an_unregistered_opener_opens_no_menu() {
    assert!(
        !open_context_menu(right_click()),
        "with nothing registered a right click opens no menu"
    );
}

#[test]
fn the_registered_opener_runs_once_per_call_with_the_request() {
    let seen = Rc::new(RefCell::new(Vec::<ContextMenuRequest>::new()));
    let sink = Rc::clone(&seen);
    register_context_menu_opener(Rc::new(move |request| sink.borrow_mut().push(request)));
    assert!(open_context_menu(right_click()));
    assert_eq!(*seen.borrow(), [right_click()]);
    let empty_ground = ContextMenuRequest {
        hit: None,
        selection: Vec::new(),
        ..right_click()
    };
    assert!(open_context_menu(empty_ground.clone()));
    assert_eq!(*seen.borrow(), [right_click(), empty_ground]);
}

#[test]
fn a_second_registration_replaces_the_first() {
    let first = Rc::new(Cell::new(0u32));
    let second = Rc::new(Cell::new(0u32));
    let first_count = Rc::clone(&first);
    register_context_menu_opener(Rc::new(move |_request| {
        first_count.set(first_count.get() + 1)
    }));
    let second_count = Rc::clone(&second);
    register_context_menu_opener(Rc::new(move |_request| {
        second_count.set(second_count.get() + 1);
    }));
    assert!(open_context_menu(right_click()));
    assert_eq!(
        (first.get(), second.get()),
        (0, 1),
        "a remount's registration replaces the earlier one and never stacks"
    );
}
