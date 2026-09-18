use super::*;
use std::io::Cursor;

pub(crate) const META: &str = r#"{"v":"tbd-voxel-dump/1","slug":"t","resource":"r","origin":[0,0,0],"cell":0.1,"dims":[10,10,10],"span":[1,1,1],"bboxMin":[0,0,0],"bboxMax":[1,1,1],"rootYawDeg":0,"excluded":{"doors":0,"glass":0,"furniture":0},"tick":1}"#;

fn parse_str(s: &str) -> Result<VoxelDump> {
    parse_reader(Cursor::new(s.to_string()), "test")
}

#[test]
fn round_trip_minimal() {
    let s = format!(
        "{META}\n[\"x+\",1,2,[0.25,0.61]]\n[\"x-\",1,2,[0.66,0.30]]\n{{\"furn\":{{\"name\":\"w\",\"res\":\"r\",\"pos\":[1,0,1],\"worldYawDeg\":90,\"size\":[1,2,1],\"boundsMinY\":0}}}}\n{{\"end\":{{\"lines\":3,\"ms\":5}}}}\n"
    );
    let d = parse_str(&s).unwrap();
    assert_eq!(d.x_pos[&(1, 2)], vec![0.25, 0.61]);
    assert_eq!(d.x_neg[&(1, 2)], vec![0.66, 0.30]);
    assert_eq!(d.furniture.len(), 1);
    assert_eq!(d.truncated, 0);
}

#[test]
fn missing_end_line_is_truncation() {
    let s = format!("{META}\n[\"x+\",1,2,[0.25]]\n");
    assert!(
        parse_str(&s)
            .unwrap_err()
            .to_string()
            .contains("no end line")
    );
}

#[test]
fn wrong_line_count_fails() {
    let s = format!("{META}\n[\"x+\",1,2,[0.25]]\n{{\"end\":{{\"lines\":7,\"ms\":5}}}}\n");
    assert!(
        parse_str(&s)
            .unwrap_err()
            .to_string()
            .contains("declares 7")
    );
}

#[test]
fn march_order_violation_fails() {
    let s = format!("{META}\n[\"x+\",1,2,[0.61,0.25]]\n{{\"end\":{{\"lines\":1,\"ms\":5}}}}\n");
    assert!(
        parse_str(&s)
            .unwrap_err()
            .to_string()
            .contains("march order")
    );
}

#[test]
fn descending_required_for_minus_runs() {
    let s = format!("{META}\n[\"y-\",1,2,[0.25,0.61]]\n{{\"end\":{{\"lines\":1,\"ms\":5}}}}\n");
    assert!(
        parse_str(&s)
            .unwrap_err()
            .to_string()
            .contains("march order")
    );
}
