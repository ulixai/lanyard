use crate::state::{changed, show, work, AccessReply, Desktop, Pending, RequestInput};
use axum::{
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use lanyard_core::Error;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tauri::AppHandle;
#[derive(Clone)]
struct Api {
    desktop: Arc<Desktop>,
    app: AppHandle,
}
fn failure(status: StatusCode, code: &str, message: &str) -> Response {
    (
        status,
        [("Cache-Control", "no-store")],
        Json(serde_json::json!({"status":"error","code":code,"error":message})),
    )
        .into_response()
}
fn validate(headers: &HeaderMap, port: u16, post: bool) -> std::result::Result<(), Response> {
    let expected = format!("127.0.0.1:{port}");
    if headers.get("host").and_then(|v| v.to_str().ok()) != Some(expected.as_str())
        || headers.contains_key("origin")
        || headers.contains_key("sec-fetch-site")
    {
        return Err(failure(
            StatusCode::FORBIDDEN,
            "desktop_only",
            "Only local desktop and CLI clients are supported.",
        ));
    }
    if post
        && (headers
            .get("x-lanyard-protocol")
            .and_then(|v| v.to_str().ok())
            != Some("2")
            || headers
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.split(';').next().unwrap_or("").trim())
                != Some("application/json"))
    {
        return Err(failure(
            StatusCode::BAD_REQUEST,
            "protocol",
            "Use the Lanyard v2 JSON protocol.",
        ));
    }
    Ok(())
}
async fn health(State(api): State<Api>, headers: HeaderMap) -> Response {
    if let Err(r) = validate(&headers, api.desktop.port, false) {
        return r;
    }
    ( [("Cache-Control","no-store")],Json(serde_json::json!({"protocol":2,"version":env!("CARGO_PKG_VERSION"),"instance":api.desktop.instance}))).into_response()
}
async fn legacy() -> Response {
    failure(
        StatusCode::UPGRADE_REQUIRED,
        "upgrade_required",
        "Install lanyard>=0.2.0. The old name-only protocol has been retired.",
    )
}
async fn request(
    State(api): State<Api>,
    headers: HeaderMap,
    input: std::result::Result<Json<RequestInput>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if let Err(r) = validate(&headers, api.desktop.port, true) {
        return r;
    }
    let input = match input {
        Ok(Json(i)) => i,
        Err(_) => {
            return failure(
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "Invalid JSON request.",
            )
        }
    };
    if input.timeout == 0
        || input.timeout > 300
        || input.reason.as_ref().is_some_and(|r| r.len() > 2048)
        || input
            .target_id
            .as_ref()
            .is_some_and(|id| uuid::Uuid::parse_str(id).is_err())
        || input.auth.validate().is_err()
    {
        return failure(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "Check identity, target, reason, and timeout.",
        );
    }
    let auth = input.auth.clone();
    let target = input.target_id.clone();
    let category = input.category;
    let preflight = work(&api.desktop, move |v| {
        let paired = v.authenticate(&auth)?;
        if !v.locked() {
            if let Some(id) = target {
                if !v.item_matches(&id, category)? {
                    return Err(Error::Denied);
                }
                if v.has_grant(&auth, &id)? {
                    return Ok((
                        paired,
                        Some(AccessReply {
                            status: "success",
                            data: v.reveal(&id)?,
                            target_id: id,
                        }),
                    ));
                }
            }
        }
        Ok((paired, None))
    })
    .await;
    let paired = match preflight {
        Ok((_, Some(reply))) => {
            return ([("Cache-Control", "no-store")], Json(reply)).into_response()
        }
        Ok((paired, None)) => paired,
        Err(Error::NotFound) => {
            return failure(
                StatusCode::NOT_FOUND,
                "not_found",
                "The requested credential no longer exists.",
            )
        }
        Err(e) => return failure(StatusCode::FORBIDDEN, "denied", &e.to_string()),
    };
    let duration = Duration::from_secs(input.timeout);
    let id = uuid::Uuid::new_v4().to_string();
    let (sender, receiver) = tokio::sync::oneshot::channel();
    {
        let Ok(mut queue) = api.desktop.pending.lock() else {
            return failure(
                StatusCode::SERVICE_UNAVAILABLE,
                "unavailable",
                "Request queue unavailable.",
            );
        };
        if api
            .desktop
            .updating
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            return failure(
                StatusCode::SERVICE_UNAVAILABLE,
                "updating",
                "Lanyard is installing an update. Try again shortly.",
            );
        }
        queue.retain(|p| !p.sender.is_closed() && p.deadline > Instant::now());
        if queue.len() >= 16 {
            return failure(
                StatusCode::TOO_MANY_REQUESTS,
                "busy",
                "Too many pending requests. Try again shortly.",
            );
        }
        queue.push_back(Pending {
            id: id.clone(),
            input,
            deadline: Instant::now() + duration,
            paired,
            sender,
        });
    }
    changed(&api.app);
    show(&api.app);
    let result = tokio::time::timeout(duration, receiver).await;
    if let Ok(mut queue) = api.desktop.pending.lock() {
        queue.retain(|p| p.id != id)
    }
    changed(&api.app);
    match result {
        Ok(Ok(Ok(reply))) => ([("Cache-Control", "no-store")], Json(reply)).into_response(),
        Ok(Ok(Err(e))) => failure(StatusCode::FORBIDDEN, "denied", &e.to_string()),
        _ => failure(
            StatusCode::REQUEST_TIMEOUT,
            "timeout",
            "The access request expired or was cancelled.",
        ),
    }
}
pub fn start(listener: std::net::TcpListener, desktop: Arc<Desktop>, app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let router = Router::new()
            .route("/v1/health", get(health))
            .route("/v1/request", post(request))
            .route("/lanyard-ipc", post(legacy))
            .layer(DefaultBodyLimit::max(16 * 1024))
            .with_state(Api {
                desktop,
                app: app.clone(),
            });
        let result = async {
            let listener = tokio::net::TcpListener::from_std(listener)?;
            axum::serve(listener, router).await
        }
        .await;
        if let Err(e) = result {
            eprintln!("Lanyard local API stopped: {e}");
            use tauri::Emitter;
            let _ = app.emit_to(
                "main",
                "lanyard-error",
                "The local API stopped. Restart Lanyard to reconnect desktop apps.",
            );
        }
    });
}
