//! The HTTP API end to end: `serve` started in-process on port 0, driven by a
//! blocking HTTP client as publishers and consumers would drive it, then stopped.
//! Every assertion reads a reply the server wrote; every wait is bounded.

// Test-only from the root, so clippy treats the helpers here as test code, as it
// treats the tests: the unwrap, expect and panic allowances of clippy.toml cover them.
#![cfg(test)]

use std::fmt::Write as _;
use std::time::{Duration, Instant};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use pachinko::ServeConfig;
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio::sync::watch;

/// How long one request may take before the test fails.
const REQUEST_BOUND: Duration = Duration::from_secs(10);
/// How long `serve` may take to stop once asked before the test fails.
const STOP_BOUND: Duration = Duration::from_secs(10);

/// A blocking client against one server.
struct Client {
    agent: ureq::Agent,
    base: String,
}

impl Client {
    fn new(address: std::net::SocketAddr) -> Self {
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(REQUEST_BOUND))
            .build()
            .into();
        Self {
            agent,
            base: format!("http://{address}"),
        }
    }

    /// Sends one request with a raw body and returns the status and the body: JSON
    /// when it parses, a string when it does not, `Null` when empty.
    fn send(&self, method: &str, path: &str, body: Option<&str>) -> (u16, Value) {
        let url = format!("{}{path}", self.base);
        let response = match (method, body) {
            ("GET", None) => self.agent.get(&url).call(),
            ("POST", None) => self.agent.post(&url).send_empty(),
            ("POST", Some(body)) => self
                .agent
                .post(&url)
                .header("content-type", "application/json")
                .send(body),
            ("PUT", Some(body)) => self
                .agent
                .put(&url)
                .header("content-type", "application/json")
                .send(body),
            other => panic!("no such request in this test: {other:?}"),
        };
        let mut response = response.unwrap();
        let status = response.status().as_u16();
        let text = response.body_mut().read_to_string().unwrap();
        let value = if text.is_empty() {
            Value::Null
        } else {
            serde_json::from_str(&text).unwrap_or(Value::String(text))
        };
        (status, value)
    }

    fn json(&self, method: &str, path: &str, body: &Value) -> (u16, Value) {
        self.send(method, path, Some(&body.to_string()))
    }

    fn join(&self, path: &str) {
        assert_eq!(self.send("POST", path, None), (201, Value::Null), "{path}");
    }

    /// Publishes `payload` as `p1` into `pocket` under `key`, both percent-encoded.
    fn publish(&self, pocket: &str, key: &str, rank: i64, payload: &[u8]) -> (u16, Value) {
        let body = json!({"publisher_id": "p1", "rank": rank, "payload": STANDARD.encode(payload)});
        let path = format!("/pockets/{}/records/{}", encode(pocket), encode(key));
        self.json("PUT", &path, &body)
    }

    fn pull(&self, consumer: &str, limit: i64) -> (u16, Value) {
        let path = format!("/consumers/{consumer}/pull");
        self.json("POST", &path, &json!({"limit": limit}))
    }

    fn lease(&self, consumer: &str) -> (u16, Value) {
        self.send("GET", &format!("/consumers/{consumer}/lease"), None)
    }
}

/// `text` with every byte outside the URL's unreserved set percent-encoded.
fn encode(text: &str) -> String {
    text.bytes().fold(String::new(), |mut encoded, byte| {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            write!(encoded, "%{byte:02X}").unwrap();
        }
        encoded
    })
}

/// The `(record_key, rank, payload bytes)` of each record in a batch or lease.
fn records(reply: &Value) -> Vec<(String, i64, Vec<u8>)> {
    reply["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|record| {
            (
                record["record_key"].as_str().unwrap().to_owned(),
                record["rank"].as_i64().unwrap(),
                STANDARD
                    .decode(record["payload"].as_str().unwrap())
                    .unwrap(),
            )
        })
        .collect()
}

fn record(key: &str, rank: i64, payload: &[u8]) -> (String, i64, Vec<u8>) {
    (key.to_owned(), rank, payload.to_vec())
}

