mod business_metrics;
mod metrics_client;

use business_metrics::{publish, GameSnapshot};
use metrics_client::InfraiClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = InfraiClient::from_env()?;
    let snapshot = GameSnapshot { published_assets: 128, live_events: 4, pending_moderation: 11 };
    let report = publish(&client, &snapshot).await?;
    let _usage = client.usage_timeseries("metrics").await?;
    println!("reported assets={}, live_events={}, moderation_queue={}", report.assets, report.events, report.moderation_queue);

    if std::env::var("INFRAI_DEMO_TEMP_KEY").is_ok() {
        let created = client.create_key("metrics-demo-temporary").await?;
        if let Some(id) = created.get("id").and_then(|v| v.as_str()) {
            let _rotated = client.rotate_key(id).await?;
            client.revoke_key(id).await?;
            println!("temporary control-plane key rotated and revoked");
        }
    }
    Ok(())
}

