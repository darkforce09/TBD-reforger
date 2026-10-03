use super::*;
fn snapshot() -> Vec<u8> {
    let nodes=(0..31).map(|i|{let properties=(0..35).map(|p|(format!("property_{p:03}"),json!({"value":0,"native_type":"SCALAR","status":"present","origin":"declared","native_unit":null,"enum_values":[{"name":"x".repeat(5000),"value":0}],"source":{"node_id":format!("opaque #{i}"),"property":format!("property_{p:03}")}}))).collect::<serde_json::Map<_,_>>();
        json!({"node_id":format!("opaque #{i}"),"view":if i==30{"ancestor"}else{"effective"},"class_name":"RepeatedComponent","instance_name":i.to_string(),"native_instance_id":null,"instance_identity_kind":"structural_path","resource_name":"exact","source_addons":[],"ancestor_id":null,"children":[],"declared_properties":[],"properties":properties})}).collect::<Vec<_>>();
    serde_json::to_vec(&json!({"schema_version":2,"resource_id":"native-id","root_node":"opaque #0","nodes":nodes})).unwrap()
}
#[test]
fn equipment_viewer_cards_preserve_order_repeated_instances_and_bounded_continuations() {
    let bytes = snapshot();
    let mut p = ViewerQuery {
        resource_id: Some("native-id".into()),
        ..Default::default()
    };
    let mut seen = Vec::new();
    loop {
        let page = build(&bytes, "generation", &p, &BTreeMap::new()).unwrap();
        assert!(serde_json::to_vec(&page).unwrap().len() <= PAGE_BYTES);
        assert_eq!(page["total"], 30);
        for card in page["items"].as_array().unwrap() {
            seen.push(card["node_id"].as_str().unwrap().to_owned());
            assert_eq!(card["property_count"], 35);
            assert_eq!(card["facts"][0]["value_json"], "0");
            assert_eq!(
                card["property_next_cursor"],
                card["facts"].as_array().unwrap().len().to_string()
            );
        }
        if let Some(next) = page["next_cursor"].as_str() {
            p.cursor = Some(next.into());
        } else {
            break;
        }
    }
    assert_eq!(
        seen,
        (0..30).map(|i| format!("opaque #{i}")).collect::<Vec<_>>()
    );
    p.node_id = Some("opaque #25".into());
    p.cursor = None;
    let page = build(&bytes, "generation", &p, &BTreeMap::new()).unwrap();
    assert_eq!(page["start_index"], 24);
    assert_eq!(page["items"][1]["node_id"], "opaque #25");
    p.node_id = None;
    p.q = Some("property_034".into());
    p.kind = Some("contents".into());
    let page = build(&bytes, "generation", &p, &BTreeMap::new()).unwrap();
    assert_eq!(page["total"], 30);
    assert_eq!(page["items"][0]["facts"], json!([]));
}
