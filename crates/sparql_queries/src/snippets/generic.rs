use grapher::prelude::{GenericEdge, GenericNode, GenericType};

use crate::snippets::SparqlSnippet;

impl SparqlSnippet for GenericType {
    fn snippet(self) -> &'static str {
        match self {
            GenericType::Node(node) => node.snippet(),
            GenericType::Edge(edge) => edge.snippet(),
        }
    }
}

impl SparqlSnippet for GenericNode {
    fn snippet(self) -> &'static str {
        match self {
            GenericNode::Generic => todo!(),
        }
    }
}
impl SparqlSnippet for GenericEdge {
    fn snippet(self) -> &'static str {
        match self {
            GenericEdge::Generic => todo!(),
        }
    }
}
