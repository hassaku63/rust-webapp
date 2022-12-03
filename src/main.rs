mod handlers;
mod repositories;

use crate::repositories::{
    TodoRepository,
    TodoRepositoryForMemory,
};
use axum::{
    extract::Extension,
    routing::{get, post},
    Router,
};
use handlers::{
    all_todo,
    create_todo,
    delete_todo,
    find_todo,
    update_todo,
};
use std::net::SocketAddr;
use std::{env, sync::Arc};

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
        .route("/todos", post(create_todo::<T>).get(all_todo::<T>))
        .route(
            "/todos/:id",
            get(find_todo::<T>)
                .delete(delete_todo::<T>)
                .patch(update_todo::<T>)
        ).layer(Extension(Arc::new(repoitory)))
}

async fn root() -> &'static str {
    "hello world"
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::repositories::{CreateTodo, Todo};
    use axum::response::Response;
    use axum::{
        body::{Body},
        http::{header, Method, Request, StatusCode},
    };
    use tower::ServiceExt;

    fn build_todo_req_with_json(path: &str, method: Method, json_body: String) -> Request<Body> {
        Request::builder()
            .uri(path)
            .method(method)
            .header(header::CONTENT_TYPE, mime::APPLICATION_JSON.to_string())
            .body(Body::from(json_body))
            .unwrap()
    }

    fn build_todo_req_with_empty(method: Method, path: &str) -> Request<Body> {
        Request::builder()
            .uri(path)
            .method(method)
            .body(Body::empty())
            .unwrap()
    }

    async fn res_to_todo(res: Response) -> Todo {
        let bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
        let body: String = String::from_utf8(bytes.to_vec()).unwrap();
        let todo: Todo = serde_json::from_str(&body)
            .expect(&format!("cannot convert Todo instance. body: {}", body));
        todo
    }

    #[tokio::test]
    async fn should_return_hello_world() {
        let repo = TodoRepositoryForMemory::new();
        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        let router = create_app(repo);
        let res = router.oneshot(req).await.unwrap();
        let bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
        let body = String::from_utf8(bytes.to_vec()).unwrap();
        assert_eq!(body, "hello world");
    }

    #[tokio::test]
    async fn should_created_todo() {
        let expected = Todo::new(1, "should_return_created_todo".to_string());

        let repo = TodoRepositoryForMemory::new();
        let req = build_todo_req_with_json(
            "/todos",
            Method::POST,
            r#"{ "text": "should_return_created_todo" }"#.to_string(),
        );
        let res = create_app(repo).oneshot(req).await.unwrap();

        let todo = res_to_todo(res).await;
        assert_eq!(expected, todo);
    }

    #[tokio::test]
    async fn should_find_todo() {
        let expected = Todo::new(1, "should_find_todo".to_string());

        let repo = TodoRepositoryForMemory::new();
        repo.create(CreateTodo::new("should_find_todo".to_string()));
        let req = build_todo_req_with_empty(Method::GET, "/todos/1");
        let res = create_app(repo).oneshot(req).await.unwrap();
        let todo = res_to_todo(res).await;
        assert_eq!(expected, todo);
    }

    #[tokio::test]
    async fn should_get_all_todos() {
        let expected = Todo::new(1, "should_get_all_todos".to_string());

        let repo = TodoRepositoryForMemory::new();
        repo.create(CreateTodo::new("should_get_all_todos".to_string()));
        let req = build_todo_req_with_empty(Method::GET, "/todos");
        let res = create_app(repo).oneshot(req).await.unwrap();
        let bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
        let body = String::from_utf8(bytes.to_vec()).unwrap();
        let todo: Vec<Todo> = serde_json::from_str(&body)
            .expect(&format!("cannot convert Todo instance. body: {:?}", body));
        assert_eq!(vec![expected], todo);
    }

    #[tokio::test]
    async fn should_update_todo() {
        let expected = Todo::new(1, "should_update_todo".to_string());

        let repo = TodoRepositoryForMemory::new();
        repo.create(CreateTodo::new("before_update_todo".to_string()));
        let req = build_todo_req_with_json(
            "/todos/1",
            Method::PATCH,
            r#"{
                "text": "should_update_todo",
                "completed": false
            }"#.to_string(),
        );
        let res = create_app(repo).oneshot(req).await.unwrap();
        let todo = res_to_todo(res).await;
        assert_eq!(expected, todo); 
    }

    #[tokio::test]
    async fn should_delete_todo() {
        let repo = TodoRepositoryForMemory::new();
        repo.create(CreateTodo::new("should_find_todo".to_string()));
        let req = build_todo_req_with_empty(Method::DELETE, "/todos/1");
        let res = create_app(repo).oneshot(req).await.unwrap();
        assert_eq!(StatusCode::NO_CONTENT, res.status());
    }
}
