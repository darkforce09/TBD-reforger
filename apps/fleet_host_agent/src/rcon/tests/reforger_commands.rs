use super::*;

const LISTING: &str = "Players on server:\r\n\
    [Player#] ; [Player UID] ; [Player Name]\r\n\
    ------------------------------------------\r\n\
    0 ; 3d8f0e52-6c5a-4e0b-9a55-0c2b1d7e4f10 ; Rhodes\r\n\
    12 ; b6955d91-4749-4cdb-9a51-e69f630ec435 ; Jérôme Lefèvre\r\n";

#[test]
fn player_rows_are_read_under_the_listing_header() {
    let listing = parse_player_listing(LISTING);
    assert_eq!(
        listing.players,
        vec![
            ListedPlayer {
                player_id: 0,
                arma_id: "3d8f0e52-6c5a-4e0b-9a55-0c2b1d7e4f10".to_owned(),
                name: "Rhodes".to_owned(),
            },
            ListedPlayer {
                player_id: 12,
                arma_id: "b6955d91-4749-4cdb-9a51-e69f630ec435".to_owned(),
                name: "Jérôme Lefèvre".to_owned(),
            },
        ]
    );
    assert_eq!(listing.raw_lines, None, "every line was understood");
}

#[test]
fn an_empty_server_lists_no_players() {
    for response in [
        "",
        "Players on server:\n[Player#] ; [Player UID] ; [Player Name]\n",
    ] {
        let listing = parse_player_listing(response);
        assert_eq!(listing, PlayerListing::default(), "{response:?}");
    }
}

#[test]
fn a_partly_understood_response_keeps_every_raw_line() {
    let response = "Players on server:\n\
        4 ; 3d8f0e52-6c5a-4e0b-9a55-0c2b1d7e4f10 ; Rhodes\n\
        5 ; b6955d91-4749-4cdb-9a51-e69f630ec435 ; Name ; with separator\n\
        (2 players in total)\n";
    let listing = parse_player_listing(response);
    assert_eq!(listing.players.len(), 1);
    assert_eq!(listing.players[0].player_id, 4);
    assert_eq!(
        listing.raw_lines.unwrap(),
        vec![
            "Players on server:",
            "4 ; 3d8f0e52-6c5a-4e0b-9a55-0c2b1d7e4f10 ; Rhodes",
            "5 ; b6955d91-4749-4cdb-9a51-e69f630ec435 ; Name ; with separator",
            "(2 players in total)",
        ]
    );
}

#[test]
fn rows_without_a_number_identity_or_name_are_not_players() {
    for line in [
        "x ; 3d8f0e52-6c5a-4e0b-9a55-0c2b1d7e4f10 ; Rhodes",
        "-1 ; 3d8f0e52-6c5a-4e0b-9a55-0c2b1d7e4f10 ; Rhodes",
        "3 ;  ; Rhodes",
        "3 ; not an identity ; Rhodes",
        "3 ; 3d8f0e52-6c5a-4e0b-9a55-0c2b1d7e4f10 ;",
        "3 ; 3d8f0e52-6c5a-4e0b-9a55-0c2b1d7e4f10",
    ] {
        let listing = parse_player_listing(line);
        assert!(listing.players.is_empty(), "{line:?}");
        assert_eq!(listing.raw_lines, Some(vec![line.trim().to_owned()]));
    }
}

#[test]
fn the_outcome_names_each_player_by_identity() {
    let outcome = parse_player_listing(LISTING).into_outcome();
    assert_eq!(
        Value::Object(outcome),
        json!({
            "players": [
                {"player_id": 0, "arma_id": "3d8f0e52-6c5a-4e0b-9a55-0c2b1d7e4f10", "name": "Rhodes"},
                {"player_id": 12, "arma_id": "b6955d91-4749-4cdb-9a51-e69f630ec435", "name": "Jérôme Lefèvre"},
            ]
        })
    );
    let partial = parse_player_listing("Unknown response").into_outcome();
    assert_eq!(
        Value::Object(partial),
        json!({"players": [], "raw_lines": ["Unknown response"]})
    );
}
