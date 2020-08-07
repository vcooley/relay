//! A simple metrics endpoint for relay.

use actix_web::{HttpRequest, HttpResponse};
use metrics_exporter_statsd::HtmlExporter;
use serde_json::json;

use crate::extractors::CurrentServiceState;
use crate::service::{ServiceApp, ServiceState};

fn metrics_data(state: CurrentServiceState) -> HttpResponse {
    if let Some(ref mc) = *state.metrics_collector().lock() {
        let html = mc.html();
        HttpResponse::Ok().json(html.json_snapshot())
    } else {
        HttpResponse::Ok().json(json!({}))
    }
}

// TODO: serving hardcoded snippets might not be the best idea.
// rather serve files from disk?

fn js(_: &HttpRequest<ServiceState>) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/javascript")
        .body(HtmlExporter::JS)
}

fn index(_: &HttpRequest<ServiceState>) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(HtmlExporter::INDEX)
}

pub fn configure_app(app: ServiceApp) -> ServiceApp {
    app.resource("/api/relay/metrics/data.json", |r| {
        r.name("internal-metrics-data");
        r.get().with(metrics_data);
    })
    .handler("/api/relay/metrics/graph.js", js)
    .handler("/api/relay/metrics/", index)
}
