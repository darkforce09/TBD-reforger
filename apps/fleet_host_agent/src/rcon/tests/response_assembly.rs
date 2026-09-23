use super::*;

fn fragment(total: u8, index: u8, part: &[u8]) -> ResponseBody {
    ResponseBody::Fragment {
        total,
        index,
        part: part.to_vec(),
    }
}

#[test]
fn a_whole_response_is_complete_at_once() {
    let mut assembly = ResponseAssembly::default();
    assert_eq!(
        assembly.accept(ResponseBody::Whole(b"done".to_vec())),
        Some(b"done".to_vec())
    );
}

#[test]
fn parts_in_order_complete_the_response() {
    let mut assembly = ResponseAssembly::default();
    assert_eq!(assembly.accept(fragment(3, 0, b"Play")), None);
    assert_eq!(assembly.accept(fragment(3, 1, b"ers on ")), None);
    assert_eq!(
        assembly.accept(fragment(3, 2, b"server")),
        Some(b"Players on server".to_vec())
    );
}

#[test]
fn parts_out_of_order_are_joined_in_index_order() {
    let mut assembly = ResponseAssembly::default();
    assert_eq!(assembly.accept(fragment(3, 2, b"C")), None);
    assert_eq!(assembly.accept(fragment(3, 0, b"A")), None);
    assert_eq!(assembly.accept(fragment(3, 1, b"B")), Some(b"ABC".to_vec()));
}

#[test]
fn a_duplicated_part_keeps_its_first_copy() {
    let mut assembly = ResponseAssembly::default();
    assert_eq!(assembly.accept(fragment(2, 0, b"first")), None);
    assert_eq!(assembly.accept(fragment(2, 0, b"other")), None);
    assert_eq!(
        assembly.accept(fragment(2, 1, b"-last")),
        Some(b"first-last".to_vec())
    );
}

#[test]
fn a_part_with_another_number_of_parts_is_ignored() {
    let mut assembly = ResponseAssembly::default();
    assert_eq!(assembly.accept(fragment(2, 0, b"A")), None);
    assert_eq!(assembly.accept(fragment(3, 1, b"stale")), None);
    assert_eq!(assembly.accept(fragment(2, 1, b"B")), Some(b"AB".to_vec()));
}

#[test]
fn a_character_split_across_parts_is_rejoined_before_decoding() {
    let text = "Jérôme";
    let bytes = text.as_bytes();
    // Byte 2 is the second byte of 'é'.
    let mut assembly = ResponseAssembly::default();
    assert_eq!(assembly.accept(fragment(2, 1, &bytes[2..])), None);
    let joined = assembly.accept(fragment(2, 0, &bytes[..2])).unwrap();
    assert_eq!(String::from_utf8(joined).unwrap(), text);
}
