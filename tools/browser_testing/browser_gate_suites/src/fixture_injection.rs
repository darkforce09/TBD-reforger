//! The determinism payload the browser runs on document start.
//!
//! `FREEZE_SRC` executes inside V8 through `Page.addScriptToEvaluateOnNewDocument`; the harness only
//! ferries it. It fixes the clock, seeds the random sources and disables animations and
//! transitions, so a screenshot or a timing-free probe sees the same page on every run.
//!
//! **Role:** `FREEZE_SRC`, the script a frozen page runs on document start.
//! **Position:** injected by `gate render-check` (unless `--no-freeze`) through `new_page`'s init
//! scripts.
//! **Signals & state:** none; one constant.
//! **Invariants:** the clock reads one fixed epoch and the random sources one fixed seed.

/// The determinism payload — fixed clock, seeded RNG, animation kill — injected at document start.
pub const FREEZE_SRC: &str = r#"
(() => {
  const T0 = 1700000000000; // fixed epoch (2023-11-14T22:13:20Z)
  const OrigDate = Date;
  function FrozenDate(...args) {
    if (!(this instanceof FrozenDate)) return new OrigDate(T0).toString();
    return args.length ? new OrigDate(...args) : new OrigDate(T0);
  }
  FrozenDate.now = () => T0;
  FrozenDate.parse = OrigDate.parse;
  FrozenDate.UTC = OrigDate.UTC;
  FrozenDate.prototype = OrigDate.prototype;
  try { window.Date = FrozenDate; } catch (e) {}
  try { Date.now = () => T0; } catch (e) {}
  try { performance.now = () => 0; } catch (e) {}

  // Seeded LCG for Math.random + crypto.getRandomValues (deterministic across runs).
  let seed = 0x1a2b3c4d;
  const next = () => { seed = (Math.imul(seed, 1103515245) + 12345) & 0x7fffffff; return seed; };
  Math.random = () => next() / 0x80000000;
  try {
    crypto.getRandomValues = (arr) => { for (let i = 0; i < arr.length; i++) arr[i] = next() & 0xff; return arr; };
  } catch (e) {}

  const inject = () => {
    const s = document.createElement('style');
    s.setAttribute('data-dom-oracle-freeze', '1');
    s.textContent = '*,*::before,*::after{animation:none!important;transition:none!important;caret-color:transparent!important;scroll-behavior:auto!important}';
    (document.head || document.documentElement).appendChild(s);
  };
  if (document.head || document.documentElement) inject();
  else document.addEventListener('DOMContentLoaded', inject);
})();
"#;
