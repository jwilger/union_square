//! Monitoring and health check endpoints for projections
//!
//! This module provides HTTP-friendly monitoring data structures
//! that can be easily serialized for health check endpoints.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use super::runner::{HealthStatus, ProjectionHealth};

/// Health check response for projection system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionSystemHealth {
    /// Overall system status
    pub status: SystemStatus,
    /// Individual projection health
    pub projections: Vec<ProjectionHealthDto>,
    /// Summary statistics
    pub summary: HealthSummary,
    /// Timestamp of the health check
    pub checked_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SystemStatus {
    /// All projections healthy
    Healthy,
    /// One or more projections lagging but operational
    Degraded,
    /// One or more projections failed
    Unhealthy,
}

/// DTO for projection health suitable for JSON serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionHealthDto {
    pub name: String,
    pub status: String,
    pub last_checkpoint: Option<String>,
    pub events_processed: u64,
    pub last_error: Option<String>,
    pub lag_seconds: Option<u64>,
}

/// Summary of projection system health
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSummary {
    pub total_projections: usize,
    pub healthy_projections: usize,
    pub lagging_projections: usize,
    pub failed_projections: usize,
    pub total_events_processed: u64,
}

impl From<Vec<ProjectionHealth>> for ProjectionSystemHealth {
    fn from(projections: Vec<ProjectionHealth>) -> Self {
        let mut healthy_count = 0;
        let mut lagging_count = 0;
        let mut failed_count = 0;
        let mut total_events = 0;

        let projection_dtos: Vec<ProjectionHealthDto> = projections
            .iter()
            .map(|p| {
                match p.status {
                    HealthStatus::Healthy => healthy_count += 1,
                    HealthStatus::Lagging => lagging_count += 1,
                    HealthStatus::Failed => failed_count += 1,
                    HealthStatus::Rebuilding => {} // Don't count as failed
                }

                total_events += p.events_processed;

                ProjectionHealthDto {
                    name: p.name.clone(),
                    status: format!("{:?}", p.status),
                    last_checkpoint: p.last_checkpoint.map(|_| {
                        // EventCore timestamps don't expose internal representation
                        chrono::Utc::now().to_rfc3339()
                    }),
                    events_processed: p.events_processed,
                    last_error: p.last_error.clone(),
                    lag_seconds: p.lag.as_ref().map(|d| d.as_secs()),
                }
            })
            .collect();

        let status = if failed_count > 0 {
            SystemStatus::Unhealthy
        } else if lagging_count > 0 {
            SystemStatus::Degraded
        } else {
            SystemStatus::Healthy
        };

        Self {
            status,
            projections: projection_dtos,
            summary: HealthSummary {
                total_projections: projections.len(),
                healthy_projections: healthy_count,
                lagging_projections: lagging_count,
                failed_projections: failed_count,
                total_events_processed: total_events,
            },
            checked_at: chrono::Utc::now(),
        }
    }
}

/// Metrics for Prometheus/OpenTelemetry export
#[derive(Debug, Clone)]
pub struct ProjectionMetrics {
    /// Counter: Total events processed by projection
    pub events_processed: HashMap<String, u64>,
    /// Gauge: Current lag in seconds by projection
    pub lag_seconds: HashMap<String, f64>,
    /// Gauge: Projection status (1 = healthy, 0 = unhealthy)
    pub health_status: HashMap<String, f64>,
    /// Counter: Total errors by projection
    pub error_count: HashMap<String, u64>,
}

