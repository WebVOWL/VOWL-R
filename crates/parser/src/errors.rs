use std::{
    io::{Error, ErrorKind},
    panic::Location,
};

use horned_owl::error::HornedError;
use rdf_fusion::{
    error::LoaderError,
    execution::sparql::error::QueryEvaluationError,
    model::{IriParseError, StorageError},
};
use tokio::task::JoinError;

#[derive(Debug)]
pub enum VOWLRStoreErrorKind {
    InvalidInput(String),
    HornedError(HornedError),
    IOError(std::io::Error),
    IriParseError(IriParseError),
    LoaderError(LoaderError),
    QueryEvaluationError(QueryEvaluationError),
    JoinError(JoinError),
    StorageError(StorageError),
}

#[derive(Debug)]
pub struct VOWLRStoreError {
    inner: VOWLRStoreErrorKind,
    location: &'static Location<'static>,
}

impl Into<Error> for VOWLRStoreError {
    fn into(self) -> Error {
        Error::new(ErrorKind::Other, self.to_string())
    }
}
impl From<String> for VOWLRStoreError {
    #[track_caller]
    fn from(error: String) -> Self {
        VOWLRStoreError {
            inner: VOWLRStoreErrorKind::InvalidInput(error),
            location: &Location::caller(),
        }
    }
}

impl From<HornedError> for VOWLRStoreError {
    #[track_caller]
    fn from(error: HornedError) -> Self {
        VOWLRStoreError {
            inner: VOWLRStoreErrorKind::HornedError(error),
            location: &Location::caller(),
        }
    }
}

impl From<IriParseError> for VOWLRStoreError {
    #[track_caller]
    fn from(error: IriParseError) -> Self {
        VOWLRStoreError {
            inner: VOWLRStoreErrorKind::IriParseError(error),
            location: &Location::caller(),
        }
    }
}

impl From<LoaderError> for VOWLRStoreError {
    #[track_caller]
    fn from(error: LoaderError) -> Self {
        VOWLRStoreError {
            inner: VOWLRStoreErrorKind::LoaderError(error),
            location: &Location::caller(),
        }
    }
}
impl From<VOWLRStoreErrorKind> for VOWLRStoreError {
    #[track_caller]
    fn from(error: VOWLRStoreErrorKind) -> Self {
        VOWLRStoreError {
            inner: error,
            location: &Location::caller(),
        }
    }
}

impl From<std::io::Error> for VOWLRStoreError {
    #[track_caller]
    fn from(error: std::io::Error) -> Self {
        VOWLRStoreError {
            inner: VOWLRStoreErrorKind::IOError(error),
            location: &Location::caller(),
        }
    }
}
impl From<QueryEvaluationError> for VOWLRStoreError {
    #[track_caller]
    fn from(error: QueryEvaluationError) -> Self {
        VOWLRStoreError {
            inner: VOWLRStoreErrorKind::QueryEvaluationError(error),
            location: &Location::caller(),
        }
    }
}
impl From<JoinError> for VOWLRStoreError {
    #[track_caller]
    fn from(error: JoinError) -> Self {
        VOWLRStoreError {
            inner: VOWLRStoreErrorKind::JoinError(error),
            location: &Location::caller(),
        }
    }
}

impl From<StorageError> for VOWLRStoreError {
    #[track_caller]
    fn from(error: StorageError) -> Self {
        VOWLRStoreError {
            inner: VOWLRStoreErrorKind::StorageError(error),
            location: &Location::caller(),
        }
    }
}

impl std::fmt::Display for VOWLRStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} at {}", self.inner, self.location)
    }
}

impl std::error::Error for VOWLRStoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.inner {
            VOWLRStoreErrorKind::InvalidInput(_) => None,
            VOWLRStoreErrorKind::HornedError(e) => Some(e),
            VOWLRStoreErrorKind::IOError(e) => Some(e),
            VOWLRStoreErrorKind::IriParseError(e) => Some(e),
            VOWLRStoreErrorKind::LoaderError(e) => Some(e),
            VOWLRStoreErrorKind::QueryEvaluationError(e) => Some(e),
            VOWLRStoreErrorKind::JoinError(e) => Some(e),
            VOWLRStoreErrorKind::StorageError(e) => Some(e),
        }
    }
}
