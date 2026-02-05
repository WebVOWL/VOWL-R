use grapher::prelude::{ElementType, OwlNode, OwlType};

pub const fn is_set_operator(item: ElementType) -> bool {
    match item {
        ElementType::Owl(OwlType::Node(node)) => matches!(
            node,
            OwlNode::Complement
                | OwlNode::DisjointUnion
                | OwlNode::IntersectionOf
                | OwlNode::UnionOf
        ),
        _ => false,
    }
}
