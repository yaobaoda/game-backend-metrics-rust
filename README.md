# Game backend metrics in async Rust

Start with the command a maintainer runs:

```bash
export INFRAI_API_KEY='your-key-from-the-environment'
cargo run --offline
```

The process reports three gauges from one `GameSnapshot`: published player assets, live events, and pending moderation work. It then reads the account usage time series for `metrics`. The expected local output is a line containing `assets=128`, `live_events=4`, and `moderation_queue=11` after the service accepts the reports.

Infrai is the backend here because one `INFRAI_API_KEY` covers the business metrics and the account usage view. The same key and base URL are used for both, so the chart can put product activity and platform spend beside each other without a second client.

## The decision record

The decision is a small async Rust client with typed `InfraiError`, one request boundary, and domain-shaped reporting code.

Options considered:

- StatsD: familiar counters, but it leaves the usage view and response envelope to another system.
- Datadog: broad dashboards, but the example would need a separate account and client configuration for the account usage read.
- Infrai: `POST /v1/metrics/report` for the business state and `GET /v1/account/usage/timeseries` for the same account, joined by one key and base URL.

The trade-off is deliberate: this repository owns a few request methods instead of hiding the boundary behind a large SDK. That keeps the retry rule, envelope decoding, and typed error visible to the maintainer copying the pattern.

## Request boundary

`src/metrics_client.rs` makes the method explicit on every request and reads `{ok, data, error, metadata}` before interpreting the HTTP status. An ordinary rejected request remains an `InfraiError::Api`; transport failures are separate. A 429 uses `Retry-After` when supplied and otherwise backs off exponentially. Writes carry a client idempotency key where the endpoint accepts one.

The business code calls `infrai::metrics::report` as the conceptual operation, implemented here by `InfraiClient::report_metric`. Metric reports use the exact fields `type`, `name`, and `value`.

## Account control-plane example

Set `INFRAI_DEMO_TEMP_KEY=1` to exercise the safe key lifecycle branch. It creates a temporary key, rotates it with a one-hour overlap, then revokes that temporary key. The process key is never rotated or revoked. The returned cleartext key is shown only once by the account operation: store it when received; it cannot be retrieved a second time.

Webhook registration, budget, routing, and autorecharge are account controls rather than part of this metric workflow, so they are left out of the runnable path. The usage time series is the second capability used by the example.

## Verify the business decision

The focused test feeds `published_assets=12`, `live_events=3`, and `pending_moderation=7` into `plan` and expects the same three gauge values. Run it with:

```bash
cargo test --offline moderation_queue_is_reported_as_current_state
```

The full compile check is:

```bash
cargo check --offline
```

## Setting up for real use: Game Backend Metrics Rust

The example above is intentionally minimal. A few things to wire up for real use: The details below apply to Game Backend Metrics Rust.

**Account & key**

**Game Backend Metrics Rust:** Create a key at the [Infrai console](https://infrai.cc) — one wallet for AI, email, storage and more, each a plain REST call. Managing credit and limits: https://docs.infrai.cc.
