use axum::{
    async_trait,
    extract::{Extension, Path, FromRequest, RequestParts},
    http::StatusCode,
    response::IntoResponse,
    BoxError, Json,
};
use http_body;
use serde::de::DeserializeOwned;
use validator::Validate;
use std::sync::Arc;
use crate::repositories::{
    CreateTodo,
    TodoRepository,
    UpdateTodo,
};

#[derive(Debug)]
pub struct ValidatedJson<T>(T);

// trait 内のメソッドでは asycn を宣言できないので、 async-trait パッケージのマクロを用いる
#[async_trait]
impl<T, B> FromRequest<B> for ValidatedJson<T>
where
    // Json::<T>::from_request(req) を実装するために必要なトレイト境界の宣言
    T: DeserializeOwned + Validate,
    B: http_body::Body + Send,
    B::Data: Send,
    B::Error: Into<BoxError>,
{
    type Rejection = (StatusCode, String);

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req).await.map_err(|rejection| {
            let message = format!("Json parse error: [{}]", rejection);
            (StatusCode::BAD_REQUEST, message)
        })?;
        value.validate().map_err(|rejection| {
            let message = format!("Validation error: [{}]", rejection).replace('\n', ", ");
            (StatusCode::BAD_REQUEST, message)
        })?;
        Ok(ValidatedJson(value))
    }
}

pub async fn create_todo<T: TodoRepository>(
    ValidatedJson(payload): ValidatedJson<CreateTodo>,
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
    ValidatedJson(payload): ValidatedJson<UpdateTodo>,
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