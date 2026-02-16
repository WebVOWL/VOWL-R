use grapher::prelude::{ElementType, OwlEdge, OwlType};

pub mod serializers;
pub mod store;
pub mod vocab;

pub mod prelude {
    pub use crate::serializers::frontend::GraphDisplayDataSolutionSerializer;
    pub use rdf_fusion::execution::results::QueryResults;

    pub use crate::store::VOWLRStore;
}

pub const SYMMETRIC_EDGE_TYPES: [ElementType; 1] = [ElementType::Owl(OwlType::Edge(OwlEdge::DisjointWith))];
