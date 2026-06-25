//! Provisional direct HTTP sync for local Phase 1 peers.

use std::future::Future;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use agoramesh_core::objects::validation;
use agoramesh_core::{Clock, Message, MessageId, Verification};
use agoramesh_store::{InsertResult, SqliteStore, Store};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use tokio::net::TcpListener;

/// Error type returned by direct HTTP sync operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to bind the HTTP listener.
    #[error("failed to bind direct sync listener: {0}")]
    Bind(#[source] std::io::Error),

    /// The HTTP server failed while serving requests.
    #[error("direct sync server failed: {0}")]
    Serve(#[source] std::io::Error),

    /// The local store returned an error.
    #[error("store error: {0}")]
    Store(#[from] agoramesh_store::Error),

    /// The HTTP client failed.
    #[error("http client error: {0}")]
    Http(#[from] reqwest::Error),

    /// A peer returned an unexpected status code.
    #[error("peer returned unexpected status {status} for {operation}")]
    UnexpectedStatus {
        /// Operation that received the status.
        operation: &'static str,
        /// Returned HTTP status.
        status: reqwest::StatusCode,
    },
}

/// Summary of a completed peer sync.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SyncStats {
    /// Remote objects inserted into the local store.
    pub objects_pulled: usize,
    /// Local objects accepted by the remote peer.
    pub objects_pushed: usize,
    /// Objects rejected by verification locally or remotely.
    pub objects_rejected: usize,
}

#[derive(Clone, Debug)]
struct DirectSyncState {
    store: Arc<Mutex<SqliteStore>>,
    clock: Arc<dyn Clock + Send + Sync>,
}

#[derive(Debug, Deserialize)]
struct ObjectsQuery {
    scope: String,
}

#[derive(Debug, thiserror::Error)]
enum HandlerError {
    #[error("object not found")]
    NotFound,
    #[error("object rejected: {0}")]
    Rejected(agoramesh_core::message::Error),
    #[error("object validation failed: {0}")]
    Invalid(validation::Error),
    #[error("duplicate object")]
    Duplicate,
    #[error("store lock poisoned")]
    StoreLock,
    #[error("store error: {0}")]
    Store(agoramesh_store::Error),
}

/// Starts a direct HTTP sync server and returns its bound address plus server future.
///
/// # Errors
///
/// Returns [`Error::Bind`] if the listener cannot bind to `listen_addr`.
pub async fn serve(
    store: SqliteStore,
    clock: Arc<dyn Clock + Send + Sync>,
    listen_addr: SocketAddr,
) -> Result<(SocketAddr, impl Future<Output = Result<(), Error>>), Error> {
    let listener = TcpListener::bind(listen_addr).await.map_err(Error::Bind)?;
    let bound_addr = listener.local_addr().map_err(Error::Bind)?;
    let app = router(DirectSyncState {
        store: Arc::new(Mutex::new(store)),
        clock,
    });
    let server = async move { axum::serve(listener, app).await.map_err(Error::Serve) };
    Ok((bound_addr, server))
}

/// Pulls objects from a peer for `scope`, inserts them locally, then pushes local objects.
///
/// # Errors
///
/// Returns an error when HTTP transport fails, the local store fails for reasons
/// other than duplicate/rejected objects, or the peer returns an unexpected status.
pub async fn sync_with_peer(
    peer_url: &str,
    store: &mut SqliteStore,
    clock: &dyn Clock,
    scope: &str,
) -> Result<SyncStats, Error> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            "/",
            env!("CARGO_PKG_VERSION")
        ))
        .build()?;
    let base_url = peer_url.trim_end_matches('/');
    let local_messages = store.list_by_scope(scope, clock)?;
    let mut stats = SyncStats::default();

    let remote_messages: Vec<Message> = client
        .get(format!("{base_url}/objects"))
        .query(&[("scope", scope)])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    for message in remote_messages {
        match message.verify() {
            Verification::Accepted | Verification::AcceptedWithWarning(_) => {}
            Verification::Rejected(_) => {
                stats.objects_rejected = stats.objects_rejected.saturating_add(1);
                continue;
            }
        }
        match message.classify_clock_skew(clock) {
            Verification::Accepted | Verification::AcceptedWithWarning(_) => {}
            Verification::Rejected(_) => {
                stats.objects_rejected = stats.objects_rejected.saturating_add(1);
                continue;
            }
        }
        if let Err(_error) = validation::validate_phase1_message(&message) {
            stats.objects_rejected = stats.objects_rejected.saturating_add(1);
            continue;
        }
        match store.insert(message, clock)? {
            InsertResult::Inserted => {
                stats.objects_pulled = stats.objects_pulled.saturating_add(1);
            }
            InsertResult::Duplicate => {}
        }
    }

    for message in local_messages {
        let response = client
            .post(format!("{base_url}/objects"))
            .json(&message)
            .send()
            .await?;
        match response.status() {
            reqwest::StatusCode::CREATED => {
                stats.objects_pushed = stats.objects_pushed.saturating_add(1);
            }
            reqwest::StatusCode::CONFLICT => {}
            reqwest::StatusCode::UNPROCESSABLE_ENTITY => {
                stats.objects_rejected = stats.objects_rejected.saturating_add(1);
            }
            response_status => {
                return Err(Error::UnexpectedStatus {
                    operation: "push object",
                    status: response_status,
                });
            }
        }
    }

    Ok(stats)
}

