use potion::Error;

pub struct QueryError {
    info: String,
}

impl QueryError {
    pub fn new(info: String) -> Self {
        Self { info }
    }
}

impl From<sqlx::Error> for QueryError {
    fn from(value: sqlx::Error) -> Self {
        match value {
            sqlx::Error::Configuration(e) => Self::new(format!("{e}")),
            sqlx::Error::Database(e) => Self::new(format!("{e}")),
            sqlx::Error::Io(e) => Self::new(format!("{e}")),
            sqlx::Error::Tls(e) => Self::new(format!("{e}")),
            sqlx::Error::Protocol(e) => Self::new(format!("{e}")),
            sqlx::Error::RowNotFound => Self::new(format!("RowNotFound")),
            sqlx::Error::TypeNotFound { type_name } => {
                Self::new(format!("Type not found: {type_name}"))
            }
            sqlx::Error::ColumnIndexOutOfBounds { index, len } => {
                Self::new(format!("Column index out of bounds {index} ({len})"))
            }
            sqlx::Error::ColumnNotFound(e) => Self::new(format!("{e}")),
            sqlx::Error::ColumnDecode { index, source } => {
                Self::new(format!("Column decode {index} ({source})"))
            }
            sqlx::Error::Decode(e) => Self::new(format!("{e}")),
            sqlx::Error::AnyDriverError(e) => Self::new(format!("{e}")),
            sqlx::Error::PoolTimedOut => Self::new(format!("Pool timed out")),
            sqlx::Error::PoolClosed => Self::new(format!("Pool closed")),
            sqlx::Error::WorkerCrashed => Self::new(format!("Worker crashed")),
            sqlx::Error::Migrate(e) => Self::new(format!("{e}")),
            _ => Self::new(format!("Unknown error")),
        }
    }
}

impl Into<Error> for QueryError {
    fn into(self) -> Error {
        Error {
            code: 500,
            info: Some(self.info),
            redirect: None,
        }
    }
}

pub struct CacheError {
    info: String,
}

impl From<redis::RedisError> for CacheError {
    fn from(value: redis::RedisError) -> Self {
        Self {
            info: format!("{:?} - {:?}", value.code(), value.detail()),
        }
    }
}

impl CacheError {
    pub fn new(info: String) -> Self {
        Self { info }
    }
}

impl Into<Error> for CacheError {
    fn into(self) -> Error {
        Error {
            code: 500,
            info: Some(self.info),
            redirect: None,
        }
    }
}
