use std::sync::Arc;

use engine::{registry::resolvers::graphql, RequestHeaders};
use futures_util::future::BoxFuture;
use http::header;
use wasm_bindgen::prelude::*;
use web_sys::Response;
use worker::{Cors, Method};

pub const ASSETS: &[u8] = include_bytes!("../assets/registry.json");

#[wasm_bindgen]
pub async fn handler(request: web_sys::Request) -> std::result::Result<Response, JsValue> {
    console_error_panic_hook::set_once();
    log::configure(log::Config::WORKER);
    if Method::from(request.method()) == Method::Options {
        return Response::new_with_opt_str(Some(""));
    }
    let response = process_request(worker::Request::from(request))
        .await
        .unwrap_or_else(|err| {
            worker::Response::from_body(worker::ResponseBody::Body(err.to_string().into_bytes()))
                .unwrap()
        });
    Ok(response
        .with_cors(
            &Cors::new()
                .with_allowed_headers(
                    [
                        header::ACCEPT,
                        header::AUTHORIZATION,
                        header::CACHE_CONTROL,
                        header::CONTENT_TYPE,
                        header::ORIGIN,
                        http::HeaderName::from_static("x-api-key"),
                    ]
                    .iter()
                    .map(http::HeaderName::as_str),
                )
                .with_max_age(86400)
                .with_methods([Method::Options, Method::Post])
                .with_origins(["*"]),
        )
        .unwrap()
        .into())
}

pub async fn process_request(mut request: worker::Request) -> worker::Result<worker::Response> {
    if request.method() == Method::Options {
        return worker::Response::from_body(worker::ResponseBody::Empty);
    }
    let ray_id = ulid::Ulid::new().to_string();
    let schema = build_schema(&ray_id, request.headers().into()).await;
    let request: engine::Request = request.json().await?;
    log::info!(
        &ray_id,
        "Executing request{}",
        request
            .operation_name
            .as_ref()
            .map(|op| format!(" {op}"))
            .unwrap_or_default()
    );
    let response = schema.execute(request).await;
    worker::Response::from_json(&response.to_graphql_response())
}

async fn build_schema(ray_id: &str, headers: http::HeaderMap) -> engine::Schema {
    let runtime_ctx = runtime::Context::new(
        &Arc::new(RequestContext {
            ray_id: ray_id.to_string(),
            headers: headers.clone(),
        }),
        runtime::context::LogContext {
            fetch_log_endpoint_url: None,
            request_log_event_id: None,
        },
    );

    engine::Schema::build(serde_json::from_slice(ASSETS).unwrap())
        .data(graphql::QueryBatcher::new())
        .data(RequestHeaders::new(
            headers
                .into_iter()
                .map(|(name, value)| {
                    (
                        name.map(|name| name.to_string()).unwrap_or_default(),
                        String::from_utf8_lossy(value.as_bytes()).to_string(),
                    )
                })
                .collect::<Vec<_>>(),
        ))
        .data(runtime_ctx)
        .finish()
}

struct RequestContext {
    ray_id: String,
    headers: http::HeaderMap,
}

#[async_trait::async_trait]
impl runtime::context::RequestContext for RequestContext {
    fn ray_id(&self) -> &str {
        &self.ray_id
    }

    async fn wait_until(&self, _fut: BoxFuture<'static, ()>) {
        unimplemented!("wait_until not implemented...");
    }

    fn headers(&self) -> &http::HeaderMap {
        &self.headers
    }
}
