use grapher::prelude::{ElementType, GenericType, OwlType, RdfType, RdfsType, XSDType};

use crate::snippets::SparqlSnippet;

impl SparqlSnippet for ElementType {
    fn snippet(self) -> &'static str {
        match self {
            Self::NoDraw => "",
            Self::Rdf(RdfType::Node(node)) => node.snippet(),
            Self::Rdf(RdfType::Edge(edge)) => edge.snippet(),
            Self::Rdfs(RdfsType::Node(node)) => node.snippet(),
            Self::Rdfs(RdfsType::Edge(edge)) => edge.snippet(),
            Self::Owl(OwlType::Node(node)) => node.snippet(),
            Self::Owl(OwlType::Edge(edge)) => edge.snippet(),
            Self::Generic(GenericType::Node(node)) => node.snippet(),
            Self::Generic(GenericType::Edge(edge)) => edge.snippet(),
            Self::Xsd(XSDType::Node(node)) => node.snippet(),
        }
    }
}
