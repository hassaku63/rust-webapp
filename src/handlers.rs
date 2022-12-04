pub mod label;
pub mod todo;

use axum::{
    async_trait,
    extract::{FromRequest, RequestParts},
    http::StatusCode,
    BoxError, Json,
};
use serde::de::DeserializeOwned;
use validator::Validate;

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
