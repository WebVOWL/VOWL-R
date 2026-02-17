use grapher::prelude::{ElementType, OwlEdge, OwlType, RdfEdge, RdfType};

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

pub const PROPERTY_EDGE_TYPES: [ElementType; 7] =
    [ElementType::Owl(OwlType::Edge(OwlEdge::ObjectProperty)),
    ElementType::Owl(OwlType::Edge(OwlEdge::DatatypeProperty)),
    ElementType::Owl(OwlType::Edge(OwlEdge::DeprecatedProperty)),
    ElementType::Owl(OwlType::Edge(OwlEdge::ExternalProperty)),
    ElementType::Owl(OwlType::Edge(OwlEdge::ValuesFrom)),
    ElementType::Owl(OwlType::Edge(OwlEdge::InverseOf)),
    ElementType::Rdf(RdfType::Edge(RdfEdge::RdfProperty)),
];