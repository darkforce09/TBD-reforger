//! T-165.5 — static SPA server (port of `driver/serve.mjs`).
//!
//! Serves a built SPA with the SAME cross-origin-isolation headers the app expects
//! (`crossOriginIsolated === true` for the wasm/SAB path). Any path without a file extension
//! falls back to index.html (client routing). Optional same-origin `/api/` proxy (the Trunk
//! `[[proxy]]` equivalent) and `/map-assets/` passthrough to the real packages/map-assets.

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, Method, StatusCode, Uri, header};
use axum::response::Response;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

fn mime_for(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "wasm" => "application/wasm",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "webp" => "image/webp",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ico" => "image/x-icon",
        "map" => "application/json",
        _ => return None,
    })
}

/// Strip leading `..` components (the serve.mjs traversal guard).
fn sanitize_rel(p: &str) -> PathBuf {
    PathBuf::from(p)
        .components()
        .filter(|c| matches!(c, Component::Normal(_)))
        .collect()
}

pub struct ServeConfig {
    pub dir: PathBuf,
    pub api_proxy: Option<String>,
    pub map_assets_dir: Option<PathBuf>,
}

struct AppState {
    cfg: ServeConfig,
    client: reqwest::Client,
}

pub struct RunningServer {
    pub port: u16,
    shutdown: Option<oneshot::Sender<()>>,
    handle: tokio::task::JoinHandle<()>,
}

impl RunningServer {
    pub async fn close(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        let _ = self.handle.await;
    }
}

fn base_headers(res: &mut Response) {
    let h = res.headers_mut();
    h.insert("Cross-Origin-Opener-Policy", "same-origin".parse().unwrap());
    h.insert(
        "Cross-Origin-Embedder-Policy",
        "credentialless".parse().unwrap(),
    );
    h.insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
}

fn respond(status: StatusCode, content_type: Option<&str>, body: Vec<u8>) -> Response {
    let mut res = Response::builder().status(status);
    if let Some(ct) = content_type {
        res = res.header(header::CONTENT_TYPE, ct);
    }
    let mut res = res.body(Body::from(body)).unwrap();
    base_headers(&mut res);
    res
}

/// Parse `bytes=START-END` (END optional). Returns inclusive byte range clamped to `file_len`.
fn parse_bytes_range(h: &HeaderMap, file_len: u64) -> Option<(u64, u64)> {
    if file_len == 0 {
        return None;
    }
    let raw = h.get(header::RANGE)?.to_str().ok()?;
    let spec = raw.strip_prefix("bytes=")?;
    // Single range only (sat preview / TBDS tiles).
    let spec = spec.split(',').next()?.trim();
    let (start_s, end_s) = spec.split_once('-')?;
    if start_s.is_empty() {
        // suffix form `bytes=-N`
        let n: u64 = end_s.parse().ok()?;
        if n == 0 {
            return None;
        }
        let start = file_len.saturating_sub(n);
        return Some((start, file_len - 1));
    }
    let start: u64 = start_s.parse().ok()?;
    if start >= file_len {
        return None;
    }
    let end = if end_s.is_empty() {
        file_len - 1
    } else {
        end_s.parse::<u64>().ok()?.min(file_len - 1)
    };
    if end < start {
        return None;
    }
    Some((start, end))
}

/// Serve one map-asset file with optional HTTP Range (T-166). Full GET still reads the file;
/// Range uses seek + exact-length read so a 152_713_114 B sat bundle is never fully buffered.
async fn serve_map_asset(file: &Path, headers: &HeaderMap) -> Response {
    let meta = match tokio::fs::metadata(file).await {
        Ok(m) if m.is_file() => m,
        _ => return respond(StatusCode::NOT_FOUND, None, b"map-asset not found".to_vec()),
    };
    let file_len = meta.len();
    let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
    let ct = mime_for(ext).unwrap_or("application/octet-stream");

    if let Some((start, end)) = parse_bytes_range(headers, file_len) {
        let len = end - start + 1;
        let mut f = match tokio::fs::File::open(file).await {
            Ok(f) => f,
            Err(_) => {
                return respond(StatusCode::NOT_FOUND, None, b"map-asset not found".to_vec());
            }
        };
        if f.seek(std::io::SeekFrom::Start(start)).await.is_err() {
            return respond(
                StatusCode::INTERNAL_SERVER_ERROR,
                None,
                b"seek failed".to_vec(),
            );
        }
        let mut buf = vec![0u8; len as usize];
        if f.read_exact(&mut buf).await.is_err() {
            return respond(
                StatusCode::INTERNAL_SERVER_ERROR,
                None,
                b"range read failed".to_vec(),
            );
        }
        let mut res = Response::builder()
            .status(StatusCode::PARTIAL_CONTENT)
            .header(header::CONTENT_TYPE, ct)
            .header(header::ACCEPT_RANGES, "bytes")
            .header(
                header::CONTENT_RANGE,
                format!("bytes {start}-{end}/{file_len}"),
            )
            .header(header::CONTENT_LENGTH, len)
            .body(Body::from(buf))
            .unwrap();
        base_headers(&mut res);
        return res;
    }

    match tokio::fs::read(file).await {
        Ok(buf) => {
            let mut res = respond(StatusCode::OK, Some(ct), buf);
            res.headers_mut()
                .insert(header::ACCEPT_RANGES, "bytes".parse().unwrap());
            res
        }
        Err(_) => respond(StatusCode::NOT_FOUND, None, b"map-asset not found".to_vec()),
    }
}

