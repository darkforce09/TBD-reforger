//! The mortar calculator page as the offline gate drives it over the DevTools protocol.
//!
//! **Role:** waits on the offline pack state the page mirrors onto `<html>`, types into the
//! calculator's fields the way a person's input events would, clicks the map, presses Calculate
//! and reads the solution tables back.
//! **Position:** between `super::run`, which orders the steps, and the page's DOM hooks
//! (`data-offline-state`, `data-offline-optional`, `data-mortar-input`, `data-mortar-position`, `data-mortar-map-state`,
//! `data-mortar-solution`, `data-mortar-battery`, `data-mortar-gun`).
//! **Signals & state:** none of its own; every call acts on the given [`Page`].
//! **Invariants:** a value is set through the element's own `value` setter followed by the
//! event the page listens to (`input` for text and number fields, `change` for pickers); a
//! terminal pack state other than `ready` ends the wait with that state named; a `ready` pack
//! reports whether its optional icon font is cached (`complete` or `missing`), and a `ready`
//! without that report fails; every wait is bounded.

use std::time::{Duration, Instant};

use anyhow::{Result, anyhow, bail};
use serde_json::{Value, json};

use super::expected_solution::SolutionTables;
use super::map_pixels::CssRect;
use chrome_devtools_protocol::{Page, sleep_ms};

/// The pack states after which the pack never becomes `ready` in this page lifetime.
const TERMINAL_STATES: [&str; 4] = ["incomplete", "quota-short", "unsupported", "failed"];

/// The page script that sets one field by selector and fires the event the page listens to.
const SET_FIELD: &str = r#"((selector, index, value) => {
  const el = document.querySelectorAll(selector)[index];
  if (!el) return 'missing ' + selector + '[' + index + ']';
  const proto = el.tagName === 'SELECT' ? HTMLSelectElement.prototype : HTMLInputElement.prototype;
  Object.getOwnPropertyDescriptor(proto, 'value').set.call(el, value);
  el.dispatchEvent(new Event(el.tagName === 'SELECT' ? 'change' : 'input', { bubbles: true }));
  return el.value === value ? 'ok' : 'value did not stick: ' + el.value;
})"#;

/// The page script that reads the battery table and every gun's charge table.
const READ_TABLES: &str = r#"(() => {
  const solution = document.querySelector('[data-mortar-solution]');
  if (!solution) {
    const problems = document.querySelector('[data-mortar-problems]');
    return { problems: problems ? problems.innerText : null };
  }
  const rows = (section) => Array.from(section.querySelectorAll('tbody tr'))
    .map((tr) => Array.from(tr.children).map((td) => td.textContent.trim()));
  return {
    tables: {
      battery: rows(solution.querySelector('[data-mortar-battery]')),
      guns: Array.from(solution.querySelectorAll('[data-mortar-gun]')).map((section) => ({
        heading: section.querySelector('h2').textContent.trim(),
        rows: rows(section),
        laid: Array.from(section.querySelectorAll('tbody tr'))
          .map((tr) => tr.classList.contains('font-semibold')),
      })),
    },
  };
})()"#;

/// The optional-file report of a `ready` pack: `complete` or `missing`.
///
/// # Errors
///
/// The page set no `data-offline-optional`, or a value other than the two it documents.
pub fn optional_files_report(value: Option<&str>) -> Result<&str> {
    match value {
        Some(report @ ("complete" | "missing")) => Ok(report),
        Some(other) => bail!("the offline pack is ready with data-offline-optional = {other:?}"),
        None => bail!("the offline pack is ready without data-offline-optional"),
    }
}

/// Waits until the offline pack is `ready`, printing progress and the optional icon font's
/// coverage; refuses on a terminal state.
///
/// # Errors
///
/// A terminal state, a `ready` without a valid [`optional_files_report`], or no `ready` within
/// `budget`.
pub async fn wait_for_pack_ready(page: &Page, budget: Duration) -> Result<()> {
    let started = Instant::now();
    let mut last_progress = String::new();
    loop {
        let v = page
            .evaluate(
                "({ state: document.documentElement.getAttribute('data-offline-state'), \
                 progress: document.documentElement.getAttribute('data-offline-progress'), \
                 optional: document.documentElement.getAttribute('data-offline-optional') })",
                false,
            )
            .await?;
        let state = v["state"].as_str().unwrap_or("absent");
        let progress = v["progress"].as_str().unwrap_or("").to_string();
        if state == "ready" {
            let optional = optional_files_report(v["optional"].as_str())?;
            println!("mortar-offline: pack ready at {progress}%, optional icon font {optional}");
            return Ok(());
        }
        if TERMINAL_STATES.contains(&state) {
            bail!("the offline pack ended {state} at {progress}%");
        }
        if progress != last_progress {
            println!("mortar-offline: pack {state} {progress}%");
            last_progress = progress;
        }
        if started.elapsed() > budget {
            bail!("the offline pack is still {state} at {last_progress}% after {budget:?}");
        }
        sleep_ms(1000).await;
    }
}

