use std::fmt::{Display, Formatter};

use grapher::prelude::{ElementType, OwlEdge, OwlType, RdfEdge, RdfType};
use oxrdf::{BlankNodeIdParseError, IriParseError};
use vowlr_parser::errors::VOWLRStoreError;

use crate::serializers::Triple;

pub mod serializers;
pub mod store;
pub mod vocab;

pub mod prelude {
    pub use crate::serializers::frontend::GraphDisplayDataSolutionSerializer;
    pub use rdf_fusion::execution::results::QueryResults;

    pub use crate::store::VOWLRStore;
}

pub const SYMMETRIC_EDGE_TYPES: [ElementType; 1] =
    [ElementType::Owl(OwlType::Edge(OwlEdge::DisjointWith))];

pub const PROPERTY_EDGE_TYPES: [ElementType; 7] = [
    ElementType::Owl(OwlType::Edge(OwlEdge::ObjectProperty)),
    ElementType::Owl(OwlType::Edge(OwlEdge::DatatypeProperty)),
    ElementType::Owl(OwlType::Edge(OwlEdge::DeprecatedProperty)),
    ElementType::Owl(OwlType::Edge(OwlEdge::ExternalProperty)),
    ElementType::Owl(OwlType::Edge(OwlEdge::ValuesFrom)),
    ElementType::Owl(OwlType::Edge(OwlEdge::InverseOf)),
    ElementType::Rdf(RdfType::Edge(RdfEdge::RdfProperty)),
];

pub trait SerializationErrorExt {
    fn triple(&self) -> Option<&Triple>;
}

macro_rules! ser_err {
    ($variant:ident($triple:expr, $msg:expr)) => {
        $crate::SerializationErrorKind::$variant(($triple).map(Box::new), $msg)
    };
}
pub(crate) use ser_err;

#[derive(Debug)]
pub enum SerializationErrorKind {
    MissingObject(Option<Box<Triple>>, String),
    MissingSubject(Option<Box<Triple>>, String),
    SerializationFailed(Option<Box<Triple>>, String),
    IriParseError(Option<Box<Triple>>, Box<IriParseError>),
    BlankNodeParseError(Option<Box<Triple>>, Box<BlankNodeIdParseError>),
}
impl SerializationErrorExt for SerializationErrorKind {
    fn triple(&self) -> Option<&Triple> {
        match &self {
            SerializationErrorKind::MissingObject(triple, _)
            | SerializationErrorKind::MissingSubject(triple, _)
            | SerializationErrorKind::SerializationFailed(triple, _)
            | SerializationErrorKind::IriParseError(triple, _)
            | SerializationErrorKind::BlankNodeParseError(triple, _) => {
                triple.as_ref().map(|t| &**t)
            }
        }
    }
}

#[derive(Debug)]
pub struct SerializationError {
    inner: SerializationErrorKind,
}
impl Display for SerializationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "SerializationError: {:?}", self.inner)
    }
}

impl SerializationErrorExt for SerializationError {
    fn triple(&self) -> Option<&Triple> {
        self.inner.triple()
    }
}

impl From<SerializationErrorKind> for SerializationError {
    fn from(error: SerializationErrorKind) -> Self {
        SerializationError { inner: error }
    }
}

impl From<IriParseError> for SerializationError {
    fn from(error: IriParseError) -> Self {
        SerializationError {
            inner: SerializationErrorKind::IriParseError(None, Box::new(error)),
        }
    }
}

impl From<SerializationError> for VOWLRStoreError {
    fn from(error: SerializationError) -> Self {
        VOWLRStoreError::from(error.to_string())
    }
}

impl From<BlankNodeIdParseError> for SerializationError {
    fn from(error: BlankNodeIdParseError) -> Self {
        SerializationError {
            inner: SerializationErrorKind::BlankNodeParseError(None, Box::new(error)),
        }
    }
}
