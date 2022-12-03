use axum::{
    extract::{Extension, Path},
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
    let todo = repo.find(id).ok_or(StatusCode::NOT_FOUND)?;
    Ok((StatusCode::OK, Json(todo)))
}

pub async fn all_todo<T: TodoRepository>(
    Extension(repo): Extension<Arc<T>>,
) -> impl IntoResponse {
    let todos = repo.all();
    (StatusCode::OK, Json(todos))
}

pub async fn update_todo<T: TodoRepository>(
    Path(id): Path<i32>,
    Json(payload): Json<UpdateTodo>,
    Extension(repo): Extension<Arc<T>>,
) -> Result<impl IntoResponse, StatusCode> {
    let todo = repo
        .update(id, payload)
        .or(Err(StatusCode::NOT_FOUND))?;
    Ok((StatusCode::CREATED, Json(todo)))
}

pub async fn delete_todo<T: TodoRepository>(
    Path(id): Path<i32>,
    Extension(repo): Extension<Arc<T>>,
) -> impl IntoResponse {
    repo.delete(id)
        .map(|_| StatusCode::NO_CONTENT)
        .unwrap_or(StatusCode::NOT_FOUND)
}