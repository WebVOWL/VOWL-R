use std::path::Path;

use rkyv::{Archive, Deserialize as RDeserialize, Serialize as RSerialize};
use serde::{Deserialize, Serialize};

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
use rdf_fusion::io::{RdfFormat, JsonLdProfileSet};

/// Supported content types.
#[repr(C)]
#[derive(Archive, RDeserialize, RSerialize, Deserialize, Serialize, Debug, Clone)]
pub enum DataType {
    OWL,
    OFN,
    OWX,
    TTL,
    RDF,
    NTriples,
    NQuads,
    TriG,
    JsonLd,
    N3,
    SPARQLJSON,
    SPARQLXML,
    SPARQLCSV,
    SPARQLTSV,
    /// Fallback when type can't be determined.
    UNKNOWN,
}

impl DataType {
    // Fixed string literals called by reference as to not allocate new memory each time the function is called
    /// Get mime type of the data.
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::OWL => "application/owl+xml",
            Self::OFN => "text/ofn",
            Self::OWX => "application/owl+xml",
            Self::TTL => "text/turtle",
            Self::RDF => "application/rdf+xml",
            Self::NTriples => "application/n-triples",
            Self::NQuads => "application/n-quads",
            Self::TriG => "application/trig",
            Self::JsonLd => "application/ld+json",
            Self::N3 => "text/n3",
            Self::SPARQLJSON => "application/sparql-results+json",
            Self::SPARQLXML => "application/sparql-results+xml",
            Self::SPARQLCSV => "text/csv",
            Self::SPARQLTSV => "text/tab-separated-values",
            Self::UNKNOWN => "application/octet-stream",
        }
    }

    pub fn format_from_resource_type(&self) -> Option<RdfFormat> {
    match self {
        DataType::RDF => Some(RdfFormat::RdfXml),
        DataType::TTL => Some(RdfFormat::Turtle),
        DataType::NTriples => Some(RdfFormat::NTriples),
        DataType::NQuads => Some(RdfFormat::NQuads),
        DataType::TriG => Some(RdfFormat::TriG),
        DataType::JsonLd => Some(RdfFormat::JsonLd {
            profile: JsonLdProfileSet::default(),
        }),
        DataType::N3 => Some(RdfFormat::N3),
        DataType::OWL => Some(RdfFormat::RdfXml),
        _ => None,
    }
}
}

impl From<&Path> for DataType {
    fn from(value: &Path) -> Self {
        match value.extension().and_then(|os| os.to_str()) {
            Some(ext) => ext.into(),
            None => Self::UNKNOWN,
        }
    }
}

impl From<&str> for DataType {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "owl" => Self::OWL,
            "ofn" => Self::OFN,
            "owx" => Self::OWX,
            "ttl" => Self::TTL,
            "rdf" => Self::RDF,
            "nt" => Self::NTriples,
            "nq" => Self::NQuads,
            "trig" => Self::TriG,
            "jsonld" => Self::JsonLd,
            "n3" => Self::N3,
            "srj" | "json" => Self::SPARQLJSON,
            "srx" | "xml" => Self::SPARQLXML,
            "src" | "csv" => Self::SPARQLCSV,
            "tsv" => Self::SPARQLTSV, //TODO: Figure out file extension for TSV and if the file extension of TSV SPARQL Query Result differs.
            _ => Self::UNKNOWN,
        }
    }
}

impl From<String> for DataType {
    fn from(value: String) -> Self {
        value.to_lowercase().as_str().into()
    }
}
