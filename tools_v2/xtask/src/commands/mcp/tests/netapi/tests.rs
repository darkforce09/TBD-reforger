use super::*;

#[test]
fn request_and_response_framing_round_trip() {
    let req = encode_request(
        "TbdXtask",
        "EMCP_WB_TbdBlueprint",
        &json!({"action": "recon", "maxEntities": 600}),
    );
    assert_eq!(&req[0..4], &1i32.to_le_bytes());
    let (client, next) = read_pascal(&req, 4).unwrap();
    assert_eq!(client, "TbdXtask");
    let (ct, next) = read_pascal(&req, next).unwrap();
    assert_eq!(ct, "JsonRPC");
    let (payload, end) = read_pascal(&req, next).unwrap();
    assert_eq!(end, req.len());
    let v: Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(v["APIFunc"], "EMCP_WB_TbdBlueprint");
    assert_eq!(v["action"], "recon");
    assert_eq!(v["maxEntities"], 600);

    let mut ok = Vec::new();
    pascal(&mut ok, "Ok");
    pascal(&mut ok, r#"{"status":"ok","message":"OK 181 entities"}"#);
    let v = decode_response(&ok).unwrap();
    assert_eq!(v["message"], "OK 181 entities");
    let mut bare = Vec::new();
    pascal(&mut bare, "Ok");
    assert_eq!(decode_response(&bare).unwrap(), json!({}));
    assert_eq!(decode_response(&[]).unwrap(), json!({}));
    let mut err = Vec::new();
    pascal(&mut err, "Unknown APIFunc");
    assert!(
        decode_response(&err)
            .unwrap_err()
            .to_string()
            .contains("Unknown APIFunc")
    );
    assert!(decode_response(&[5, 0, 0, 0, b'O']).is_err());
}
