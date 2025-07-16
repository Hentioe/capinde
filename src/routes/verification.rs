use crate::{
    errors::{Error, Result},
    models::{params::verification::Input, payload::VefifyResult},
    verification,
};
use axum::Json;

pub async fn verify(input: Json<Input>) -> Result<Json<VefifyResult>> {
    match verification::verify(&input.unique_id, &input.answer).await {
        Some(ok) => Ok(Json(VefifyResult { ok })),
        None => Err(Error::VerificationCacheNotFound(input.unique_id.clone())),
    }
}
