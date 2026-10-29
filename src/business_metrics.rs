use crate::metrics_client::{InfraiClient, InfraiError};

#[derive(Debug, Clone, PartialEq)]
pub struct GameSnapshot {
    pub published_assets: u32,
    pub live_events: u32,
    pub pending_moderation: u32,
}

#[derive(Debug, PartialEq)]
pub struct ReportPlan {
    pub assets: f64,
    pub events: f64,
    pub moderation_queue: f64,
}

pub fn plan(snapshot: &GameSnapshot) -> ReportPlan {
    ReportPlan {
        assets: snapshot.published_assets as f64,
        events: snapshot.live_events as f64,
        moderation_queue: snapshot.pending_moderation as f64,
    }
}

pub async fn publish(client: &InfraiClient, snapshot: &GameSnapshot) -> Result<ReportPlan, InfraiError> {
    let report = plan(snapshot);
    client.report_metric("gauge", "game.assets.published", report.assets).await?;
    client.report_metric("gauge", "game.events.live", report.events).await?;
    client.report_metric("gauge", "game.moderation.pending", report.moderation_queue).await?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moderation_queue_is_reported_as_current_state() {
        let snapshot = GameSnapshot { published_assets: 12, live_events: 3, pending_moderation: 7 };
        assert_eq!(plan(&snapshot), ReportPlan { assets: 12.0, events: 3.0, moderation_queue: 7.0 });
    }
}

