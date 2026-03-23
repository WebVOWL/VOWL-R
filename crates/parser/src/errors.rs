use std::{io::Error, panic::Location};

use horned_owl::error::HornedError;

use lovet_util::prelude::{ErrorRecord, ErrorSeverity, ErrorType, LOVETError};
use rdf_fusion::{
    error::LoaderError,
    execution::sparql::error::QueryEvaluationError,
    model::{IriParseError, StorageError},
};
use tokio::task::JoinError;

#[derive(Debug)]
pub enum LOVETStoreErrorKind {
    /// The file type is not supported by the server.
    ///
    /// Example: server only supports `.owl` and is given `.png`
    InvalidFileType(String),
    /// An error raised by Horned-OWL during parsing (of OWL files).
    HornedError(HornedError),
    /// Generic IO error.
    IOError(std::io::Error),
    /// An error raised while trying to parse an invalid IRI.
    IriParseError(IriParseError),
    /// An error raised while loading a file into a Store (database).
    LoaderError(LoaderError),
    /// A SPARQL evaluation error.
    QueryEvaluationError(QueryEvaluationError),
    /// A Tokio task failed to execute to completion.
    JoinError(JoinError),
    /// An error related to (database) storage operations (reads, writes...).
    StorageError(StorageError),
}

#[derive(Debug)]
pub struct LOVETStoreError {
    /// The contained error type.
    inner: LOVETStoreErrorKind,
    /// The error's location in the source code.
    location: &'static Location<'static>,
}

impl From<LOVETStoreError> for Error {
    fn from(val: LOVETStoreError) -> Self {
        Error::other(val.to_string())
    }
}
impl From<String> for LOVETStoreError {
    #[track_caller]
    fn from(error: String) -> Self {
        LOVETStoreError {
            inner: LOVETStoreErrorKind::InvalidFileType(error),
            location: Location::caller(),
        }
    }
}

impl From<HornedError> for LOVETStoreError {
    #[track_caller]
    fn from(error: HornedError) -> Self {
        LOVETStoreError {
            inner: LOVETStoreErrorKind::HornedError(error),
            location: Location::caller(),
        }
    }
}

impl From<IriParseError> for LOVETStoreError {
    #[track_caller]
    fn from(error: IriParseError) -> Self {
        LOVETStoreError {
            inner: LOVETStoreErrorKind::IriParseError(error),
            location: Location::caller(),
        }
    }
}

impl From<LoaderError> for LOVETStoreError {
    #[track_caller]
    fn from(error: LoaderError) -> Self {
        LOVETStoreError {
            inner: LOVETStoreErrorKind::LoaderError(error),
            location: Location::caller(),
        }
    }
}
impl From<LOVETStoreErrorKind> for LOVETStoreError {
    #[track_caller]
    fn from(error: LOVETStoreErrorKind) -> Self {
        LOVETStoreError {
            inner: error,
            location: Location::caller(),
        }
    }
}

impl From<std::io::Error> for LOVETStoreError {
    #[track_caller]
    fn from(error: std::io::Error) -> Self {
        LOVETStoreError {
            inner: LOVETStoreErrorKind::IOError(error),
            location: Location::caller(),
        }
    }
}
impl From<QueryEvaluationError> for LOVETStoreError {
    #[track_caller]
    fn from(error: QueryEvaluationError) -> Self {
        LOVETStoreError {
            inner: LOVETStoreErrorKind::QueryEvaluationError(error),
            location: Location::caller(),
        }
    }
}
impl From<JoinError> for LOVETStoreError {
    #[track_caller]
    fn from(error: JoinError) -> Self {
        LOVETStoreError {
            inner: LOVETStoreErrorKind::JoinError(error),
            location: Location::caller(),
        }
    }
}

impl From<StorageError> for LOVETStoreError {
    #[track_caller]
    fn from(error: StorageError) -> Self {
        LOVETStoreError {
            inner: LOVETStoreErrorKind::StorageError(error),
            location: Location::caller(),
        }
    }
}

impl std::fmt::Display for LOVETStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} at {}", self.inner, self.location)
    }
}

impl std::error::Error for LOVETStoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.inner {
            LOVETStoreErrorKind::InvalidFileType(_) => None,
            LOVETStoreErrorKind::HornedError(e) => Some(e),
            LOVETStoreErrorKind::IOError(e) => Some(e),
            LOVETStoreErrorKind::IriParseError(e) => Some(e),
            LOVETStoreErrorKind::LoaderError(e) => Some(e),
            LOVETStoreErrorKind::QueryEvaluationError(e) => Some(e),
            LOVETStoreErrorKind::JoinError(e) => Some(e),
            LOVETStoreErrorKind::StorageError(e) => Some(e),
        }
    }
}

impl From<LOVETStoreError> for ErrorRecord {
    fn from(value: LOVETStoreError) -> Self {
        let (message, error_type) = match value.inner {
            LOVETStoreErrorKind::InvalidFileType(e) => (e, ErrorType::Parser),
            LOVETStoreErrorKind::HornedError(horned_error) => {
                (horned_error.to_string(), ErrorType::Parser)
            }
            LOVETStoreErrorKind::IOError(error) => {
                (error.to_string(), ErrorType::InternalServerError)
            }
            LOVETStoreErrorKind::IriParseError(iri_parse_error) => {
                (iri_parse_error.to_string(), ErrorType::Parser)
            }
            LOVETStoreErrorKind::LoaderError(loader_error) => {
                (loader_error.to_string(), ErrorType::Database)
            }
            LOVETStoreErrorKind::QueryEvaluationError(query_evaluation_error) => {
                (query_evaluation_error.to_string(), ErrorType::Database)
            }
            LOVETStoreErrorKind::JoinError(join_error) => {
                (join_error.to_string(), ErrorType::InternalServerError)
            }
            LOVETStoreErrorKind::StorageError(storage_error) => {
                (storage_error.to_string(), ErrorType::Database)
            }
        };
        ErrorRecord::new(
            ErrorSeverity::Critical,
            error_type,
            message,
            #[cfg(debug_assertions)]
            Some(value.location.to_string()),
        )
    }
}

impl From<LOVETStoreError> for LOVETError {
    fn from(value: LOVETStoreError) -> Self {
        let record: ErrorRecord = value.into();
        record.into()
    }
}
