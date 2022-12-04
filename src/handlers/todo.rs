use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use crate::repositories::todo::{
    CreateTodo,
    TodoRepository,
    UpdateTodo,
};
use super::ValidatedJson;

pub async fn create_todo<T: TodoRepository>(
    ValidatedJson(payload): ValidatedJson<CreateTodo>,
    Extension(repo): Extension<Arc<T>>,
) -> Result<impl IntoResponse, StatusCode> {
    let todo = repo
        .create(payload)
        .await
        .or(Err(StatusCode::NOT_FOUND))?;

    Ok((StatusCode::CREATED, Json(todo)))
}

pub async fn find_todo<T: TodoRepository>(
    Path(id): Path<i32>,
    Extension(repo): Extension<Arc<T>>,
) -> Result<impl IntoResponse, StatusCode> {
    let todo = repo.find(id).await.or(Err(StatusCode::NOT_FOUND))?;
    Ok((StatusCode::OK, Json(todo)))
}

pub async fn all_todo<T: TodoRepository>(
    Extension(repo): Extension<Arc<T>>,
) -> Result<impl IntoResponse, StatusCode> {
    let todos = repo.all().await.unwrap();
    Ok((StatusCode::OK, Json(todos)))
}

pub async fn update_todo<T: TodoRepository>(
    Path(id): Path<i32>,
    ValidatedJson(payload): ValidatedJson<UpdateTodo>,
    Extension(repo): Extension<Arc<T>>,
) -> Result<impl IntoResponse, StatusCode> {
    let todo = repo
        .update(id, payload)
        .await
        .or(Err(StatusCode::NOT_FOUND))?;
    Ok((StatusCode::CREATED, Json(todo)))
}

pub async fn delete_todo<T: TodoRepository>(
    Path(id): Path<i32>,
    Extension(repo): Extension<Arc<T>>,
) -> impl IntoResponse {
    repo.delete(id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .unwrap_or(StatusCode::NOT_FOUND)
}