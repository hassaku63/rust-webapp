use anyhow::Context;
use axum::{
    extract::Extension,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Serialize, Deserialize};
use std::net::SocketAddr;
use std::{
    collections::HashMap,
    env,
    sync::{Arc, RwLock},
};
use thiserror::Error;

#[derive(Debug, Error)]
enum RepositoryError {
    #[error("NotFound, id is {0}")]
    NotFound(i32),
}

// Clone, Send, Sync, 'static の多重継承
// axum でこのレポジトリ機能を共有(?)するために layer という機能を使う。layer を利用するためにこれらを継承する必要がある
// ここでの「共有」は単一プロセスの中でシングルトン的に扱いたい、という意味合いと勝手に解釈した
pub trait TodoRepository: Clone + std::marker::Send + std::marker::Sync + 'static {
    fn create(&self, payload: CreateTodo) -> Todo;
    fn find(&self, id: i32) -> Option<Todo>;
    fn all(&self) -> Vec<Todo>;
    fn update(&self, id: i32, paylaod: UpdateTodo) -> anyhow::Result<Todo>;
    fn delete(&self, id: i32) -> anyhow::Result<()>;
}

// 以下の Todo に関連する構造体は derive Clone しないと axum の「共有状態」として利用できなくなる(?)
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Todo {
    id: i32,
    text: String,
    complated: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct CreateTodo {
    text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct UpdateTodo {
    text: Option<String>,
    completed: Option<bool>,
}

impl Todo {
    pub fn new(id: i32, text: String) -> Self {
        Self {
            id,
            text,
            complated: false,
        }
    }
}

type TodoDatas = HashMap<i32, Todo>;

#[derive(Debug, Clone)]
pub struct TodoRepositoryForMemory {
    // 複数スレッドからのアクセスを想定し Arc<RwLock<>> でスレッドセーフにする
    // 不変参照の場合は複数スレッドで共有できるが、可変参照の場合はスレッドを1つに制限する
    store: Arc<RwLock<TodoDatas>>,
}

impl TodoRepositoryForMemory {
    pub fn new() -> Self {
        TodoRepositoryForMemory {
            store: Arc::default(),
        }
    }
}

impl TodoRepository for TodoRepositoryForMemory {
    fn create(&self, payload: CreateTodo) -> Todo {
        todo!()
    }

    fn find(&self, id: i32) -> Option<Todo> {
        todo!()
    }

    fn update(&self, id: i32, paylaod: UpdateTodo) -> anyhow::Result<Todo> {
        todo!()
    }

    fn all(&self) -> Vec<Todo> {
        todo!()
    }

    fn delete(&self, id: i32) -> anyhow::Result<()> {
        todo!()
    }
}

#[tokio::main]
async fn main() {
    let log_level = env::var("RUST_LOG").unwrap_or("info".to_string());
    env::set_var("RUST_LOG", log_level);
    tracing_subscriber::fmt::init();

    let repo = TodoRepositoryForMemory::new();
    let app = create_app(repo);
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    tracing::debug!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

fn create_app<T: TodoRepository>(repoitory: T) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/users", post(create_user))
        .route("/todos", post(create_todo::<T>))
        .layer(Extension(Arc::new(repoitory)))
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


pub async fn create_todo<T: TodoRepository>(
    Json(payload): Json<CreateTodo>,
    Extension(repo): Extension<Arc<T>>,
) -> impl IntoResponse {
    let todo = repo.create(payload);

    (StatusCode::CREATED, Json(todo))
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
        let repo = TodoRepositoryForMemory::new();
        let req = Request::builder()
            .uri("/")
            .body(Body::empty())
            .unwrap();
        let router = create_app(repo);
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
        let repo = TodoRepositoryForMemory::new();
        let builder = Request::builder();
        let req = builder
            .uri("/users")
            .method(Method::POST)
            .header(header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
            .body(Body::from(r#"{"username": "Alice"}"#))
            .unwrap();
        let res = create_app(repo).oneshot(req).await.unwrap();

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