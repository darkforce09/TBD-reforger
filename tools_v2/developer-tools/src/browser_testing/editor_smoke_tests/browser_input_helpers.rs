use super::*;

pub(super) async fn mouse(page: &Page, ev: &str, x: f64, y: f64, extra: Value) -> Result<()> {
    let mut params = json!({ "type": ev, "x": x, "y": y });
    if let (Value::Object(b), Value::Object(e)) = (&mut params, extra) {
        for (k, v) in e {
            b.insert(k, v);
        }
    }
    page.send("Input.dispatchMouseEvent", params).await?;
    Ok(())
}

/// Trusted LMB drag with intermediate `button:none` moves (the pan/marquee smoke shape).
pub(super) async fn drag(page: &Page, x0: f64, y0: f64, x1: f64, y1: f64) -> Result<()> {
    mouse(
        page,
        "mousePressed",
        x0,
        y0,
        json!({ "button": "left", "buttons": 1, "clickCount": 1 }),
    )
    .await?;
    let steps = 6.0;
    for i in 1..=6 {
        let f = f64::from(i) / steps;
        mouse(
            page,
            "mouseMoved",
            x0 + (x1 - x0) * f,
            y0 + (y1 - y0) * f,
            json!({ "button": "none", "buttons": 1 }),
        )
        .await?;
    }
    mouse(
        page,
        "mouseReleased",
        x1,
        y1,
        json!({ "button": "left", "buttons": 0, "clickCount": 1 }),
    )
    .await?;
    Ok(())
}

pub(super) async fn click_at(page: &Page, x: f64, y: f64, ctrl: bool) -> Result<()> {
    let m = if ctrl {
        json!({ "modifiers": 2 })
    } else {
        json!({})
    };
    let mut down = json!({ "button": "left", "buttons": 1, "clickCount": 1 });
    let mut up = json!({ "button": "left", "buttons": 0, "clickCount": 1 });
    for v in [&mut down, &mut up] {
        if let (Value::Object(b), Some(e)) = (v, m.as_object()) {
            for (k, vv) in e {
                b.insert(k.clone(), vv.clone());
            }
        }
    }
    mouse(page, "mousePressed", x, y, down).await?;
    mouse(page, "mouseReleased", x, y, up).await?;
    Ok(())
}

/// One trusted key chord: rawKeyDown + keyUp ONLY (T-159.22.1 — keyDown would double-fire).
pub(super) async fn key_chord(
    page: &Page,
    key: &str,
    code: &str,
    modifiers: u32,
    vk: u32,
) -> Result<()> {
    for ev in ["rawKeyDown", "keyUp"] {
        page.send(
            "Input.dispatchKeyEvent",
            json!({ "type": ev, "key": key, "code": code, "modifiers": modifiers,
                    "windowsVirtualKeyCode": vk, "nativeVirtualKeyCode": vk }),
        )
        .await?;
    }
    Ok(())
}

pub(super) async fn dbl_click(page: &Page, x: f64, y: f64) -> Result<()> {
    // down/up ×2, clickCount 2 on the second pair (the Node smokes' shape via page.dispatchMouse).
    for (ev, cc) in [
        ("mousePressed", 1),
        ("mouseReleased", 1),
        ("mousePressed", 2),
        ("mouseReleased", 2),
    ] {
        page.send(
            "Input.dispatchMouseEvent",
            json!({ "type": ev, "x": x, "y": y, "button": "left", "clickCount": cc }),
        )
        .await?;
    }
    Ok(())
}

pub(super) async fn click_selector(page: &Page, sel: &str) -> Result<bool> {
    match rect_of(page, sel).await? {
        Some((x, y)) => {
            click_at(page, x, y, false).await?;
            Ok(true)
        }
        None => Ok(false),
    }
}
