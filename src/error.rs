use std::fmt;

/// A query compilation or command-line input error.
#[derive(Debug)]
pub struct QueryError(pub(crate) String);

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for QueryError {}
pub(crate) use QueryError as AppError;
pub(crate) type Result<T, E = QueryError> = std::result::Result<T, E>;
