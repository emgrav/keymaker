use actix_web::error;
use thiserror::Error;
#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Unable to open file")]
    FileMissing(#[from] std::io::Error),
    #[error("Possible Directory/Path Traversal Attack detected")]
    PathTraversal,
}

impl error::ResponseError for ServerError {}
