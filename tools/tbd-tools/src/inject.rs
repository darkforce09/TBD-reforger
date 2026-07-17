//! T-165.5 — the browser-injected JS payloads, byte-copied from the Node harness.
//!
//! PROVENANCE + HARD RULE (T-165 plan, V-golden byte-parity): these strings are the exact
//! template-literal bodies of `.ai/artifacts/t159_gates/driver/freeze.js` (`FREEZE_SRC`) and
//! `driver/dom.js` (`DOM_SERIALIZER_SRC`). They execute inside V8 via
//! `Page.addScriptToEvaluateOnNewDocument`; the harness only ferries them. The frozen V-suite
//! goldens were serialized by THIS exact serializer source — never re-implement or "clean up"
//! these payloads natively, or the byte-identity contract with the committed goldens breaks.

/// `freeze.js` — determinism payload (fixed clock, seeded RNG, animation kill), document-start.
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
    s.setAttribute('data-t159-freeze', '1');
    s.textContent = '*,*::before,*::after{animation:none!important;transition:none!important;caret-color:transparent!important;scroll-behavior:auto!important}';
    (document.head || document.documentElement).appendChild(s);
  };
  if (document.head || document.documentElement) inject();
  else document.addEventListener('DOMContentLoaded', inject);
})();
"#;

/// `dom.js` — the V-gate normalized DOM + computed-style serializer
/// (`window.__t159SerializeDom`). See dom.js header for the normalization contract.
///
/// NOTE on escapes: the .js file lives inside a JS template literal, so its source escapes
/// `\s` as `\\s` — the browser receives single-backslash `\s`. This Rust raw string holds the
/// POST-UNESCAPE bytes (what V8 actually received), i.e. `\s+` and `/\s+/g`, keeping the wire
/// bytes identical to the Node harness.
pub const DOM_SERIALIZER_SRC: &str = r#"
window.__t159SerializeDom = function (selector, exclude) {
  const STYLE_PROPS = [
    'display', 'position', 'visibility', 'opacity',
    'color', 'background-color',
    'border-top-color', 'border-top-width', 'border-top-style', 'border-radius',
    'font-family', 'font-size', 'font-weight', 'line-height', 'letter-spacing',
    'text-align', 'text-transform', 'white-space',
    'flex-direction', 'justify-content', 'align-items', 'gap',
  ];
  const ID_REF_ATTRS = new Set([
    'id', 'for', 'aria-controls', 'aria-labelledby', 'aria-describedby',
    'aria-owns', 'aria-activedescendant', 'aria-details', 'headers',
    'list', 'form', 'popovertarget',
  ]);
  const SKIP = new Set(['SCRIPT', 'STYLE', 'NOSCRIPT', 'LINK', 'META', 'TEMPLATE']);
  // Provably non-visual framework artifacts stripped from BOTH sides — no CSS targets them, no
  // layout/paint effect. data-discover is React Router v7's prefetch hint; carrying it into the
  // Leptos DOM would be dead cruft. Equality is defined modulo these.
  const SKIP_ATTRS = new Set(['data-discover']);

  // Positional id map (document order) → '#0', '#1', …
  const ids = [];
  document.querySelectorAll('[id]').forEach((el) => { if (!ids.includes(el.id)) ids.push(el.id); });
  const idx = (v) => { const i = ids.indexOf(v); return i >= 0 ? '#' + i : v; };
  const rewriteRefs = (v) => v.split(/\s+/).filter(Boolean).map(idx).join(' ');

  function walk(el) {
    if (SKIP.has(el.tagName) || el.getAttribute('data-t159-freeze') === '1') return null;
    const cs = getComputedStyle(el);
    const style = {};
    for (const p of STYLE_PROPS) style[p] = cs.getPropertyValue(p);
    const attrs = {};
    for (const a of [...el.attributes].sort((x, y) => x.name.localeCompare(y.name))) {
      if (SKIP_ATTRS.has(a.name)) continue;
      attrs[a.name] = a.name === 'id' ? idx(a.value) : ID_REF_ATTRS.has(a.name) ? rewriteRefs(a.value) : a.value;
    }
    const children = [];
    // An excluded element is serialized as a leaf (tag/attrs/style kept, subtree opaque) — used to
    // compare the chrome skeleton while a not-yet-ported page region inside <main> is ignored.
    if (!(exclude && el.matches(exclude))) {
      for (const node of el.childNodes) {
        if (node.nodeType === 3) {
          const t = node.textContent.replace(/\s+/g, ' ').trim();
          if (t) children.push(t);
        } else if (node.nodeType === 1) {
          const c = walk(node);
          if (c) children.push(c);
        }
      }
    }
    return { tag: el.tagName.toLowerCase(), attrs, style, children };
  }
  var root = selector ? document.querySelector(selector) : document.body;
  return JSON.stringify(root ? walk(root) : null);
};
"#;

#[cfg(test)]
mod tests {
    use super::*;

    /// The two payloads must stay byte-identical to the Node harness sources (post template-
    /// literal unescape: `\\s` in the .js file source = `\s` on the wire). Guards against
    /// "cleanup" drift while the .mjs driver still exists (deleted @ T-165.6 — then this test
    /// pins the frozen-golden contract alone).
    #[test]
    fn payloads_match_node_driver_sources() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
        for (rust_src, file, export) in [
            (FREEZE_SRC, "freeze.js", "FREEZE_SRC"),
            (DOM_SERIALIZER_SRC, "dom.js", "DOM_SERIALIZER_SRC"),
        ] {
            let path = format!("{root}/.ai/artifacts/t159_gates/driver/{file}");
            let Ok(js) = std::fs::read_to_string(&path) else {
                // Driver deleted (post T-165.6): the Rust consts ARE the source of truth.
                continue;
            };
            let tail = js
                .split(&format!("{export} = /* js */ `"))
                .nth(1)
                .unwrap_or_else(|| panic!("{file}: export marker not found"));
            let body = tail
                .rsplit_once('`')
                .unwrap_or_else(|| panic!("{file}: unterminated template literal"))
                .0;
            // The only escape used in these literals is `\\` (file) → `\` (wire).
            let expected = body.replace("\\\\", "\\");
            assert_eq!(
                rust_src, expected,
                "{file}: Rust const drifted from the Node driver template literal"
            );
        }
    }
}
