use grapher::prelude::{
    ElementType, GenericEdge, GenericNode, GenericType, OwlEdge, OwlNode, OwlType, RdfEdge,
    RdfType, RdfsEdge, RdfsNode, RdfsType,
};

pub trait SparqlSnippet {
    /// Get the SPARQL snippet representing `self`.
    fn snippet(self) -> &'static str;
}

impl SparqlSnippet for ElementType {
    fn snippet(self) -> &'static str {
        match self {
            ElementType::NoDraw => "",
            ElementType::Rdf(RdfType::Edge(edge)) => edge.snippet(),
            ElementType::Rdfs(RdfsType::Node(node)) => node.snippet(),
            ElementType::Rdfs(RdfsType::Edge(edge)) => edge.snippet(),
            ElementType::Owl(OwlType::Node(node)) => node.snippet(),
            ElementType::Owl(OwlType::Edge(edge)) => edge.snippet(),
            ElementType::Generic(GenericType::Node(node)) => node.snippet(),
            ElementType::Generic(GenericType::Edge(edge)) => edge.snippet(),
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
                ?id rdfs:Datatype ?target
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

impl SparqlSnippet for OwlNode {
    fn snippet(self) -> &'static str {
        match self {
            OwlNode::AnonymousClass => {
                r#"{
                ?id a owl:Class
                FILTER(!isIRI(?id))
                BIND("blanknode" AS ?nodeType)
                }"#
            }
            OwlNode::Class => {
                r#"{
                ?id a owl:Class .
                FILTER(isIRI(?id))
                BIND(owl:Class AS ?nodeType)
                }"#
            }
            OwlNode::Complement => {
                r#"{
                ?id owl:complementOf ?target .
                BIND(owl:complementOf AS ?nodeType)
                }"#
            }
            OwlNode::DeprecatedClass => {
                r#"{
                ?id a owl:DeprecatedClass .
                BIND(owl:DeprecatedClass AS ?nodeType)
                }"#
            }
            OwlNode::ExternalClass => {
                // Not handled here as externals uses identical
                // logic across classes and properties.
                ""
            }
            OwlNode::EquivalentClass => {
                r#"{
                ?id owl:equivalentClass ?target
                BIND(owl:equivalentClass AS ?nodeType)
                }"#
            }
            OwlNode::DisjointUnion => {
                r#"{
                ?id owl:disjointUnionOf ?target .
                BIND(owl:disjointUnionOf AS ?nodeType)
                }"#
            }
            OwlNode::IntersectionOf => {
                r#"{
                ?id owl:intersectionOf ?target .
                BIND(owl:intersectionOf AS ?nodeType)
                }"#
            }
            OwlNode::Thing => {
                r#"{
                ?id a owl:Thing .
                BIND(owl:Thing AS ?nodeType)
                }"#
            }
            OwlNode::UnionOf => {
                r#"{
                ?id owl:unionOf ?list .
                BIND(owl:unionOf AS ?nodeType)
                }"#
            }
        }
    }
}

impl SparqlSnippet for OwlEdge {
    fn snippet(self) -> &'static str {
        match self {
            OwlEdge::DatatypeProperty => {
                r#"{
                ?id a owl:DatatypeProperty 
                BIND(owl:DatatypeProperty AS ?nodeType)
                }"#
            }
            OwlEdge::DisjointWith => {
                r#"{
                ?id owl:disjointWith ?target
                BIND(owl:disjointWith AS ?nodeType)
                }"#
            }
            OwlEdge::DeprecatedProperty => {
                r#"{
                ?id a owl:DeprecatedProperty .
                BIND(owl:DeprecatedProperty AS ?nodeType)
                }"#
            }
            OwlEdge::ExternalProperty => {
                // Not handled here as externals uses identical
                // logic across classes and properties.
                ""
            }
            OwlEdge::InverseOf => {
                r#"{
                ?id owl:inverseOf ?target .
                BIND(owl:inverseOf AS ?nodeType)
                }"#
            }
            OwlEdge::ObjectProperty => {
                r#"{
                ?id a owl:ObjectProperty
                BIND(owl:ObjectProperty AS ?nodeType)
                }"#
            }
            OwlEdge::ValuesFrom => {
                r#"{
                {
                    ?id owl:someValuesFrom ?target .
                }
                UNION
                {
                    ?id owl:allValuesFrom ?target .
                }
                BIND("ValuesFrom" AS ?nodeType)
                }"#
            }
        }
    }
}
