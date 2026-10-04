//! The draft-persist hook: the unregistered default, one run per call, and replacement.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use mission_model::ids::MissionId;

use super::{EditPersistDocument, register_edit_persist_hook, schedule_edit_persist};

fn empty_document() -> EditPersistDocument {
    Rc::new(RefCell::new(None))
}

#[test]
fn an_unregistered_hook_arms_no_draft_write() {
    assert!(
        !schedule_edit_persist(empty_document(), &MissionId::new("mission-a")),
        "with nothing registered the edit tail arms no draft write"
    );
}

#[test]
fn the_registered_hook_runs_once_per_call_with_the_mission_id() {
    let seen = Rc::new(RefCell::new(Vec::<String>::new()));
    let sink = Rc::clone(&seen);
    register_edit_persist_hook(Rc::new(move |_doc, id: &MissionId| {
        sink.borrow_mut().push(id.to_string());
    }));
    assert!(schedule_edit_persist(
        empty_document(),
        &MissionId::new("mission-a")
    ));
    assert_eq!(*seen.borrow(), ["mission-a"]);
    assert!(schedule_edit_persist(
        empty_document(),
        &MissionId::new("mission-b")
    ));
    assert_eq!(*seen.borrow(), ["mission-a", "mission-b"]);
}

#[test]
fn the_hook_receives_the_document_it_was_given() {
    let doc = empty_document();
    let expected = Rc::clone(&doc);
    let same = Rc::new(Cell::new(false));
    let flag = Rc::clone(&same);
    register_edit_persist_hook(Rc::new(move |got, _id: &MissionId| {
        flag.set(Rc::ptr_eq(&got, &expected));
    }));
    assert!(schedule_edit_persist(doc, &MissionId::new("mission-a")));
    assert!(
        same.get(),
        "the hook receives the caller's document cell, not a copy"
    );
}

#[test]
fn a_second_registration_replaces_the_first() {
    let first = Rc::new(Cell::new(0u32));
    let second = Rc::new(Cell::new(0u32));
    let first_count = Rc::clone(&first);
    register_edit_persist_hook(Rc::new(move |_doc, _id: &MissionId| {
        first_count.set(first_count.get() + 1);
    }));
    let second_count = Rc::clone(&second);
    register_edit_persist_hook(Rc::new(move |_doc, _id: &MissionId| {
        second_count.set(second_count.get() + 1);
    }));
    assert!(schedule_edit_persist(
        empty_document(),
        &MissionId::new("mission-a")
    ));
    assert_eq!(
        (first.get(), second.get()),
        (0, 1),
        "a remount's registration replaces the earlier one and never stacks"
    );
}
