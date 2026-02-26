use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

pub async fn guard_cron_request(
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // GAE sets "X-Appengine-Cron: true" for internal cron calls
    if let Some(header_value) = req.headers().get("X-Appengine-Cron") {
        if header_value == "true" {
            return Ok(next.run(req).await);
        }
    }

    // Reject anyone else with a 403 Forbidden
    Err(StatusCode::FORBIDDEN)
}