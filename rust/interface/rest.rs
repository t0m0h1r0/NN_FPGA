//! REST API interface implementation
//!
//! This module provides a RESTful API for interacting with the accelerator.

use std::sync::Arc;
use axum::{
    Router,
    routing::{get, post, delete},
    extract::{State, Path, Json},
    response::{IntoResponse, Response},
    http::StatusCode,
};
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;
use tracing::{info, error};

use crate::domain::{
    operation::{Operation, UnitId, OperationStatus},
    error::{Result, DomainError},
};
use crate::app::{
    executor::OperationExecutor,
    scheduler::{Scheduler, Priority},
    monitor::{Monitor, SystemStatus},
};

/// Application state
pub struct AppState {
    scheduler: Arc<Scheduler>,
    monitor: Arc<Monitor>,
}

/// Operation request
#[derive(Debug, Serialize, Deserialize)]
pub struct OperationRequest {
    /// Operation to execute
    operation: Operation,
    /// Target unit
    unit_id: u8,
    /// Operation priority
    priority: Priority,
}

/// Operation response
#[derive(Debug, Serialize)]
pub struct OperationResponse {
    /// Operation ID
    operation_id: String,
    /// Operation status
    status: OperationStatus,
    /// Estimated completion time
    eta: Option<u64>,
}

/// System status response
#[derive(Debug, Serialize)]
pub struct StatusResponse {
    /// System status
    status: SystemStatus,
    /// API version
    version: String,
}

/// API error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Error message
    message: String,
    /// Error code
    code: String,
    /// Additional details
    details: Option<String>,
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        let status = match self.code.as_str() {
            "INVALID_REQUEST" => StatusCode::BAD_REQUEST,
            "NOT_FOUND" => StatusCode::NOT_FOUND,
            "CONFLICT" => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = Json(self);
        (status, body).into_response()
    }
}

/// Create API router
pub fn create_router(
    scheduler: Arc<Scheduler>,
    monitor: Arc<Monitor>,
) -> Router {
    let state = Arc::new(AppState {
        scheduler,
        monitor,
    });

    Router::new()
        .route("/api/v1/operations", post(submit_operation))
        .route("/api/v1/operations/:id", get(get_operation))
        .route("/api/v1/operations/:id", delete(cancel_operation))
        .route("/api/v1/units/:id/status", get(get_unit_status))
        .route("/api/v1/system/status", get(get_system_status))
        .with_state(state)
}

/// Submit new operation
async fn submit_operation(
    State(state): State<Arc<AppState>>,
    Json(request): Json<OperationRequest>,
) -> impl IntoResponse {
    info!("Submitting operation: {:?}", request);

    let unit_id = match UnitId::new(request.unit_id) {
        Some(id) => id,
        None => {
            return ErrorResponse {
                message: "Invalid unit ID".into(),
                code: "INVALID_REQUEST".into(),
                details: Some(format!("Unit ID {} is out of range", request.unit_id)),
            }.into_response();
        }
    };

    match state.scheduler.schedule(
        request.operation,
        unit_id,
        request.priority,
    ).await {
        Ok(()) => {
            let response = OperationResponse {
                operation_id: uuid::Uuid::new_v4().to_string(),
                status: OperationStatus::Success,
                eta: Some(1000), // Estimated milliseconds
            };
            (StatusCode::ACCEPTED, Json(response)).into_response()
        }
        Err(e) => ErrorResponse {
            message: "Failed to schedule operation".into(),
            code: "INTERNAL_ERROR".into(),
            details: Some(e.to_string()),
        }.into_response(),
    }
}

/// Get operation status
async fn get_operation(
    State(state): State<Arc<AppState>>,
    Path(operation_id): Path<String>,
) -> impl IntoResponse {
    info!("Getting operation status: {}", operation_id);

    // In a real implementation, we would look up the operation status
    // For now, return a mock response
    let response = OperationResponse {
        operation_id,
        status: OperationStatus::Success,
        eta: None,
    };

    (StatusCode::OK, Json(response)).into_response()
}

/// Cancel operation
async fn cancel_operation(
    State(state): State<Arc<AppState>>,
    Path(operation_id): Path<String>,
) -> impl IntoResponse {
    info!("Cancelling operation: {}", operation_id);

    // In a real implementation, we would cancel the specific operation
    // For now, return success
    StatusCode::NO_CONTENT.into_response()
}

/// Get unit status
async fn get_unit_status(
    State(state): State<Arc<AppState>>,
    Path(unit_id): Path<u8>,
) -> impl IntoResponse {
    info!("Getting unit status: {}", unit_id);

    let unit_id = match UnitId::new(unit_id) {
        Some(id) => id,
        None => {
            return ErrorResponse {
                message: "Invalid unit ID".into(),
                code: "INVALID_REQUEST".into(),
                details: Some(format!("Unit ID {} is out of range", unit_id)),
            }.into_response();
        }
    };

    let queue_status = state.scheduler.queue_status(unit_id).await;
    (StatusCode::OK, Json(queue_status)).into_response()
}

/// Get system status
async fn get_system_status(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    info!("Getting system status");

    match state.monitor.status_receiver().borrow().clone() {
        status => {
            let response = StatusResponse {
                status,
                version: env!("CARGO_PKG_VERSION").into(),
            };
            (StatusCode::OK, Json(response)).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_submit_operation() {
        // Create test app
        let scheduler = Arc::new(Scheduler::default());
        let monitor = Arc::new(Monitor::default());
        let app = create_router(scheduler, monitor);

        // Create test request
        let request = OperationRequest {
            operation: Operation::Copy {
                source: UnitId::new(0).unwrap(),
            },
            unit_id: 1,
            priority: Priority::Normal,
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/operations")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }

    #[tokio::test]
    async fn test_get_system_status() {
        // Create test app
        let scheduler = Arc::new(Scheduler::default());
        let monitor = Arc::new(Monitor::default());
        let app = create_router(scheduler, monitor);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/system/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}