use axum::{
    extract::Extension,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use crate::repositories::{
    CreateTodo,
    TodoRepository,
};

pub async fn create_todo<T: TodoRepository>(
    Json(payload): Json<CreateTodo>,
    Extension(repo): Extension<Arc<T>>,
) -> impl IntoResponse {
    let todo = repo.create(payload);

    (StatusCode::CREATED, Json(todo))
}
