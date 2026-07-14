// freeze.js — determinism payload, injected on document-start (before app scripts) via
// Page.addScriptToEvaluateOnNewDocument on BOTH the React oracle and the Leptos target.
//
// Kills every non-determinism source that would make two renders differ: wall-clock time,
// randomness, and CSS animation/transition/caret. With this in place a fixed fixture dataset
// renders byte-identically across runs and across frameworks — the precondition for V/R/T ε=0.

export const FREEZE_SRC = /* js */ `
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
    s.setAttribute('data-t159-freeze', '1');
    s.textContent = '*,*::before,*::after{animation:none!important;transition:none!important;caret-color:transparent!important;scroll-behavior:auto!important}';
    (document.head || document.documentElement).appendChild(s);
  };
  if (document.head || document.documentElement) inject();
  else document.addEventListener('DOMContentLoaded', inject);
})();
`
