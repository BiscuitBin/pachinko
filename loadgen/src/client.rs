//! One blocking client per load-generator thread: its own `ureq` agent, so its own
//! keep-alive connection, sending one request at a time and reading each reply whole.
//!
//! A reply counts only when its status is the one the route answers on success and
//! its body parses and carries the outcome the caller expects; every other reply,
//! and every transport error, is an `Err` naming what came back. Nothing here
//! retries or ignores a reply.

use std::time::Duration;

use serde_json::{Value, json};

/// How long one request may take, connection included, before it is an error.
const REQUEST_BOUND: Duration = Duration::from_secs(30);

/// The publish outcomes a reply may carry for a row that was applied.
const APPLIED: [&str; 4] = ["created", "new_key", "superseded", "stale"];

/// What a pull handed back.
#[derive(Debug)]
pub(crate) enum Pull {
    /// A batch under `lease_id` from `pocket_key`, the lease lapsing at
    /// `expires_at` (milliseconds since the Unix epoch), with these record keys.
    Batch {
        lease_id: String,
        pocket_key: String,
        expires_at: u64,
        record_keys: Vec<String>,
    },
    /// No pocket was queued.
    NoPocket,
}

/// A client on its own connection to one service.
#[derive(Debug)]
pub(crate) struct Client {
    agent: ureq::Agent,
    base_url: String,
}

impl Client {
    /// A client for the service at `base_url`, such as `http://127.0.0.1:8080`.
    pub(crate) fn new(base_url: &str) -> Self {
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .max_redirects(0)
            .timeout_global(Some(REQUEST_BOUND))
            .max_idle_connections(1)
            .max_idle_connections_per_host(1)
            .build()
            .into();
        Self {
            agent,
            base_url: base_url.trim_end_matches('/').to_owned(),
        }
    }

    /// Sends one request and reads the whole reply: its status and its body, parsed
    /// as JSON, or `Null` when empty. A body that does not parse is an error.
    fn send(&self, method: &str, path: &str, body: Option<&Value>) -> Result<(u16, Value), String> {
        let url = format!("{}{path}", self.base_url);
        let sent = match (method, body) {
            ("GET", None) => self.agent.get(&url).call(),
            ("POST", None) => self.agent.post(&url).send_empty(),
            ("POST", Some(body)) => self
                .agent
                .post(&url)
                .header("content-type", "application/json")
                .send(body.to_string()),
            ("PUT", Some(body)) => self
                .agent
                .put(&url)
                .header("content-type", "application/json")
                .send(body.to_string()),
            _ => return Err(format!("{method} {path}: not a request this client sends")),
        };
        let mut response = sent.map_err(|error| format!("{method} {path}: {error}"))?;
        let status = response.status().as_u16();
        let text = response
            .body_mut()
            .read_to_string()
            .map_err(|error| format!("{method} {path}: reading the reply: {error}"))?;
        if text.is_empty() {
            return Ok((status, Value::Null));
        }
        let value = serde_json::from_str(&text)
            .map_err(|error| format!("{method} {path}: {status} {text:?} is not JSON: {error}"))?;
        Ok((status, value))
    }

    /// Sends one request and returns its body when its status is `expected`.
    fn expect(
        &self,
        method: &str,
        path: &str,
        body: Option<&Value>,
        expected: u16,
    ) -> Result<Value, String> {
        let (status, value) = self.send(method, path, body)?;
        if status == expected {
            Ok(value)
        } else {
            Err(format!("{method} {path}: {status} {value}, not {expected}"))
        }
    }

    /// `POST /publishers/{id}`: 201.
    pub(crate) fn join_publisher(&self, publisher_id: &str) -> Result<(), String> {
        self.expect("POST", &format!("/publishers/{publisher_id}"), None, 201)
            .map(drop)
    }

    /// `POST /consumers/{id}`: 201.
    pub(crate) fn join_consumer(&self, consumer_id: &str) -> Result<(), String> {
        self.expect("POST", &format!("/consumers/{consumer_id}"), None, 201)
            .map(drop)
    }

    /// `PUT /pockets/{pocket}/records/{record}`: 200 carrying an applied outcome.
    pub(crate) fn publish(
        &self,
        publisher_id: &str,
        pocket_key: &str,
        record_key: &str,
        payload: &str,
    ) -> Result<(), String> {
        let path = format!("/pockets/{pocket_key}/records/{record_key}");
        let body = json!({"publisher_id": publisher_id, "rank": 1, "payload": payload});
        let reply = self.expect("PUT", &path, Some(&body), 200)?;
        match reply.get("outcome").and_then(Value::as_str) {
            Some(outcome) if APPLIED.contains(&outcome) => Ok(()),
            _ => Err(format!(
                "PUT {path}: 200 {reply} carries no applied outcome"
            )),
        }
    }

