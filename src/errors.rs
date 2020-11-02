use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Possible Directory/Path Traversal Attack detected")]
    PathTraversal,
}