fn router(state: DirectSyncState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/objects", get(list_objects).post(post_object))
        .route("/objects/{object_id_hex}", get(get_object))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

async fn list_objects(
    State(state): State<DirectSyncState>,
    Query(query): Query<ObjectsQuery>,
) -> Result<Json<Vec<Message>>, HandlerError> {
    let messages = {
        let store = lock_store(&state)?;
        store
            .list_by_scope(&query.scope, state.clock.as_ref())
            .map_err(HandlerError::Store)?
    };
    Ok(Json(messages))
}

async fn get_object(
    State(state): State<DirectSyncState>,
    Path(object_id_hex): Path<String>,
) -> Result<Json<Message>, HandlerError> {
    let id = MessageId::from_hex(&object_id_hex).map_err(|_error| HandlerError::NotFound)?;
    let message = {
        let store = lock_store(&state)?;
        store
            .get(id, state.clock.as_ref())
            .map_err(HandlerError::Store)?
            .ok_or(HandlerError::NotFound)?
    };
    Ok(Json(message))
}

async fn post_object(
    State(state): State<DirectSyncState>,
    Json(message): Json<Message>,
) -> Result<(StatusCode, Json<Message>), HandlerError> {
    match message.verify() {
        Verification::Accepted | Verification::AcceptedWithWarning(_) => {}
        Verification::Rejected(error) => return Err(HandlerError::Rejected(error)),
    }
    match message.classify_clock_skew(state.clock.as_ref()) {
        Verification::Accepted | Verification::AcceptedWithWarning(_) => {}
        Verification::Rejected(error) => return Err(HandlerError::Rejected(error)),
    }
    validation::validate_phase1_message(&message).map_err(HandlerError::Invalid)?;
    let mut store = lock_store(&state)?;
    match store.insert(message.clone(), state.clock.as_ref()) {
        Ok(InsertResult::Inserted) => Ok((StatusCode::CREATED, Json(message))),
        Ok(InsertResult::Duplicate) | Err(agoramesh_store::Error::DuplicateObjectId(_)) => {
            Err(HandlerError::Duplicate)
        }
        Err(agoramesh_store::Error::Rejected(error)) => Err(HandlerError::Rejected(error)),
        Err(error) => Err(HandlerError::Store(error)),
    }
}

fn lock_store(state: &DirectSyncState) -> Result<MutexGuard<'_, SqliteStore>, HandlerError> {
    state.store.lock().map_err(|_error| HandlerError::StoreLock)
}

impl IntoResponse for HandlerError {
    fn into_response(self) -> Response {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND.into_response(),
            Self::Rejected(_) | Self::Invalid(_) => {
                StatusCode::UNPROCESSABLE_ENTITY.into_response()
            }
            Self::Duplicate => StatusCode::CONFLICT.into_response(),
            Self::StoreLock | Self::Store(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
