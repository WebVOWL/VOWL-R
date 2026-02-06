use grapher::prelude::{ElementType, GenericType, OwlType, RdfType, RdfsType};

pub const fn is_property(item: ElementType) -> bool {
    matches!(
        item,
        ElementType::Generic(GenericType::Edge(_))
            | ElementType::Owl(OwlType::Edge(_))
            | ElementType::Rdf(RdfType::Edge(_))
            | ElementType::Rdfs(RdfsType::Edge(_))
    )
}
