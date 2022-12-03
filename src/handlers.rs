use axum::{
    extract::{Extension, Path, path},
    http::StatusCode,
    response::IntoResponse,
    Json
};
use std::sync::Arc;
use crate::repositories::{
    CreateTodo,
    TodoRepository,
    UpdateTodo,
};

pub async fn create_todo<T: TodoRepository>(
    Json(payload): Json<CreateTodo>,
    Extension(repo): Extension<Arc<T>>,
) -> impl IntoResponse {
    let todo = repo.create(payload);

    (StatusCode::CREATED, Json(todo))
}

pub async fn find_todo<T: TodoRepository>(
    Path(id): Path<i32>,
    Extension(repo): Extension<Arc<T>>,
) -> Result<impl IntoResponse, StatusCode> {
    todo!();
    Ok(StatusCode::OK)
}

pub async fn all_todo<T: TodoRepository>(
    Extension(repo): Extension<Arc<T>>,
) -> impl IntoResponse {
    todo!()
}

pub async fn update_todo<T: TodoRepository>(
    Extension(repo): Extension<Arc<T>>,
) -> impl IntoResponse {
    todo!()
}

pub async fn delete_todo<T: TodoRepository>(
    Path(id): Path<i32>,
    Extension(repo): Extension<Arc<T>>,
) -> impl IntoResponse {
    todo!()
}