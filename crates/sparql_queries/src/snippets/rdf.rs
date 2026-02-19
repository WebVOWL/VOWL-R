use grapher::prelude::RdfEdge;

use crate::snippets::SparqlSnippet;

impl SparqlSnippet for RdfEdge {
    fn snippet(self) -> &'static str {
        match self {
            RdfEdge::RdfProperty => {
                r#"{
                ?id rdf:Property ?target
                BIND(rdf:Property AS ?nodeType)
                }"#
            }
        }
    }
}