/// Joins `p1` and each of `consumers`.
fn joined(client: &Client, consumers: &[&str]) {
    client.join("/publishers/p1");
    for consumer in consumers {
        client.join(&format!("/consumers/{consumer}"));
    }
}

/// Serves on port 0 while `drive` runs against it, then stops the server through
/// the stop channel and checks that it returned `Ok` in time.
async fn serve_while(drive: fn(&Client)) {
    let config = ServeConfig::parse(["--listen", "127.0.0.1:0"]).unwrap();
    let listener = TcpListener::bind(config.listen()).await.unwrap();
    let address = listener.local_addr().unwrap();
    let (stop, stopped) = watch::channel(());
    let client = async move {
        tokio::task::block_in_place(|| drive(&Client::new(address)));
        drop(stop);
        Instant::now()
    };
    let server = tokio::time::timeout(STOP_BOUND * 3, pachinko::serve(listener, &config, stopped));
    let (outcome, stop_sent) = tokio::join!(server, client);
    assert_eq!(outcome.expect("serve did not stop in time"), Ok(()));
    assert!(stop_sent.elapsed() < STOP_BOUND, "stopped in time");
}

// scenario: S019
// obligation: rule-success.Supersede
// obligation: rule-success.DropStalePublish
fn large_ranks(client: &Client) {
    joined(client, &["c1"]);
    let (max, min) = (i64::MAX, i64::MIN);
    assert_eq!(
        client.publish("big", "k", min, b"min").1,
        json!({"outcome": "created"})
    );
    assert_eq!(
        client.publish("big", "k", max, b"max").1,
        json!({"outcome": "superseded"})
    );
    assert_eq!(
        client.publish("big", "k", min, b"late").1,
        json!({"outcome": "stale"})
    );
    assert_eq!(
        client.publish("big", "j", max - 1, b"j1").1,
        json!({"outcome": "new_key"})
    );
    assert_eq!(
        client.publish("big", "j", max, b"j2").1,
        json!({"outcome": "superseded"})
    );
    let (status, batch) = client.pull("c1", 10);
    assert_eq!(status, 200);
    assert_eq!(
        records(&batch),
        [record("k", max, b"max"), record("j", max, b"j2")]
    );
    assert!(batch.to_string().contains("9223372036854775807"), "{batch}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn large_ranks_keep_full_precision_across_supersede_and_stale() {
    serve_while(large_ranks).await;
}

// scenario: S020
// obligation: rule-success.PublishIntoNewPocket
// obligation: rule-success.PublishNewKey
fn round_trip(client: &Client) {
    joined(client, &["c1"]);
    let every_byte: Vec<u8> = (0..=u8::MAX).collect();
    let (pocket, key) = ("tenant a/b ünïcode", "key with spaces/and slashes é");
    assert_eq!(
        client.publish(pocket, key, 1, &every_byte).1,
        json!({"outcome": "created"})
    );
    let (status, batch) = client.pull("c1", 10);
    assert_eq!((status, &batch["pocket_key"]), (200, &json!(pocket)));
    assert_eq!(records(&batch), [record(key, 1, &every_byte)]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn payload_bytes_and_keys_round_trip_unchanged() {
    serve_while(round_trip).await;
}

// scenario: S028
// obligation: guarantee.OldestFirst
fn limit_pages(client: &Client) {
    joined(client, &["c1"]);
    for (key, payload) in [("k1", b"1st"), ("k2", b"2nd"), ("k3", b"3rd")] {
        assert_eq!(client.publish("pk", key, 1, payload).0, 200);
    }
    let (status, first) = client.pull("c1", 2);
    assert_eq!(status, 200);
    assert_eq!(first["outcome"], json!("batch"));
    assert_eq!(
        records(&first),
        [record("k1", 1, b"1st"), record("k2", 1, b"2nd")]
    );
    let (_, second) = client.pull("c1", 2);
    assert_eq!(second["lease_id"], first["lease_id"], "the same lease");
    assert_eq!(records(&second), [record("k3", 1, b"3rd")]);
    let lease_id = first["lease_id"].as_str().unwrap();
    assert_eq!(lease_id.len(), 32);
    assert!(
        lease_id.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "{lease_id}"
    );
    assert!(first["expires_at"].as_u64().unwrap() > 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_positive_limit_returns_at_most_that_many_oldest_first_and_later_pulls_continue() {
    serve_while(limit_pages).await;
}

// scenario: S031
// obligation: rule-success.PullClaimsFrontPocket
fn zero_limit(client: &Client) {
    joined(client, &["c1"]);
    assert_eq!(client.publish("pk", "k1", 1, b"x").0, 200);
    let (status, batch) = client.pull("c1", 0);
    assert_eq!((status, &batch["outcome"]), (200, &json!("batch")));
    assert_eq!(batch["records"], json!([]));
    let (status, lease) = client.lease("c1");
    assert_eq!(status, 200);
    assert_eq!(lease["pocket_key"], json!("pk"));
    assert_eq!(lease["expires_at"], batch["expires_at"]);
    assert_eq!(lease["records"], json!([]));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_zero_limit_pull_claims_a_pocket_and_returns_an_empty_batch() {
    serve_while(zero_limit).await;
}

// scenario: S033
// obligation: rule-success.PullFindsNoPocket
fn no_pocket(client: &Client) {
    joined(client, &["c1"]);
    assert_eq!(
        client.pull("c1", 10),
        (200, json!({"outcome": "no_pocket"}))
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_pull_with_no_queued_pocket_returns_no_pocket_not_an_error() {
    serve_while(no_pocket).await;
}

/// The error code of a refusal, after checking its status and that it carries a
/// message.
fn refused(reply: (u16, Value), status: u16) -> String {
    let (got, body) = reply;
    assert_eq!(got, status, "{body}");
    assert!(
        body["message"]
            .as_str()
            .is_some_and(|message| !message.is_empty()),
        "{body}"
    );
    body["error"].as_str().unwrap().to_owned()
}

fn refusals(client: &Client) {
    assert_eq!(client.send("GET", "/healthz", None), (200, json!("ok")));
    joined(client, &["c1"]);
    assert_eq!(
        refused(client.send("POST", "/publishers/p1", None), 409),
        "publisher_exists"
    );
    assert_eq!(
        refused(client.send("POST", "/consumers/c1", None), 409),
        "consumer_exists"
    );
    let stranger = json!({"publisher_id": "nobody", "rank": 1, "payload": ""});
    let put = |body: &Value| client.json("PUT", "/pockets/pk/records/k", body);
    assert_eq!(refused(put(&stranger), 404), "unknown_publisher");
    assert_eq!(refused(client.pull("nobody", 1), 404), "unknown_consumer");
    assert_eq!(refused(client.lease("nobody"), 404), "unknown_consumer");
    assert_eq!(client.lease("c1"), (204, Value::Null));
    assert_eq!(client.publish("pk", "k", 5, b"x").0, 200);
    let (status, body) = client.publish("pk", "k", 5, b"y");
    assert_eq!(refused((status, body.clone()), 409), "equal_rank");
    assert!(
        body["message"].as_str().unwrap().contains("placeholder"),
        "{body}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn each_domain_refusal_has_its_status_and_code() {
    serve_while(refusals).await;
}

fn bad_requests(client: &Client) {
    joined(client, &["c1"]);
    let put = |body: &str| client.send("PUT", "/pockets/pk/records/k", Some(body));
    assert_eq!(refused(put("{not json"), 400), "bad_request");
    let not_base64 = json!({"publisher_id": "p1", "rank": 1, "payload": "***"}).to_string();
    assert_eq!(refused(put(&not_base64), 400), "bad_request");
    let missing = json!({"publisher_id": "p1", "payload": ""}).to_string();
    assert_eq!(refused(put(&missing), 400), "bad_request");
    let pull = |body: &str| client.send("POST", "/consumers/c1/pull", Some(body));
    assert_eq!(refused(pull("{}"), 400), "bad_request");
    assert_eq!(refused(pull(r#"{"limit": "ten"}"#), 400), "bad_request");
    assert_eq!(
        client.lease("c1"),
        (204, Value::Null),
        "nothing was claimed"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_request_that_cannot_be_read_is_bad_request() {
    serve_while(bad_requests).await;
}