impl From<Vec<ProjectionHealth>> for ProjectionMetrics {
    fn from(projections: Vec<ProjectionHealth>) -> Self {
        let mut metrics = ProjectionMetrics {
            events_processed: HashMap::new(),
            lag_seconds: HashMap::new(),
            health_status: HashMap::new(),
            error_count: HashMap::new(),
        };

        for projection in projections {
            metrics
                .events_processed
                .insert(projection.name.clone(), projection.events_processed);

            if let Some(lag) = projection.lag {
                metrics
                    .lag_seconds
                    .insert(projection.name.clone(), lag.as_secs_f64());
            }

            let health_value = match projection.status {
                HealthStatus::Healthy => 1.0,
                _ => 0.0,
            };
            metrics
                .health_status
                .insert(projection.name.clone(), health_value);

            if projection.last_error.is_some() {
                // In production, you'd track error counts properly
                metrics.error_count.insert(projection.name.clone(), 1);
            }
        }

        metrics
    }
}

/// Configuration for alerting thresholds
#[derive(Debug, Clone)]
pub struct AlertThresholds {
    /// Maximum acceptable lag before alerting
    pub max_lag: Duration,
    /// Maximum consecutive errors before alerting
    pub max_consecutive_errors: u32,
    /// Minimum events per minute (for liveness check)
    pub min_events_per_minute: Option<u64>,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            max_lag: Duration::from_secs(300), // 5 minutes
            max_consecutive_errors: 3,
            min_events_per_minute: None, // Disabled by default
        }
    }
}

/// Check if projections meet alert thresholds
pub fn check_alerts(health: &[ProjectionHealth], thresholds: &AlertThresholds) -> Vec<Alert> {
    let mut alerts = Vec::new();

    for projection in health {
        // Check lag threshold
        if let Some(lag) = &projection.lag {
            if lag > &thresholds.max_lag {
                alerts.push(Alert {
                    projection: projection.name.clone(),
                    severity: AlertSeverity::Warning,
                    message: format!(
                        "Projection lag ({:?}) exceeds threshold ({:?})",
                        lag, thresholds.max_lag
                    ),
                });
            }
        }

        // Check failure status
        if matches!(projection.status, HealthStatus::Failed) {
            alerts.push(Alert {
                projection: projection.name.clone(),
                severity: AlertSeverity::Critical,
                message: format!(
                    "Projection failed: {}",
                    projection
                        .last_error
                        .as_ref()
                        .unwrap_or(&"Unknown error".to_string())
                ),
            });
        }

        // TODO: Implement events per minute check when we have time-series data
    }

    alerts
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub projection: String,
    pub severity: AlertSeverity,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::eventcore::projections::runner::HealthStatus;

    #[test]
    fn test_system_health_conversion() {
        let projections = vec![
            ProjectionHealth {
                name: "test1".to_string(),
                status: HealthStatus::Healthy,
                last_checkpoint: None,
                events_processed: 100,
                last_error: None,
                lag: None,
            },
            ProjectionHealth {
                name: "test2".to_string(),
                status: HealthStatus::Lagging,
                last_checkpoint: None,
                events_processed: 50,
                last_error: None,
                lag: Some(Duration::from_secs(120)),
            },
        ];

        let health = ProjectionSystemHealth::from(projections);
        assert_eq!(health.status, SystemStatus::Degraded);
        assert_eq!(health.summary.healthy_projections, 1);
        assert_eq!(health.summary.lagging_projections, 1);
        assert_eq!(health.summary.total_events_processed, 150);
    }

    #[test]
    fn test_alert_generation() {
        let projections = vec![
            ProjectionHealth {
                name: "test1".to_string(),
                status: HealthStatus::Failed,
                last_checkpoint: None,
                events_processed: 0,
                last_error: Some("Connection failed".to_string()),
                lag: None,
            },
            ProjectionHealth {
                name: "test2".to_string(),
                status: HealthStatus::Lagging,
                last_checkpoint: None,
                events_processed: 50,
                last_error: None,
                lag: Some(Duration::from_secs(400)),
            },
        ];

        let thresholds = AlertThresholds::default();
        let alerts = check_alerts(&projections, &thresholds);

        assert_eq!(alerts.len(), 2);
        assert_eq!(alerts[0].severity, AlertSeverity::Critical);
        assert_eq!(alerts[1].severity, AlertSeverity::Warning);
    }
}
