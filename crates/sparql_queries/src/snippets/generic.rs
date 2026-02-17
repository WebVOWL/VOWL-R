use grapher::prelude::GenericNode;

use crate::snippets::SparqlSnippet;

impl SparqlSnippet for GenericNode {
    fn snippet(self) -> &'static str {
        match self {
            GenericNode::Generic => todo!(),
        }
    }
}