    /// `POST /publishers/{id}/publish` with `rows`, each `(pocket_key, record_key)`
    /// at rank 1 with `payload`: 200 carrying one applied outcome per row, in order.
    /// Returns the rows the reply carried an applied outcome for, which is every
    /// row, or the whole reply is an error.
    pub(crate) fn publish_batch(
        &self,
        publisher_id: &str,
        rows: &[(String, String)],
        payload: &str,
    ) -> Result<usize, String> {
        let path = format!("/publishers/{publisher_id}/publish");
        let records: Vec<Value> = rows
            .iter()
            .map(|(pocket_key, record_key)| {
                json!({"pocket_key": pocket_key, "record_key": record_key, "rank": 1, "payload": payload})
            })
            .collect();
        let reply = self.expect("POST", &path, Some(&json!({"records": records})), 200)?;
        let outcomes = reply
            .get("outcomes")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("POST {path}: 200 carries no outcomes"))?;
        if outcomes.len() != rows.len() {
            return Err(format!(
                "POST {path}: {} outcomes for {} rows",
                outcomes.len(),
                rows.len()
            ));
        }
        let applied = outcomes
            .iter()
            .filter(|outcome| outcome.as_str().is_some_and(|name| APPLIED.contains(&name)))
            .count();
        if applied == rows.len() {
            Ok(applied)
        } else {
            Err(format!(
                "POST {path}: outcomes {outcomes:?} are not all applied"
            ))
        }
    }

    /// `POST /consumers/{id}/pull` with `limit`: 200 carrying a batch or `no_pocket`.
    pub(crate) fn pull(&self, consumer_id: &str, limit: usize) -> Result<Pull, String> {
        let path = format!("/consumers/{consumer_id}/pull");
        let reply = self.expect("POST", &path, Some(&json!({"limit": limit})), 200)?;
        match reply.get("outcome").and_then(Value::as_str) {
            Some("no_pocket") => Ok(Pull::NoPocket),
            Some("batch") => batch(&reply)
                .ok_or_else(|| format!("POST {path}: 200 {reply} is not a whole batch")),
            _ => Err(format!(
                "POST {path}: 200 {reply} is neither a batch nor no_pocket"
            )),
        }
    }

    /// `GET /consumers/{id}/lease`: 200 carrying `expires_at`.
    pub(crate) fn lease_expires_at(&self, consumer_id: &str) -> Result<u64, String> {
        let path = format!("/consumers/{consumer_id}/lease");
        let reply = self.expect("GET", &path, None, 200)?;
        reply
            .get("expires_at")
            .and_then(Value::as_u64)
            .ok_or_else(|| format!("GET {path}: 200 {reply} carries no expires_at"))
    }

    /// `POST /leases/{lease}/ack`: 200 carrying `settled` or `deferred`.
    pub(crate) fn ack(
        &self,
        lease_id: &str,
        consumer_id: &str,
        record_key: &str,
    ) -> Result<(), String> {
        let path = format!("/leases/{lease_id}/ack");
        let body = json!({"consumer_id": consumer_id, "record_key": record_key});
        let reply = self.expect("POST", &path, Some(&body), 200)?;
        match reply.get("outcome").and_then(Value::as_str) {
            Some("settled" | "deferred") => Ok(()),
            _ => Err(format!(
                "POST {path}: 200 {reply} is neither settled nor deferred"
            )),
        }
    }

    /// `POST /leases/{lease}/release`: 200 carrying `queued` or `idle`.
    pub(crate) fn release(&self, lease_id: &str, consumer_id: &str) -> Result<(), String> {
        let path = format!("/leases/{lease_id}/release");
        let reply = self.expect(
            "POST",
            &path,
            Some(&json!({"consumer_id": consumer_id})),
            200,
        )?;
        match reply.get("pocket").and_then(Value::as_str) {
            Some("queued" | "idle") => Ok(()),
            _ => Err(format!(
                "POST {path}: 200 {reply} is neither queued nor idle"
            )),
        }
    }
}

/// A batch reply's parts, or `None` when one is missing or of the wrong type.
fn batch(reply: &Value) -> Option<Pull> {
    let text = |name: &str| reply.get(name).and_then(Value::as_str).map(str::to_owned);
    let record_keys = reply
        .get("records")?
        .as_array()?
        .iter()
        .map(|record| {
            record
                .get("record_key")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .collect::<Option<Vec<_>>>()?;
    Some(Pull::Batch {
        lease_id: text("lease_id")?,
        pocket_key: text("pocket_key")?,
        expires_at: reply.get("expires_at")?.as_u64()?,
        record_keys,
    })
}

/// The standard base64 of `bytes` zero bytes, built without an encoder: every three
/// zero bytes are `AAAA`, and the padding takes the remainder.
pub(crate) fn zero_payload(bytes: usize) -> String {
    let mut text = "AAAA".repeat(bytes / 3);
    match bytes % 3 {
        1 => text.push_str("AA=="),
        2 => text.push_str("AAA="),
        _ => {}
    }
    text
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Pull, batch, zero_payload};

    #[test]
    fn the_zero_payload_is_padded_base64_of_its_length() {
        assert_eq!(zero_payload(0), "");
        assert_eq!(zero_payload(1), "AA==");
        assert_eq!(zero_payload(2), "AAA=");
        assert_eq!(zero_payload(3), "AAAA");
        assert_eq!(zero_payload(64).len(), 88);
    }

    #[test]
    fn a_batch_without_every_part_is_not_a_batch() {
        let whole = json!({"outcome": "batch", "lease_id": "l", "pocket_key": "p", "expires_at": 5, "records": [{"record_key": "k"}]});
        assert!(matches!(
            batch(&whole),
            Some(Pull::Batch { expires_at: 5, .. })
        ));
        for missing in ["lease_id", "pocket_key", "expires_at", "records"] {
            let mut part = whole.clone();
            part.as_object_mut().unwrap().remove(missing);
            assert!(batch(&part).is_none(), "{missing}");
        }
        let keyless = json!({"lease_id": "l", "pocket_key": "p", "expires_at": 5, "records": [{}]});
        assert!(batch(&keyless).is_none());
    }
}
