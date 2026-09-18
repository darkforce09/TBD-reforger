/// `GET /me` and `GET /me/link/status` must both derive their linked flag from
/// `arma_id_is_linked`, not `Option::is_some()`. Whitespace-only `arma_id` is not linked,
/// and `is_some()` would report it as linked to the SPA while every trimmed resolve path
/// finds nothing.
///
/// RED: either site reverts to `u.arma_id.is_some()`.
/// GREEN: both sites call `arma_id_is_linked(&u.arma_id)`.
#[test]
fn get_me_and_link_status_use_arma_id_is_linked() {
    const GET_ME_SRC: &str = include_str!("../member_profile.rs");
    const LINK_STATUS_SRC: &str = include_str!("../arma_link_codes.rs");

    let production = |src: &str| {
        src.split("#[cfg(test)]")
            .next()
            .expect("split always yields a first slice")
            .to_string()
    };
    let both = format!("{}{}", production(GET_ME_SRC), production(LINK_STATUS_SRC));

    assert!(
        !both.contains("arma_id.is_some()"),
        "neither handler may use arma_id.is_some() for a linked flag — \
         whitespace rows would report linked"
    );
    assert!(
        both.matches("arma_id_is_linked(&u.arma_id)").count() >= 2,
        "get_me and link_status must both call arma_id_is_linked(&u.arma_id)"
    );
}