/// Polls `expression` until it is `true`.
///
/// # Errors
///
/// `what` when it stays false for `seconds`.
pub async fn wait_true(page: &Page, expression: &str, seconds: u32, what: &str) -> Result<()> {
    if page.wait_for(expression, seconds * 4, 250).await? {
        Ok(())
    } else {
        bail!("{what} did not happen within {seconds} s")
    }
}

/// Sets the `index`-th element matching `selector` to `value`.
///
/// # Errors
///
/// When the element is missing or the value does not stick.
pub async fn set_field(page: &Page, selector: &str, index: usize, value: &str) -> Result<()> {
    let call = format!(
        "{SET_FIELD}({}, {index}, {})",
        json!(selector),
        json!(value)
    );
    match page.evaluate(&call, false).await?.as_str() {
        Some("ok") => Ok(()),
        other => bail!("set {selector}[{index}] = {value:?}: {other:?}"),
    }
}

/// The value of the `index`-th element matching `selector`.
///
/// # Errors
///
/// When the element is missing.
pub async fn field_value(page: &Page, selector: &str, index: usize) -> Result<String> {
    let v = page
        .evaluate(
            &format!(
                "(() => {{ const el = document.querySelectorAll({})[{index}]; \
                 return el ? el.value : null; }})()",
                json!(selector)
            ),
            false,
        )
        .await?;
    v.as_str()
        .map(str::to_string)
        .ok_or_else(|| anyhow!("no element {selector}[{index}]"))
}

/// Scrolls the map canvas into the middle of the viewport and returns its rectangle.
///
/// # Errors
///
/// When the page has no map canvas.
pub async fn map_canvas_rect(page: &Page) -> Result<CssRect> {
    let v = page
        .evaluate(
            "(() => { const c = document.querySelector('[data-mortar-map-state] canvas'); \
             if (!c) return null; c.scrollIntoView({ block: 'center' }); \
             const r = c.getBoundingClientRect(); \
             return { x: r.x, y: r.y, width: r.width, height: r.height }; })()",
            false,
        )
        .await?;
    let f = |k: &str| {
        v[k].as_f64()
            .ok_or_else(|| anyhow!("the page has no map canvas"))
    };
    Ok(CssRect {
        x: f("x")?,
        y: f("y")?,
        width: f("width")?,
        height: f("height")?,
    })
}

/// A trusted left click at `(x, y)` in CSS pixels: move, press, release.
///
/// # Errors
///
/// A protocol failure.
pub async fn click_at(page: &Page, x: f64, y: f64) -> Result<()> {
    page.dispatch_mouse(
        "mouseMoved",
        x,
        y,
        json!({ "button": "none", "buttons": 0 }),
    )
    .await?;
    page.dispatch_mouse("mousePressed", x, y, json!({ "buttons": 1 }))
        .await?;
    sleep_ms(60).await;
    page.dispatch_mouse("mouseReleased", x, y, json!({ "buttons": 0 }))
        .await?;
    Ok(())
}

/// Presses "Calculate Solution".
///
/// # Errors
///
/// When the button is missing or disabled.
pub async fn press_calculate(page: &Page) -> Result<()> {
    let v = page
        .evaluate(
            "(() => { const b = Array.from(document.querySelectorAll('button')) \
             .find((b) => b.textContent.trim() === 'Calculate Solution'); \
             if (!b) return 'missing'; if (b.disabled) return 'disabled'; \
             b.click(); return 'ok'; })()",
            false,
        )
        .await?;
    match v.as_str() {
        Some("ok") => Ok(()),
        other => bail!("Calculate Solution: {other:?}"),
    }
}

/// Reads the solution tables once the page shows a solution or its problems.
///
/// # Errors
///
/// The page's problem list, or no outcome within the wait.
pub async fn read_solution_tables(page: &Page) -> Result<SolutionTables> {
    wait_true(
        page,
        "!!document.querySelector('[data-mortar-solution], [data-mortar-problems]')",
        30,
        "a solution or a problem list",
    )
    .await?;
    let v: Value = page.evaluate(READ_TABLES, false).await?;
    if let Some(problems) = v["problems"].as_str() {
        bail!("the page refused the mission: {problems}");
    }
    serde_json::from_value(v["tables"].clone()).map_err(|e| anyhow!("read tables: {e} in {v}"))
}

#[cfg(test)]
#[path = "../tests/mortar_offline/page_driver.rs"]
mod tests;
