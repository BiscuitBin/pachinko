//! The REST API end to end: `serve` started in-process on port 0, driven by a
//! blocking HTTP client as a publisher and consumers would drive it, then stopped.

// Test-only from the root, so clippy treats the helpers below as test code, as it
// treats the tests: the unwrap, expect and panic allowances of clippy.toml cover them.
#![cfg(test)]

use std::time::{Duration, Instant};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use pachinko::ServeConfig;
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio::sync::watch;

/// The lapse test's lease: short, because nothing in that test has to happen before
/// it ends. The scenario keeps the module's default lease, five minutes, so none of its
/// assertions depends on how fast the runner is.
const LAPSING_LEASE_TTL_MS: &str = "100";
const SWEEP_MS: &str = "20";
/// How long a poll waits for the sweep before the test fails.
const DEADLINE: Duration = Duration::from_secs(15);

/// A blocking client against one server.
struct Client {
    agent: ureq::Agent,
    base: String,
}

impl Client {
    fn new(address: std::net::SocketAddr) -> Self {
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .build()
            .into();
        Self {
            agent,
            base: format!("http://{address}"),
        }
    }

    /// Sends one request and returns the status and the JSON body (`Null` if none).
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

    fn publish(&self, pocket: &str, key: &str, rank: i64, payload: &str) -> (u16, Value) {
        let body = json!({"publisher_id": "p1", "rank": rank, "payload": STANDARD.encode(payload)});
        self.json("PUT", &format!("/pockets/{pocket}/records/{key}"), &body)
    }

    fn pull(&self, consumer: &str, limit: i64) -> (u16, Value) {
        self.json(
            "POST",
            &format!("/consumers/{consumer}/pull"),
            &json!({"limit": limit}),
        )
    }

    fn settle(&self, action: &str, lease: &str, consumer: &str, key: &str) -> (u16, Value) {
        let body = json!({"consumer_id": consumer, "record_key": key});
        self.json("POST", &format!("/leases/{lease}/{action}"), &body)
    }

    fn lease_action(&self, action: &str, lease: &str, consumer: &str) -> (u16, Value) {
        self.json(
            "POST",
            &format!("/leases/{lease}/{action}"),
            &json!({"consumer_id": consumer}),
        )
    }
}

fn keys_and_payloads(batch: &Value) -> Vec<(String, i64, String)> {
    batch["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            let payload = STANDARD.decode(r["payload"].as_str().unwrap()).unwrap();
            (
                r["record_key"].as_str().unwrap().to_owned(),
                r["rank"].as_i64().unwrap(),
                String::from_utf8(payload).unwrap(),
            )
        })
        .collect()
}

fn record(key: &str, rank: i64, payload: &str) -> (String, i64, String) {
    (key.to_owned(), rank, payload.to_owned())
}

/// Joins p1, c1, c2 and c3, and shows a second join refused.
fn joins(client: &Client) {
    assert_eq!(client.send("GET", "/healthz", None).0, 200);
    for path in [
        "/publishers/p1",
        "/consumers/c1",
        "/consumers/c2",
        "/consumers/c3",
    ] {
        assert_eq!(
            client.send("POST", path, None),
            (201, Value::Null),
            "{path}"
        );
    }
    let (status, body) = client.send("POST", "/publishers/p1", None);
    assert_eq!((status, &body["error"]), (409, &json!("publisher_exists")));
    let (status, body) = client.send("POST", "/consumers/c1", None);
    assert_eq!((status, &body["error"]), (409, &json!("consumer_exists")));
}

/// Three records into two pockets; c1 gets `a` oldest first, c2 `b`, c3 nothing.
fn claims(client: &Client) -> (String, String) {
    assert_eq!(
        client.publish("a", "k1", 1, "a1@1").1,
        json!({"outcome": "created"})
    );
    assert_eq!(
        client.publish("a", "k2", 1, "a2@1").1,
        json!({"outcome": "new_key"})
    );
    assert_eq!(
        client.publish("b", "k1", 1, "b1@1").1,
        json!({"outcome": "created"})
    );
    let (status, c1) = client.pull("c1", 10);
    assert_eq!((status, &c1["pocket_key"]), (200, &json!("a")));
    assert_eq!(
        keys_and_payloads(&c1),
        [record("k1", 1, "a1@1"), record("k2", 1, "a2@1")],
        "oldest first"
    );
    let (_, c2) = client.pull("c2", 10);
    assert_eq!(c2["pocket_key"], json!("b"), "a is exclusive to c1");
    assert_eq!(
        client.pull("c3", 10),
        (200, json!({"no_pocket_available": true}))
    );
    let lease = |batch: &Value| batch["lease_id"].as_str().unwrap().to_owned();
    (lease(&c1), lease(&c2))
}

/// Supersede, stale, equal rank and unknown publisher, while c1 holds k1.
fn publishes(client: &Client) {
    assert_eq!(
        client.publish("a", "k1", 2, "a1@2").1,
        json!({"outcome": "superseded"})
    );
    assert_eq!(
        client.publish("a", "k1", 0, "a1@0").1,
        json!({"outcome": "stale"})
    );
    let (status, body) = client.publish("a", "k1", 2, "a1@2 again");
    assert_eq!((status, &body["error"]), (409, &json!("equal_rank")));
    assert!(
        body["message"]
            .as_str()
            .unwrap()
            .contains("first open question"),
        "{body}"
    );
    let body = json!({"publisher_id": "nobody", "rank": 1, "payload": ""});
    let (status, body) = client.json("PUT", "/pockets/a/records/k9", &body);
    assert_eq!((status, &body["error"]), (404, &json!("unknown_publisher")));
}

