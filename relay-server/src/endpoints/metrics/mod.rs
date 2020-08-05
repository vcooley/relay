//! A simple metrics endpoint for relay.

use actix_web::{HttpRequest, HttpResponse};
use serde_json::json;

use crate::extractors::CurrentServiceState;
use crate::service::{ServiceState, ServiceApp};

static INDEX: &str = include_str!("index.html");
static JS: &str = include_str!("graph.js");

fn metrics_data(_state: CurrentServiceState) -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "metrics": {
            "requests": 1234,
            "event.accepted": 123,
            "event.rejected": 124,
            "project_cache.hit": 90,
            "project_cache.miss": 10,
        }
    }))
}

// TODO: serving hardcoded snippets might not be the best idea.
// rather serve files from disk?

fn js(_: &HttpRequest<ServiceState>) -> HttpResponse {
    HttpResponse::Ok().content_type("application/javascript").body(JS)
}

fn index(_: &HttpRequest<ServiceState>) -> HttpResponse {
    HttpResponse::Ok().content_type("text/html").body(INDEX)
}

pub fn configure_app(app: ServiceApp) -> ServiceApp {
    app.resource("/api/relay/metrics/data.json", |r| {
        r.name("internal-metrics-data");
        r.get().with(metrics_data);
    })
    .handler("/api/relay/metrics/graph.js", js)
    .handler("/api/relay/metrics/", index)
}
