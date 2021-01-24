use actix_web::ResponseError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Unable to open file")]
    FileMissing(#[from] std::io::Error),
    #[error("Unable to request another server")]
    ReqwestError(#[from] reqwest::Error),
    #[error("A actix-web Error happened")]
    ActixWebError(#[from] actix_web::Error),
    #[error("Possible Directory/Path Traversal Attack detected")]
    PathTraversal,
    #[error("You Matrix Server is not configured correctly")]
    MatrixFederationWronglyConfigured,
}

impl ResponseError for ServerError {}
