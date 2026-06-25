#![allow(missing_docs, reason = "integration test crate has no public API")]
#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "test assertions should fail with contextual panics"
)]

use std::net::SocketAddr;
use std::sync::Arc;

use agoramesh_core::{Clock, Keypair, Message};
use agoramesh_net::direct_sync::{SyncStats, serve, sync_with_peer};
use agoramesh_net::topic::topic_for_category;
use agoramesh_store::{Connection, SqliteStore, Store};
use chrono::{DateTime, TimeDelta, Utc};
use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use serde_json::Value;

#[derive(Debug)]
struct FixedClock {
    now: DateTime<Utc>,
}

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.now
    }
}

const fn fixed_clock() -> FixedClock {
    FixedClock {
        now: DateTime::from_timestamp(1_800_000_000, 0).expect("valid timestamp"),
    }
}

fn sqlite_store() -> SqliteStore {
    SqliteStore::new(Connection::open_in_memory().expect("open in-memory sqlite"))
}

fn message(scope: &str, created_at: DateTime<Utc>, body: &[u8]) -> Message {
    Message::create(
        &Keypair::generate(),
        "message",
        created_at,
        scope.to_owned(),
        body,
    )
    .expect("create valid message")
}

fn tamper_body(message: &Message) -> Message {
    let mut value = serde_json::to_value(message).expect("serialize message");
    let Value::Object(root) = &mut value else {
        panic!("message serializes as object");
    };
    let Some(Value::Object(payload)) = root.get_mut("signed_payload") else {
        panic!("message has signed_payload object");
    };
    payload.insert("body".to_owned(), Value::String("ZXZpbA".to_owned()));
    serde_json::from_value(value).expect("deserialize tampered message")
}

async fn spawn_server(
    store: SqliteStore,
    clock: Arc<dyn Clock + Send + Sync>,
) -> (
    SocketAddr,
    tokio::task::JoinHandle<Result<(), agoramesh_net::direct_sync::Error>>,
) {
    let listen_addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let (addr, server) = serve(store, clock, listen_addr).await.expect("serve binds");
    let handle = tokio::spawn(server);
    (addr, handle)
}

#[test]
fn topic_for_category_formats_objects_topic() {
    assert_eq!(topic_for_category("abc"), "agoramesh/v0/abc/objects");
}

#[tokio::test]
async fn server_serves_health_lists_objects_oldest_first_and_fetches_by_id() {
    let clock = fixed_clock();
    let mut store = sqlite_store();
    let newer = message("scope-a", clock.now(), b"newer");
    let older = message("scope-a", clock.now() - TimeDelta::seconds(5), b"older");
    let other_scope = message("scope-b", clock.now(), b"other");
    store.insert(newer.clone(), &clock).expect("insert newer");
    store.insert(older.clone(), &clock).expect("insert older");
    store
        .insert(other_scope, &clock)
        .expect("insert other scope");

    let (addr, handle) = spawn_server(store, Arc::new(clock)).await;
    let client = reqwest::Client::new();
    let base_url = format!("http://{addr}");

    let health = client
        .get(format!("{base_url}/health"))
        .send()
        .await
        .expect("send health request")
        .text()
        .await
        .expect("read health body");
    assert_eq!(health, "ok");

    let objects: Vec<Message> = client
        .get(format!("{base_url}/objects"))
        .query(&[("scope", "scope-a")])
        .send()
        .await
        .expect("send list request")
        .error_for_status()
        .expect("list succeeds")
        .json()
        .await
        .expect("decode objects");
    assert_eq!(objects, vec![older.clone(), newer]);

    let fetched: Message = client
        .get(format!("{base_url}/objects/{}", older.id().to_hex()))
        .send()
        .await
        .expect("send get request")
        .error_for_status()
        .expect("get succeeds")
        .json()
        .await
        .expect("decode object");
    assert_eq!(fetched, older);

    handle.abort();
}

#[tokio::test]
async fn server_rejects_invalid_signatures_and_duplicate_posts() {
    let clock = fixed_clock();
    let store = sqlite_store();
    let valid = message("scope-a", clock.now(), b"valid");
    let invalid = tamper_body(&valid);
    let (addr, handle) = spawn_server(store, Arc::new(clock)).await;
    let client = reqwest::Client::new();
    let base_url = format!("http://{addr}");

    let rejected = client
        .post(format!("{base_url}/objects"))
        .json(&invalid)
        .send()
        .await
        .expect("send invalid post");
    assert_eq!(rejected.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let created = client
        .post(format!("{base_url}/objects"))
        .json(&valid)
        .send()
        .await
        .expect("send valid post");
    assert_eq!(created.status(), StatusCode::CREATED);

    let duplicate = client
        .post(format!("{base_url}/objects"))
        .json(&valid)
        .send()
        .await
        .expect("send duplicate post");
    assert_eq!(duplicate.status(), StatusCode::CONFLICT);

    let objects: Vec<Message> = client
        .get(format!("{base_url}/objects"))
        .query(&[("scope", "scope-a")])
        .send()
        .await
        .expect("send list request")
        .error_for_status()
        .expect("list succeeds")
        .json()
        .await
        .expect("decode objects");
    assert_eq!(objects, vec![valid]);

    handle.abort();
}

#[tokio::test]
async fn sync_with_peer_pulls_remote_objects_dedupes_and_pushes_local_objects() {
    let clock = fixed_clock();
    let remote_only = message("scope-a", clock.now(), b"remote");
    let local_only = message("scope-a", clock.now() - TimeDelta::seconds(10), b"local");
    let shared = message("scope-a", clock.now() - TimeDelta::seconds(20), b"shared");

    let mut remote_store = sqlite_store();
    remote_store
        .insert(remote_only.clone(), &clock)
        .expect("insert remote only");
    remote_store
        .insert(shared.clone(), &clock)
        .expect("insert remote shared");

    let mut local_store = sqlite_store();
    local_store
        .insert(local_only.clone(), &clock)
        .expect("insert local only");
    local_store
        .insert(shared.clone(), &clock)
        .expect("insert local shared");

    let (addr, handle) =
        spawn_server(remote_store, Arc::new(FixedClock { now: clock.now() })).await;

    let stats = sync_with_peer(
        &format!("http://{addr}"),
        &mut local_store,
        &clock,
        "scope-a",
    )
    .await
    .expect("sync succeeds");
    assert_eq!(
        stats,
        SyncStats {
            objects_pulled: 1,
            objects_pushed: 1,
            objects_rejected: 0,
        }
    );

    let local_messages = local_store
        .list_by_scope("scope-a", &clock)
        .expect("list local messages");
    assert_eq!(local_messages, vec![shared, local_only, remote_only]);

    handle.abort();
}
