use crate::serializers::Triple;
use oxrdf::{BlankNodeIdParseError, IriParseError};
use vowlr_parser::errors::VOWLRStoreError;
use vowlr_util::prelude::{ErrorRecord, ErrorSeverity, ErrorType};

// pub trait SerializationErrorExt {
//     fn triple(&self) -> Option<&Triple>;
// }

// macro_rules! ser_err {
//     ($variant:ident($triple:expr, $msg:expr)) => {
//         $crate::SerializationErrorKind::$variant(($triple).map(Box::new), $msg)
//     };
// }
// pub(crate) use ser_err;

// #[derive(Debug)]
// pub enum SerializationErrorKind {
//     MissingObject(Option<Box<Triple>>, String),
//     MissingSubject(Option<Box<Triple>>, String),
//     SerializationFailed(Option<Box<Triple>>, String),
//     IriParseError(Option<Box<Triple>>, Box<IriParseError>),
//     BlankNodeParseError(Option<Box<Triple>>, Box<BlankNodeIdParseError>),
// }
// impl SerializationErrorExt for SerializationErrorKind {
//     fn triple(&self) -> Option<&Triple> {
//         match &self {
//             SerializationErrorKind::MissingObject(triple, _)
//             | SerializationErrorKind::MissingSubject(triple, _)
//             | SerializationErrorKind::SerializationFailed(triple, _)
//             | SerializationErrorKind::IriParseError(triple, _)
//             | SerializationErrorKind::BlankNodeParseError(triple, _) => {
//                 triple.as_ref().map(|t| &**t)
//             }
//         }
//     }
// }

#[derive(Debug)]
pub enum SerializationErrorKind {
    /// An error raised when the object of a triple is required but missing.
    MissingObject(Triple, String),
    /// An error raised when the subject of a triple is required but missing.
    MissingSubject(Triple, String),
    /// An error raised when the serializer encountered an unrecoverable problem.
    SerializationFailed(Triple, String),
    /// An error raised during Iri or IriRef validation.
    IriParseError(Triple, IriParseError),
    /// An error raised during BlankNode IDs validation
    BlankNodeParseError(Triple, BlankNodeIdParseError),
}

#[derive(Debug)]
pub struct SerializationError {
    inner: SerializationErrorKind,
}
impl std::fmt::Display for SerializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.inner)
    }
}

// impl SerializationErrorExt for SerializationError {
//     fn triple(&self) -> Option<&Triple> {
//         self.inner.triple()
//     }
// }

impl From<SerializationErrorKind> for SerializationError {
    fn from(error: SerializationErrorKind) -> Self {
        SerializationError { inner: error }
    }
}

// impl From<IriParseError> for SerializationError {
//     fn from(error: IriParseError) -> Self {
//         SerializationError {
//             inner: SerializationErrorKind::IriParseError(None, Box::new(error)),
//         }
//     }
// }

// impl From<SerializationError> for VOWLRStoreError {
//     fn from(error: SerializationError) -> Self {
//         VOWLRStoreError::from(error.to_string())
//     }
// }

// impl From<BlankNodeIdParseError> for SerializationError {
//     fn from(error: BlankNodeIdParseError) -> Self {
//         SerializationError {
//             inner: SerializationErrorKind::BlankNodeParseError(None, Box::new(error)),
//         }
//     }
// }

impl From<SerializationError> for ErrorRecord {
    fn from(value: SerializationError) -> Self {
        let (message, severity) = match value.inner {
            SerializationErrorKind::MissingObject(triple, e) => {
                (format!("{e}\n{triple}"), ErrorSeverity::Warning)
            }
            SerializationErrorKind::MissingSubject(triple, e) => {
                (format!("{e}\n{triple}"), ErrorSeverity::Warning)
            }
            SerializationErrorKind::SerializationFailed(triple, e) => {
                (format!("{e}\n{triple}"), ErrorSeverity::Critical)
            }
            SerializationErrorKind::IriParseError(triple, iri_parse_error) => (
                format!("{iri_parse_error}\n{triple}"),
                ErrorSeverity::Severe,
            ),
            SerializationErrorKind::BlankNodeParseError(triple, blank_node_id_parse_error) => (
                format!("{blank_node_id_parse_error}\n{triple}"),
                ErrorSeverity::Severe,
            ),
        };
        ErrorRecord::new(
            severity,
            ErrorType::Serializer,
            message,
            #[cfg(debug_assertions)]
            "N/A".to_string(),
        )
    }
}
