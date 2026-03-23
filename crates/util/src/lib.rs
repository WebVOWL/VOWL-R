mod datatypes;
mod error_handler;
mod layout;

pub mod prelude {
    pub use crate::datatypes::DataType;
    pub use crate::error_handler::{ErrorRecord, ErrorSeverity, ErrorType, LOVETError};
    pub use crate::layout::TableHTML;
}
