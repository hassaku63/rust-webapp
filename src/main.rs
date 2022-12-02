use std::env;
use std::net::SocketAddr;

use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Serialize, Deserialize};

#[tokio::main]
async fn main() {
    let log_level = env::var("RUST_LOG").unwrap_or("info".to_string());
    env::set_var("RUST_LOG", log_level);
    tracing_subscriber::fmt::init();

    let app = create_app();
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    tracing::debug!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

fn create_app() -> Router {
    Router::new()
        .route("/", get(root))
        .route("/users", post(create_user))
}

async fn root() -> &'static str {
    "hello world"
}

async fn create_user(
    Json(payload): Json<CreateUser>,
) -> impl IntoResponse {
    let user = User {
        id: 1337,
        username: payload.username,
    };

    (StatusCode::CREATED, Json(user))
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct CreateUser {
    username: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct User {
    id: u64,
    username: String,
}

#[cfg(test)]
mod test {
    use super::*;
    use axum::{
        body::Body,
        http::{header, Method, Request},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn should_return_hello_world() {
        let req = Request::builder()
            .uri("/")
            .body(Body::empty())
            .unwrap();
        let router = create_app();
        let res = router.oneshot(req).await.unwrap();
        let bytes = hyper::body::to_bytes(
            res.into_body())
            .await
            .unwrap();
        let body = String::from_utf8(bytes.to_vec()).unwrap();
        assert_eq!(body, "hello world");
    }

    #[tokio::test]
    async fn should_return_user_data() {
        let builder = Request::builder();
        let req = builder
            .uri("/users")
            .method(Method::POST)
            // .header(header::CONTENT_TYPE, mime::APPLICATION_JSON)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{"username": "Alice"}"#))
            .unwrap();
        let res = create_app().oneshot(req).await.unwrap();

        let byte_body = res.into_body();
        let bytes = hyper::body::to_bytes(byte_body).await.unwrap();
        let body: String = String::from_utf8(bytes.to_vec()).unwrap();
        let user: User = serde_json::from_str(&body).expect("cannot create User instance.");
        assert_eq!(user, User {
            id: 1337,
            username: "Alice".to_string(),
        });
    }

}