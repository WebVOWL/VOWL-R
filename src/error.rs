use std::panic::Location;

use lovet_util::prelude::{ErrorRecord, ErrorSeverity, ErrorType, LOVETError};

#[derive(Debug)]
pub enum ClientErrorKind {
    /// An error raised when an unexpected value was received from JS-land.
    JavaScriptError(String),
    /// Errors related to the graph renderer (i.e. ``WasmGrapher``)
    RenderError(String),
    /// Errors related to file upload
    FileUploadError(String),
}

impl From<ClientErrorKind> for ErrorRecord {
    #[track_caller]
    fn from(value: ClientErrorKind) -> Self {
        let (message, error_type, severity) = match value {
            ClientErrorKind::JavaScriptError(e) => (e, ErrorType::Gui, ErrorSeverity::Error),
            ClientErrorKind::FileUploadError(e) => {
                (e, ErrorType::ClientError, ErrorSeverity::Error)
            }
            ClientErrorKind::RenderError(e) => (e, ErrorType::Renderer, ErrorSeverity::Critical),
        };
        Self::new(
            severity,
            error_type,
            message,
            #[cfg(debug_assertions)]
            Some(Location::caller().to_string()),
        )
    }
}

impl From<ClientErrorKind> for LOVETError {
    fn from(value: ClientErrorKind) -> Self {
        let a: ErrorRecord = value.into();
        a.into()
    }
}

// #[derive(Debug)]
// pub struct LOVETClientError {
//     /// The contained error type.
//     inner: ClientErrorKind,
//     /// The error's location in the source code.
//     location: &'static Location<'static>,
// }
// impl std::fmt::Display for LOVETClientError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{:?}", self.inner)
//     }
// }

// impl From<ClientErrorKind> for LOVETClientError {
//     #[track_caller]
//     fn from(error: ClientErrorKind) -> Self {
//         LOVETClientError {
//             inner: error,
//             location: Location::caller(),
//         }
//     }
// }

// impl From<LOVETClientError> for ErrorRecord {
//     fn from(value: LOVETClientError) -> Self {
//         let (message, error_type, severity) = match value.inner {
//             ClientErrorKind::JavaScriptError(e) => (e, ErrorType::Gui, ErrorSeverity::Error),
//             ClientErrorKind::RenderError(e) => (e, ErrorType::Renderer, ErrorSeverity::Critical),
//         };
//         ErrorRecord::new(
//             severity,
//             error_type,
//             message,
//             #[cfg(debug_assertions)]
//             Some(value.location.to_string()),
//         )
//     }
// }

// impl From<LOVETClientError> for LOVETError {
//     fn from(value: LOVETClientError) -> Self {
//         let a: ErrorRecord = value.into();
//         a.into()
//     }
// }
