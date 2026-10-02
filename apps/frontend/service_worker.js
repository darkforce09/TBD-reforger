// Offline service worker loader. It holds no policy: it imports the Rust worker that Trunk builds
// from apps/offline_service_worker, registers the three event listeners synchronously
// (a service worker must add them during its first evaluation, before the WebAssembly module has
// finished loading), and hands each event to the Rust handler once the module is ready.
// The script URL's query (`?build=<id>`) selects the build and is forwarded to both files, so a
// new build never reuses an older worker module from the HTTP cache.
importScripts(`/offline_service_worker.js${self.location.search}`);
const ready = wasm_bindgen({ module_or_path: `/offline_service_worker_bg.wasm${self.location.search}` });
self.addEventListener("install", (event) => {
  event.waitUntil(ready.then(() => wasm_bindgen.on_install(event)));
});
self.addEventListener("activate", (event) => {
  event.waitUntil(ready.then(() => wasm_bindgen.on_activate(event)));
});
self.addEventListener("fetch", (event) => {
  event.respondWith(ready.then(() => wasm_bindgen.on_fetch(event)));
});
