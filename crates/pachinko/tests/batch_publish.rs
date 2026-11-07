//! The batch publish route and the request-shape limits end to end: `serve` started
//! in-process on port 0 with the flags each test names, driven by a blocking HTTP
//! client, then stopped. Every assertion reads a reply the server wrote; every wait
//! is bounded. The client here duplicates the one in `tests/http.rs` on purpose, so
//! the two lanes that wrote them stay apart (T27, Open points).

// Test-only from the root, so clippy treats the helpers here as test code, as it
// treats the tests: the unwrap, expect and panic allowances of clippy.toml cover them.
#![cfg(test)]

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
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

/// The limits server: four rows, 16-byte payloads, and the body cap at the load
/// check's minimum for them (112) plus 256.
const LIMITS: [&str; 6] = [
    "--max-batch-rows",
    "4",
    "--max-payload-bytes",
    "16",
    "--max-body-bytes",
    "368",
];
/// The body cap `LIMITS` sets.
const BODY_CAP: usize = 368;

/// A blocking client against one server.
struct Client {
    agent: ureq::Agent,
    address: SocketAddr,
}

impl Client {
    fn new(address: SocketAddr) -> Self {
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(REQUEST_BOUND))
            .build()
            .into();
        Self { agent, address }
    }

    /// Sends one request with a raw JSON body, or none, and returns the status and
    /// the body: JSON when it parses, a string when it does not, `Null` when empty.
    fn send(&self, method: &str, path: &str, body: Option<&str>) -> (u16, Value) {
        let url = format!("http://{}{path}", self.address);
        let response = match (method, body) {
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

    fn join(&self, path: &str) {
        assert_eq!(self.send("POST", path, None), (201, Value::Null), "{path}");
    }

    /// A batch publish by `publisher` of the raw body `body`.
    fn batch_raw(&self, publisher: &str, body: &str) -> (u16, Value) {
        self.send(
            "POST",
            &format!("/publishers/{publisher}/publish"),
            Some(body),
        )
    }

    /// A batch publish by `publisher` of `rows`, each `(pocket, key, rank, payload)`.
    fn batch(&self, publisher: &str, rows: &[(&str, &str, i64, &[u8])]) -> (u16, Value) {
        self.batch_raw(publisher, &batch_body(rows))
    }

    /// A single `PUT` by `p1` of `payload` into pocket `a` under `key` at rank 1.
    fn publish(&self, key: &str, payload: &[u8]) -> (u16, Value) {
        let body = json!({"publisher_id": "p1", "rank": 1, "payload": STANDARD.encode(payload)});
        self.send(
            "PUT",
            &format!("/pockets/a/records/{key}"),
            Some(&body.to_string()),
        )
    }

    fn pull(&self, consumer: &str) -> (u16, Value) {
        let path = format!("/consumers/{consumer}/pull");
        self.send("POST", &path, Some(&json!({"limit": 100}).to_string()))
    }

    /// Sends `head`, then `body` in one chunk with chunked framing, so the request
    /// carries no `Content-Length`, and returns the reply's raw text.
    fn chunked(&self, head: &str, body: &[u8]) -> String {
        let mut stream = TcpStream::connect(self.address).unwrap();
        stream.set_read_timeout(Some(REQUEST_BOUND)).unwrap();
        let mut request = format!(
            "{head}\r\nhost: test\r\ntransfer-encoding: chunked\r\nconnection: close\r\n\r\n{:x}\r\n",
            body.len()
        )
        .into_bytes();
        request.extend(body);
        request.extend(b"\r\n0\r\n\r\n");
        // The server may answer and close before it reads the whole body.
        stream.write_all(&request).ok();
        let mut reply = String::new();
        stream.read_to_string(&mut reply).unwrap();
        reply
    }
}

/// The compact JSON body of a batch of `rows`.
fn batch_body(rows: &[(&str, &str, i64, &[u8])]) -> String {
    let records: Vec<Value> = rows
        .iter()
        .map(|&(pocket, key, rank, payload)| {
            json!({
                "pocket_key": pocket,
                "record_key": key,
                "rank": rank,
                "payload": STANDARD.encode(payload),
            })
        })
        .collect();
    json!({ "records": records }).to_string()
}

/// `body` padded with JSON whitespace to exactly `length` bytes.
fn padded(body: &str, length: usize) -> String {
    assert!(body.len() <= length, "{} bytes already", body.len());
    format!("{body}{}", " ".repeat(length - body.len()))
}

/// Asserts `reply` is the refusal `status` with code `code`, its message containing
/// `says`.
fn assert_refused(reply: &(u16, Value), status: u16, code: &str, says: &str) {
    assert_eq!(
        (reply.0, &reply.1["error"]),
        (status, &json!(code)),
        "{reply:?}"
    );
    let message = reply.1["message"].as_str().unwrap();
    assert!(message.contains(says), "{message:?} lacks {says:?}");
}

/// The `(record_key, rank, payload bytes)` of each record in a pull's batch.
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

/// Joins `p1` and consumer `c1`.
fn joined(client: &Client) {
    client.join("/publishers/p1");
    client.join("/consumers/c1");
}

/// Asserts that `c1` pulls no pocket: nothing landed.
fn assert_nothing_landed(client: &Client) {
    assert_eq!(client.pull("c1"), (200, json!({"outcome": "no_pocket"})));
}

/// Serves on port 0 with `flags` while `drive` runs against it, then stops the
/// server through the stop channel and checks that it returned `Ok` in time.
async fn serve_while(flags: &[&str], drive: fn(&Client)) {
    let mut args = vec!["--listen", "127.0.0.1:0"];
    args.extend_from_slice(flags);
    let config = ServeConfig::parse(args).unwrap();
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

// obligation: surface-provides.Publishing
// obligation: rule-success.PublishIntoNewPocket
// obligation: rule-success.PublishNewKey
// obligation: rule-success.Supersede
// obligation: rule-success.DropStalePublish
fn per_row_outcomes(client: &Client) {
    joined(client);
    let rows: [(&str, &str, i64, &[u8]); 6] = [
        ("a", "k1", 1, b"first"),
        ("a", "k2", 1, b"other"),
        ("a", "k1", 2, b"second"),
        ("a", "k1", 1, b"late"),
        ("a", "k1", 2, b"tie"),
        ("b", "k1", 1, b"b"),
    ];
    assert_eq!(
        client.batch("p1", &rows),
        (
            200,
            json!({"outcomes": ["created", "new_key", "superseded", "stale", "equal_rank", "created"]})
        )
    );
    let (status, batch) = client.pull("c1");
    assert_eq!((status, &batch["pocket_key"]), (200, &json!("a")));
    assert_eq!(
        records(&batch),
        [
            (String::from("k1"), 2, b"second".to_vec()),
            (String::from("k2"), 1, b"other".to_vec()),
        ]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_batch_answers_one_outcome_per_row_in_order_and_applies_rows_independently() {
    serve_while(&[], per_row_outcomes).await;
}

// guarantee: NeverRefusedForCapacity
// obligation: rule-success.PublishIntoNewPocket
fn maximum_requests(client: &Client) {
    joined(client);
    let payload = vec![0xa5; 65_536];
    assert_eq!(
        client.publish("single", &payload),
        (200, json!({"outcome": "created"}))
    );
    let pockets: Vec<String> = (0..64).map(|pocket| format!("pocket-{pocket}")).collect();
    let rows: Vec<(&str, &str, i64, &[u8])> = pockets
        .iter()
        .map(|pocket| (pocket.as_str(), "k", 1, payload.as_slice()))
        .collect();
    let (status, reply) = client.batch("p1", &rows);
    assert_eq!(status, 200, "{reply}");
    assert_eq!(reply, json!({"outcomes": vec!["created"; 64]}));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_largest_single_publish_and_the_largest_batch_the_caps_allow_are_accepted() {
    let flags = [
        "--max-batch-rows",
        "64",
        "--max-payload-bytes",
        "65536",
        "--max-body-bytes",
        "8388608",
    ];
    serve_while(&flags, maximum_requests).await;
}

// obligation: surface-provides.Publishing
fn row_limit(client: &Client) {
    joined(client);
    let row: (&str, &str, i64, &[u8]) = ("a", "k", 1, b"");
    assert_refused(
        &client.batch("p1", &[row; 5]),
        413,
        "too_many_rows",
        "5 rows, over the limit of 4",
    );
    assert_nothing_landed(client);
    let (status, reply) = client.batch("p1", &[row; 4]);
    assert_eq!(
        (status, reply["outcomes"].as_array().unwrap().len()),
        (200, 4)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn more_rows_than_the_limit_are_too_many_rows_and_the_limit_itself_is_accepted() {
    serve_while(&LIMITS, row_limit).await;
}

// obligation: surface-provides.Publishing
fn payload_limit(client: &Client) {
    joined(client);
    let (over, at) = ([7; 17], [7; 16]);
    assert_refused(
        &client.batch("p1", &[("a", "k", 1, &over)]),
        413,
        "payload_too_large",
        "row 1 is 17 bytes",
    );
    assert_refused(
        &client.publish("k", &over),
        413,
        "payload_too_large",
        "row 1 is 17 bytes",
    );
    assert_nothing_landed(client);
    assert_eq!(
        client.batch("p1", &[("a", "k", 1, &at)]),
        (200, json!({"outcomes": ["created"]}))
    );
    assert_eq!(
        client.publish("j", &at),
        (200, json!({"outcome": "new_key"}))
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_payload_over_the_limit_is_payload_too_large_on_both_routes_and_the_limit_is_accepted() {
    serve_while(&LIMITS, payload_limit).await;
}

// obligation: surface-provides.Publishing
fn body_limit(client: &Client) {
    joined(client);
    let body = batch_body(&[("a", "k", 1, b"x")]);
    assert_refused(
        &client.batch_raw("p1", &padded(&body, BODY_CAP + 1)),
        413,
        "body_too_large",
        "limit of 368 bytes",
    );
    let put = json!({"publisher_id": "p1", "rank": 1, "payload": "eA=="}).to_string();
    let (status, reply) = client.send(
        "PUT",
        "/pockets/a/records/k",
        Some(&padded(&put, BODY_CAP + 1)),
    );
    assert_eq!((status, &reply["error"]), (413, &json!("body_too_large")));
    assert_nothing_landed(client);
    assert_eq!(
        client.batch_raw("p1", &padded(&body, BODY_CAP)),
        (200, json!({"outcomes": ["created"]}))
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_body_over_the_cap_is_body_too_large_and_a_body_at_the_cap_is_accepted() {
    serve_while(&LIMITS, body_limit).await;
}

// obligation: surface-provides.Publishing
fn streamed_body(client: &Client) {
    joined(client);
    let head = "POST /publishers/p1/publish HTTP/1.1\r\ncontent-type: text/plain";
    let reply = client.chunked(head, &[b' '; BODY_CAP + 32]);
    assert!(reply.starts_with("HTTP/1.1 413"), "{reply}");
    assert!(reply.contains(r#""error":"body_too_large""#), "{reply}");
    assert_nothing_landed(client);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_chunked_body_over_the_cap_is_body_too_large_whatever_its_content_type() {
    serve_while(&LIMITS, streamed_body).await;
}

// obligation: surface-actor.Publishing
fn unjoined_publisher(client: &Client) {
    client.join("/consumers/c1");
    let row: (&str, &str, i64, &[u8]) = ("a", "k", 1, b"x");
    assert_refused(
        &client.batch("nobody", &[row, ("b", "k", 1, b"y")]),
        404,
        "unknown_publisher",
        "publisher nobody has not joined",
    );
    assert_nothing_landed(client);
    assert_refused(
        &client.batch("nobody", &[row; 5]),
        413,
        "too_many_rows",
        "5 rows",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unjoined_publishers_batch_lands_nothing_and_shape_checks_come_first() {
    serve_while(&LIMITS, unjoined_publisher).await;
}

// obligation: surface-provides.Publishing
fn late_row_refusals(client: &Client) {
    joined(client);
    let not_base64 = json!({"records": [
        {"pocket_key": "a", "record_key": "k", "rank": 1, "payload": "eA=="},
        {"pocket_key": "b", "record_key": "k", "rank": 1, "payload": "not base64!"},
    ]});
    assert_refused(
        &client.batch_raw("p1", &not_base64.to_string()),
        400,
        "bad_request",
        "row 2: payload is not base64",
    );
    assert_nothing_landed(client);
    assert_refused(
        &client.batch("p1", &[("a", "k", 1, b"x"), ("b", "k", 1, &[7; 17])]),
        413,
        "payload_too_large",
        "row 2 is 17 bytes",
    );
    assert_nothing_landed(client);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_refusal_of_a_later_row_leaves_the_valid_first_row_unapplied() {
    serve_while(&LIMITS, late_row_refusals).await;
}

// obligation: surface-provides.Publishing
fn malformed(client: &Client) {
    joined(client);
    for body in [
        String::from("{"),
        String::from(r#"{"records": []}"#),
        String::from(r#"{"rows": []}"#),
        json!({"records": [{"pocket_key": "a", "record_key": "k", "rank": 1, "payload": "%%"}]})
            .to_string(),
    ] {
        let (status, reply) = client.batch_raw("p1", &body);
        assert_eq!(
            (status, &reply["error"]),
            (400, &json!("bad_request")),
            "{body}"
        );
    }
    assert_nothing_landed(client);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn malformed_json_an_empty_batch_and_a_non_base64_payload_are_bad_requests() {
    serve_while(&[], malformed).await;
}

// obligation: surface-provides.Publishing
fn malformed_content_type(client: &Client) {
    joined(client);
    let url = format!("http://{}/publishers/p1/publish", client.address);
    let mut response = client
        .agent
        .post(&url)
        .header("content-type", "application/invalid subtype+json")
        .send(batch_body(&[("a", "k", 1, b"x")]))
        .unwrap();
    let status = response.status().as_u16();
    let reply: Value =
        serde_json::from_str(&response.body_mut().read_to_string().unwrap()).unwrap();
    assert_refused(&(status, reply), 400, "bad_request", "Content-Type");
    assert_nothing_landed(client);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_malformed_json_content_type_is_a_bad_request() {
    serve_while(&[], malformed_content_type).await;
}
