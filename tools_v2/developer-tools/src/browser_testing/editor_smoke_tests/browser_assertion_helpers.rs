use super::*;

/// The three panic-capture event taps every smoke installs (`grab` in the Node scripts).
pub(super) async fn attach_panic_capture(page: &Arc<Page>, panics: &Arc<StdMutex<Vec<String>>>) {
    let re = regex::Regex::new("(?i)panic|unreachable|already mapped").unwrap();
    let grab = {
        let panics = Arc::clone(panics);
        move |t: String| {
            if re.is_match(&t) {
                panics.lock().unwrap().push(t.chars().take(300).collect());
            }
        }
    };
    // JS truthiness of `a.value || a.description || ''` per console arg.
    fn arg_text(a: &Value) -> String {
        match &a["value"] {
            Value::String(s) if !s.is_empty() => s.clone(),
            Value::Number(n) if n.as_f64() != Some(0.0) => n.to_string(),
            Value::Bool(true) => "true".to_string(),
            _ => a["description"].as_str().unwrap_or("").to_string(),
        }
    }
    let mut console = page.on_event("Runtime.consoleAPICalled").await;
    let mut log = page.on_event("Log.entryAdded").await;
    let mut exc = page.on_event("Runtime.exceptionThrown").await;
    let g1 = grab.clone();
    tokio::spawn(async move {
        while let Some(e) = console.recv().await {
            let joined = e["args"]
                .as_array()
                .map(|a| a.iter().map(arg_text).collect::<Vec<_>>().join(" "))
                .unwrap_or_default();
            g1(joined);
        }
    });
    let g2 = grab.clone();
    tokio::spawn(async move {
        while let Some(e) = log.recv().await {
            g2(e["entry"]["text"].as_str().unwrap_or("").to_string());
        }
    });
    let g3 = grab;
    tokio::spawn(async move {
        while let Some(e) = exc.recv().await {
            g3(e["exceptionDetails"]["exception"]["description"]
                .as_str()
                .unwrap_or("")
                .to_string());
        }
    });
}

pub(super) async fn eval(page: &Page, expr: &str) -> Result<Value> {
    page.evaluate(expr, false).await
}

pub(super) async fn eval_i64(page: &Page, expr: &str) -> Result<i64> {
    Ok(eval(page, expr).await?.as_i64().unwrap_or(-1))
}

pub(super) async fn eval_str(page: &Page, expr: &str) -> Result<String> {
    Ok(eval(page, expr)
        .await?
        .as_str()
        .unwrap_or_default()
        .to_string())
}

pub(super) async fn eval_bool(page: &Page, expr: &str) -> Result<bool> {
    Ok(eval(page, expr).await?.as_bool() == Some(true))
}

/// `document.querySelector(sel)` centre rect, or None.
pub(super) async fn rect_of(page: &Page, sel: &str) -> Result<Option<(f64, f64)>> {
    let expr = format!(
        "(() => {{ const e = document.querySelector({sel:?});
      if (!e) return 'null'; const b = e.getBoundingClientRect();
      return JSON.stringify([b.left + b.width / 2, b.top + b.height / 2]); }})()"
    );
    let raw = eval_str(page, &expr).await?;
    let v: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
    Ok(v.as_array()
        .and_then(|a| Some((a.first()?.as_f64()?, a.get(1)?.as_f64()?))))
}

pub(super) fn checks_pass(checks: &Map<String, Value>, expected: usize) -> bool {
    checks.len() == expected && checks.values().all(|v| *v == json!(true))
}

pub(super) fn print_verdict(v: &Value) {
    println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
}

pub(super) fn to_code(pass: bool) -> u8 {
    u8::from(!pass)
}

pub(super) fn force_webgl(path: &str) -> String {
    // Idempotent — EDIT_PATH already pins `force=webgl` (T-166), so callers that wrap it must not
    // double-append.
    if path.contains("force=webgl") {
        return path.to_string();
    }
    format!(
        "{path}{}force=webgl",
        if path.contains('?') { '&' } else { '?' }
    )
}
