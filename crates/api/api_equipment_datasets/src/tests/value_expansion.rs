use super::values;
use serde_json::value::RawValue;

#[test]
fn equipment_viewer_array_ranges_preserve_order_and_numeric_text() {
    let values = (0..795)
        .map(|i| format!("{i}.000000e-3"))
        .collect::<Vec<_>>();
    let raw = RawValue::from_string(format!("[{}]", values.join(","))).unwrap();
    let mut actual = Vec::new();
    for start in (0..795).step_by(100) {
        let (count, rows) = values::expand(&raw, "", start).unwrap();
        assert_eq!(count, 795);
        for row in rows {
            actual.push(row["value_json"].as_str().unwrap().to_owned());
        }
    }
    assert_eq!(actual, values);
}

#[test]
fn equipment_viewer_long_unicode_text_and_empty_values_are_complete() {
    let original = "A🦀é".repeat(5000);
    let raw = RawValue::from_string(serde_json::to_string(&original).unwrap()).unwrap();
    let (_, rows) = values::expand(&raw, "", 0).unwrap();
    let rebuilt = rows
        .iter()
        .map(|r| serde_json::from_str::<String>(r["value_json"].as_str().unwrap()).unwrap())
        .collect::<String>();
    assert_eq!(rebuilt, original);
    for text in ["0", "false", "[]", "{}", "null", "\"\""] {
        let raw = RawValue::from_string(text.to_owned()).unwrap();
        let (count, rows) = values::expand(&raw, "", 0).unwrap();
        assert_eq!(count, 1);
        assert_eq!(rows[0]["value_json"], text);
    }
}
