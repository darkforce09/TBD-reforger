# Staging web server site

The Caddy site of the home server: the built app on port 3080, with the API's paths proxied to the
API. The `caddy` service of the staging compose file mounts this folder, and only this folder, so
nothing else of the deploy folder, the host's `deploy.env` least of all, is visible inside the
container.

## Contents

```text
deploy/caddy/
└── Caddyfile  the Caddy site on :3080: the built app, with API paths proxied to :8080
```

## How it works

`Caddyfile` listens on `:3080` and sends the cross-origin isolation headers the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s WebAssembly needs
(`Cross-Origin-Opener-Policy: same-origin`, `Cross-Origin-Embedder-Policy: credentialless`). It
proxies `/api/*`, `/uploads/*`, `/map-assets/*` and `/healthz` to `127.0.0.1:8080`, and serves every
other path from the built app's `apps/frontend/dist` with an `index.html` fallback; the
offline service worker loader `/service_worker.js` carries `Cache-Control: no-cache`, so every
update check revalidates it.

Its paths are the `caddy` container's. The service in `deploy/compose.staging.yml` mounts this
folder read-only at `/etc/tbd-caddy` and `apps/frontend/` read-only at `/srv/tbd-frontend`, and
starts Caddy on `/etc/tbd-caddy/Caddyfile`, so the site root `/srv/tbd-frontend/dist` is the built
app wherever the checkout sits. Both mounts are folders rather than single files, because the
deploy's rsync replaces a file by renaming a new one over it and a single-file mount keeps the
file it was started with.

```text
deploy/caddy/ ──mounted read-only at /etc/tbd-caddy──▶ caddy container ──:3080──▶ cloudflared, LAN
apps/frontend/ ──mounted read-only at /srv/tbd-frontend──▶ site root /srv/tbd-frontend/dist
/api/*, /uploads/*, /map-assets/*, /healthz ──reverse_proxy──▶ API on 127.0.0.1:8080
```

The service runs on the host's network, so Caddy listens on the host's `:3080` and the API sees
every proxied request come from `127.0.0.1`, the proxy its `TRUSTED_PROXIES` trusts. Caddy trusts
forwarded addresses only from the tunnel's loopback peer (`trusted_proxies static 127.0.0.1/32` in
the global `servers` block): it keeps the `X-Forwarded-For` that `cloudflared` sends and replaces
any other peer's with that peer's own address, so the API keys each visitor by their real address
and each LAN client by its own.

## Configuration

The site reads no setting: its port, headers, upstream and site root are written in `Caddyfile`.
Its location in the checkout is `CADDYFILE` in `tools/xtask/src/core/repository_layout.rs`.

## Installed by

- `Caddyfile`: rsynced with the checkout and served by the `caddy` service of
  `deploy/compose.staging.yml`, which every `cargo xtask deploy website` starts and then reloads
  with `caddy reload --config /etc/tbd-caddy/Caddyfile`, so an edit applies with the next deploy.

## Boundaries

- Depends on: the `caddy` service of `deploy/compose.staging.yml` (the `caddy:2` image, host
  networking, the two read-only mounts); the API on `127.0.0.1:8080`; the app built into
  `apps/frontend/dist`.
- Used by: `cargo xtask deploy website`, whose web server step names the file at
  `/etc/tbd-caddy/Caddyfile` (`tools/xtask/src/commands/deploy/website/remote_steps.rs`); the
  Cloudflare Tunnel, which targets `http://127.0.0.1:3080`; `apps/api/tests/forwarded_for_trust.rs`,
  which pins the `reverse_proxy 127.0.0.1:8080` upstream; the deploy tests in
  `tools/xtask/src/commands/deploy/tests/website/tests.rs`.
- Rules: this folder holds the Caddy site alone, since everything in it is readable inside the
  container (`the_caddy_service_mounts_no_folder_holding_the_deploy_secrets`); the compose
  service's mounts, the reload's path and the site root agree
  (`the_caddy_service_serves_what_the_caddyfile_and_the_reload_name`); only `127.0.0.1/32` is a
  trusted proxy (`the_caddyfile_trusts_forwarded_addresses_only_from_the_tunnel_peer`).

## Related documentation

- [Website deployment](/documentation/runbooks/website_deployment.md) — the Caddy step of the
  deploy (Phase E) and the podman proof of the forwarded-address trust.
- [Deployment templates](/deploy/README.md) — the deploy folder this one sits in.
