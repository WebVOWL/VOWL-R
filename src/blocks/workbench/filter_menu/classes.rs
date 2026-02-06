use grapher::prelude::{ElementType, OwlType, RdfsType};

use crate::blocks::workbench::filter_menu::special_operators::is_set_operator;

pub const fn is_owl_class(item: ElementType) -> bool {
    let class = matches!(item, ElementType::Owl(OwlType::Node(_)));
    class && !is_set_operator(item)
}

pub const fn is_rdf_class(item: ElementType) -> bool {
    matches!(item, ElementType::Rdfs(RdfsType::Node(_)))
}