async fn handler(
    State(state): State<Arc<AppState>>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let path = uri.path().to_string();

    // /map-assets/ passthrough (T-159.28). T-166: honor Range with seek+partial read → 206
    // so CI sat preview never loads the full 152_713_114 B `.tbd-sat` into RAM.
    if let Some(assets) = &state.cfg.map_assets_dir
        && let Some(rest) = path.strip_prefix("/map-assets/")
    {
        let decoded = percent_decode(rest);
        let file = assets.join(sanitize_rel(&decoded));
        return serve_map_asset(&file, &headers).await;
    }

    // Same-origin API proxy (T-159.25 equivalent).
    if let Some(proxy) = &state.cfg.api_proxy
        && path.starts_with("/api/")
    {
        let target = format!(
            "{proxy}{}",
            uri.path_and_query().map(|pq| pq.as_str()).unwrap_or(&path)
        );
        let mut req = state.client.request(method.clone(), &target);
        for (k, v) in headers.iter() {
            if k != header::HOST {
                req = req.header(k, v);
            }
        }
        if !matches!(method, Method::GET | Method::HEAD) {
            req = req.body(body.to_vec());
        }
        return match req.send().await {
            Ok(upstream) => {
                let status = StatusCode::from_u16(upstream.status().as_u16())
                    .unwrap_or(StatusCode::BAD_GATEWAY);
                let ct = upstream
                    .headers()
                    .get(header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("application/json")
                    .to_string();
                let buf = upstream.bytes().await.unwrap_or_default().to_vec();
                respond(status, Some(&ct), buf)
            }
            Err(e) => respond(
                StatusCode::BAD_GATEWAY,
                None,
                format!("proxy error: {e}").into_bytes(),
            ),
        };
    }

    // Static file with SPA fallback: no extension (a client route) → index.html.
    let decoded = percent_decode(&path);
    let rel = sanitize_rel(decoded.trim_start_matches('/'));
    let mut file = state.cfg.dir.join(&rel);
    let mut ext = file
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string();
    if ext.is_empty() || tokio::fs::metadata(&file).await.is_err() {
        file = state.cfg.dir.join("index.html");
        ext = "html".to_string();
    }
    match tokio::fs::read(&file).await {
        Ok(buf) => respond(
            StatusCode::OK,
            Some(mime_for(&ext).unwrap_or("application/octet-stream")),
            buf,
        ),
        Err(e) => respond(
            StatusCode::INTERNAL_SERVER_ERROR,
            None,
            format!("serve error: {e}").into_bytes(),
        ),
    }
}

fn percent_decode(s: &str) -> String {
    // Minimal %XX decoder (serve.mjs uses decodeURIComponent; asset paths here are ASCII).
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(h), Some(l)) = (hex(bytes[i + 1]), hex(bytes[i + 2]))
        {
            out.push(h * 16 + l);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Bind and serve. `port = 0` picks an ephemeral port (returned in `RunningServer.port`).
pub async fn start_server(cfg: ServeConfig, port: u16) -> Result<RunningServer> {
    let state = Arc::new(AppState {
        cfg,
        client: reqwest::Client::new(),
    });
    let app = axum::Router::new().fallback(handler).with_state(state);
    let listener = TcpListener::bind(("127.0.0.1", port)).await?;
    let actual = listener.local_addr()?.port();
    let (tx, rx) = oneshot::channel::<()>();
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = rx.await;
            })
            .await;
    });
    Ok(RunningServer {
        port: actual,
        shutdown: Some(tx),
        handle,
    })
}

/// Resolve the repo root from CARGO_MANIFEST_DIR (tools/tbd-tools → ../..) or cwd.
pub fn repo_root() -> PathBuf {
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    here.parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}