/// c1 acks the superseded k1 (deferred), nacks k2, renews, reads its lease, releases,
/// and is refused under the released lease.
fn settles(client: &Client, l1: &str, l2: &str) {
    assert_eq!(
        client.settle("ack", l1, "c1", "k1"),
        (200, json!({"outcome": "deferred"}))
    );
    assert_eq!(
        client.settle("nack", l1, "c1", "k2"),
        (200, json!({"outcome": "settled"}))
    );
    let (status, body) = client.settle("ack", l1, "c1", "k2");
    assert_eq!((status, &body["error"]), (404, &json!("record_not_held")));
    let (status, body) = client.lease_action("renew", l1, "c1");
    assert_eq!(status, 200);
    let renewed = body["expires_at"].as_u64().unwrap();
    let (status, lease) = client.send("GET", "/consumers/c1/lease", None);
    assert_eq!(status, 200);
    assert_eq!(
        (&lease["lease_id"], &lease["pocket_key"]),
        (&json!(l1), &json!("a"))
    );
    assert_eq!(
        (lease["expires_at"].as_u64(), &lease["records"]),
        (Some(renewed), &json!([]))
    );
    let (status, body) = client.lease_action("renew", l2, "c1");
    assert_eq!((status, &body["error"]), (403, &json!("lease_not_yours")));
    assert_eq!(
        client.lease_action("release", l1, "c1"),
        (200, json!({"pocket": "queued"}))
    );
    let (status, body) = client.settle("ack", l1, "c1", "k1");
    assert_eq!((status, &body["error"]), (409, &json!("lease_not_active")));
}

/// The deferred version returns once c1's lease has ended, and c3 settles it; c2
/// renews its own lease first.
fn deferred_returns(client: &Client, l2: &str) {
    assert_eq!(client.lease_action("renew", l2, "c2").0, 200);
    let (_, c3) = client.pull("c3", 10);
    assert_eq!(c3["pocket_key"], json!("a"));
    assert_eq!(keys_and_payloads(&c3), [record("k1", 2, "a1@2")]);
    let lease = c3["lease_id"].as_str().unwrap();
    assert_eq!(
        client.settle("ack", lease, "c3", "k1"),
        (200, json!({"outcome": "settled"}))
    );
}

/// Malformed requests are 400 `bad_request`; unknown consumers 404.
fn refusals(client: &Client) {
    let (status, body) = client.send("POST", "/consumers/c1/pull", Some("{\"limit\":"));
    assert_eq!((status, &body["error"]), (400, &json!("bad_request")));
    let (status, body) = client.lease_action("renew", "not-a-lease", "c1");
    assert_eq!((status, &body["error"]), (400, &json!("bad_request")));
    let body = json!({"publisher_id": "p1", "rank": 1, "payload": "%%%"});
    let (status, body) = client.json("PUT", "/pockets/a/records/k1", &body);
    assert_eq!((status, &body["error"]), (400, &json!("bad_request")));
    let (status, body) = client.pull("nobody", 1);
    assert_eq!((status, &body["error"]), (404, &json!("unknown_consumer")));
    let (status, body) = client.send("GET", "/consumers/nobody/lease", None);
    assert_eq!((status, &body["error"]), (404, &json!("unknown_consumer")));
}

/// c2 claims `b` and never acts again, so the sweep lapses its lease; polls until it
/// has, or fails at the deadline, then c1 claims `b` again.
fn lapse_frees(client: &Client) {
    for path in ["/publishers/p1", "/consumers/c1", "/consumers/c2"] {
        assert_eq!(client.send("POST", path, None).0, 201, "{path}");
    }
    assert_eq!(
        client.publish("b", "k1", 1, "b1@1").1,
        json!({"outcome": "created"})
    );
    let (_, c2) = client.pull("c2", 10);
    assert_eq!(c2["pocket_key"], json!("b"));
    let deadline = Instant::now() + DEADLINE;
    loop {
        let (status, _) = client.send("GET", "/consumers/c2/lease", None);
        if status == 204 {
            break;
        }
        assert_eq!(status, 200);
        assert!(
            Instant::now() < deadline,
            "the sweep did not lapse c2's lease in time"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
    let (_, c1) = client.pull("c1", 10);
    assert_eq!(
        c1["pocket_key"],
        json!("b"),
        "the lapsed pocket is pullable again"
    );
    assert_eq!(keys_and_payloads(&c1), [record("k1", 1, "b1@1")]);
}

fn scenario(client: &Client) {
    joins(client);
    let (l1, l2) = claims(client);
    publishes(client);
    settles(client, &l1, &l2);
    deferred_returns(client, &l2);
    refusals(client);
}

/// Serves on port 0 with `flags` while `drive` runs against it, then stops the server
/// and checks that it returned `Ok` within the grace period.
async fn serve_while(flags: &[&str], drive: fn(&Client)) {
    let mut args = vec!["--listen", "127.0.0.1:0", "--sweep-ms", SWEEP_MS];
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
    let (served, stop_sent) = tokio::join!(pachinko::serve(listener, &config, stopped), client);
    assert_eq!(served, Ok(()));
    assert!(
        stop_sent.elapsed() < Duration::from_secs(10),
        "stopped within the grace period"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_publisher_and_consumers_drive_the_queue_over_http_and_it_stops_cleanly() {
    serve_while(&[], scenario).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_sweep_frees_a_lapsed_lease_and_the_server_stops_cleanly() {
    serve_while(&["--lease-ttl-ms", LAPSING_LEASE_TTL_MS], lapse_frees).await;
}
