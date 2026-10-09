use super::*;

#[test]
fn a_repeated_message_is_delivered_once() {
    let mut window = ServerMessageWindow::default();
    assert!(window.first_delivery(0));
    assert!(!window.first_delivery(0));
    assert!(window.first_delivery(1));
    assert!(!window.first_delivery(0));
}

#[test]
fn a_number_reused_after_the_window_is_a_new_message() {
    let mut window = ServerMessageWindow::default();
    for sequence in 0..=255u8 {
        assert!(window.first_delivery(sequence), "{sequence}");
    }
    assert!(window.first_delivery(0), "the numbering wrapped");
}

#[test]
fn a_new_login_forgets_the_previous_numbers() {
    let mut window = ServerMessageWindow::default();
    assert!(window.first_delivery(0));
    window.clear();
    assert!(window.first_delivery(0));
}
