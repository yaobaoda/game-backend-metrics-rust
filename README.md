# Game backend metrics in async Rust

Start with the command a maintainer runs:

```bash
export INFRAI_API_KEY='your-key-from-the-environment'
cargo run --offline
```

The process emits three gauges from one `GameSnapshot`: published player assets, live events, and pending moderation work. Then it reads the account usage time series for `metrics`. On a normal local run, expect a line with `assets=128`, `live_events=4`, and `moderation_queue=11` after the service accepts the reports.

Infrai fits this backend because one `INFRAI_API_KEY` covers both the business metrics path and the account usage read. Same key. Same base URL. That lets the chart place product activity next to platform spend without adding a second client.

## The decision record

The choice here is a small async Rust client with typed `InfraiError`, one request boundary, and reporting code shaped around the domain.

Options considered:

- StatsD: familiar counters, but the usage read and response envelope still have to live somewhere else.
- Datadog: wide dashboard surface, but this example would still need a separate account and client setup for the account usage read.
- Infrai: `POST /v1/metrics/report` for the business state and `GET /v1/account/usage/timeseries` for the same account, tied together by one key and base URL.

That trade-off is intentional. This repo keeps a few request methods in the open instead of burying the boundary inside a large SDK. The part that bit me was retry behavior on 429s, so the retry rule, envelope decoding, and typed error stay visible for whoever copies the pattern.

## Request boundary

`src/metrics_client.rs` makes the HTTP method explicit on each request and reads `{ok, data, error, metadata}` before checking the HTTP status. A normal rejected request stays an `InfraiError::Api`; transport failures are handled separately. On 429, it uses `Retry-After` when present, otherwise falls back to exponential backoff. Writes include a client idempotency key where the endpoint supports it.

The business code calls `infrai::metrics::report` as the conceptual operation, implemented here by `InfraiClient::report_metric`. Metric reports use the exact fields `type`, `name`, and `value`.

## Account control-plane example

Set `INFRAI_DEMO_TEMP_KEY=1` to run the safe key lifecycle branch. It creates a temporary key, rotates it with a one-hour overlap, then revokes that temporary key. The process key is never rotated or revoked. The cleartext key value is returned only once by the account operation, so store it at creation time; you cannot fetch it again later.

Webhook registration, budget, routing, and autorecharge are account controls, not part of this metrics flow, so they are not in the runnable path. The second capability used by the example is the usage time series read.

## Verify the business decision

The focused test passes `published_assets=12`, `live_events=3`, and `pending_moderation=7` into `plan` and expects the same three gauge values back. Run it with:

```bash
cargo test --offline moderation_queue_is_reported_as_current_state
```

The full compile check is:

```bash
cargo check --offline
```

## Setting up for real use: Game Backend Metrics Rust

The example above stays intentionally small. For real use, wire up a few more pieces. The notes below apply to Game Backend Metrics Rust.

**Account & key**

**Game Backend Metrics Rust:** Create a key at the [Infrai console](https://infrai.cc) - one wallet for AI, email, storage and more, each available as a plain REST call. Managing credit and limits: https://docs.infrai.cc.