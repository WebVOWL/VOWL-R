use grapher::prelude::{RdfsEdge, RdfsNode};

use crate::snippets::SparqlSnippet;

impl SparqlSnippet for RdfsNode {
    fn snippet(self) -> &'static str {
        match self {
            RdfsNode::Class => {
                r#"{
                ?id a rdfs:Class .
                FILTER(?id != owl:Class)
                BIND(rdfs:Class AS ?nodeType)
                }"#
            }
            RdfsNode::Literal => {
                r#"{
                ?id a rdfs:Literal .
                BIND(rdfs:Literal AS ?nodeType)
                }"#
            }
            RdfsNode::Resource => {
                r#"{
                ?id a rdfs:Resource .
                FILTER(isIRI(?id) || isBlank(?id))
                BIND(rdfs:Resource AS ?nodeType)
                }"#
            }
            RdfsNode::Datatype => {
                r#"{
                ?id a rdfs:Datatype .
                BIND(rdfs:Datatype AS ?nodeType)
                }"#
            }
        }
    }
}

impl SparqlSnippet for RdfsEdge {
    fn snippet(self) -> &'static str {
        match self {
            RdfsEdge::SubclassOf => {
                r#"{
                ?id rdfs:subClassOf ?target
                BIND(rdfs:subClassOf AS ?nodeType)
                }"#
            }
        }
    }
}
