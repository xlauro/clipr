pub mod db;
pub mod error;
pub mod schema;

pub use db::DatabaseManager;
pub use error::PersistenceError;

pub mod prelude {
    pub use super::{DatabaseManager, PersistenceError};
}
