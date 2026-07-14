// dom.js — the V-gate PRIMARY oracle: a normalized DOM + computed-style serializer.
//
// Injected as an init script so window.__t159SerializeDom() is callable after load. Walks the
// rendered <body>, emitting canonical JSON of {tag, attrs, style, children}. Deterministic and
// driver-independent (no layout geometry), so React and Leptos producing the same structure under
// the same index.css serialize byte-identically — that equality IS the gate.
//
// Normalizations that make cross-framework equality achievable:
//   - text nodes: collapse whitespace, trim, drop empties
//   - ids: rewritten to positional '#<n>' (React useId vs Leptos ids differ, structure doesn't);
//     every id-referencing attr (for, aria-controls/labelledby/describedby, …) rewritten the same
//   - attrs sorted by name; class kept verbatim (class strings ARE the styling contract)
//   - computed styles: a curated non-geometric subset (color/font/display/border/…), never
//     width/height/top/left (those are layout-dependent and flaky)
//   - skipped: script/style/noscript/link/meta/template + the harness's own freeze <style>

export const DOM_SERIALIZER_SRC = /* js */ `
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
  const rewriteRefs = (v) => v.split(/\\s+/).filter(Boolean).map(idx).join(' ');

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
          const t = node.textContent.replace(/\\s+/g, ' ').trim();
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
`
