use grapher::prelude::{GenericEdge, GenericNode, GenericType};

use crate::snippets::SparqlSnippet;

impl SparqlSnippet for GenericType {
    fn snippet(self) -> &'static str {
        match self {
            Self::Node(node) => node.snippet(),
            Self::Edge(edge) => edge.snippet(),
        }
    }
}

impl SparqlSnippet for GenericNode {
    fn snippet(self) -> &'static str {
        match self {
            Self::Generic => todo!(),
        }
    }
}
impl SparqlSnippet for GenericEdge {
    fn snippet(self) -> &'static str {
        match self {
            Self::Generic => todo!(),
        }
    }
}
