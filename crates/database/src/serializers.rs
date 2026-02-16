use std::{
    collections::{HashMap, HashSet},
    fmt::{Display, Formatter},
    hash::{Hash, Hasher},
};

use grapher::prelude::{ElementType, GraphDisplayData, OwlEdge, OwlType};
use log::error;
use oxrdf::Term;

use crate::SYMMETRIC_EDGE_TYPES;

pub mod frontend;
pub mod util;

#[derive(Debug, Hash, Clone, Eq, PartialEq)]
pub struct Triple {
    /// The subject
    id: Term,
    /// The predicate
    element_type: Term,
    /// The object
    target: Option<Term>,
}

impl Display for Triple {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Triple{{ ")?;
        write!(f, "{} - ", self.id)?;
        write!(f, "{} - ", self.element_type)?;
        write!(
            f,
            "{}",
            self.target
                .as_ref()
                .map(|t| t.to_string())
                .unwrap_or_default(),
        )?;
        write!(f, "}}")
    }
}

impl Triple {
    pub fn new(id: Term, element_type: Term, target: Option<Term>) -> Self {
        Self {
            id,
            element_type,
            target,
        }
    }
}

#[derive(Debug, Clone, Eq)]
pub struct Edge {
    /// The subject IRI
    subject: Term,
    /// The element type
    element_type: ElementType,
    /// The object IRI
    object: Term,
    /// The property IRI
    property: Option<Term>,
}

impl PartialEq for Edge {
    fn eq(&self, other: &Self) -> bool {
        // Element type and property must always match
        if self.element_type != other.element_type || self.property != other.property {
            return false;
        }

        // For symmetric relations, treat (A, B) and (B, A) as equal
        let eq_so = [ElementType::Owl(OwlType::Edge(OwlEdge::DisjointWith))];
        if eq_so.contains(&self.element_type) {
            (self.subject == other.subject && self.object == other.object)
                || (self.subject == other.object && self.object == other.subject)
        } else {
            self.subject == other.subject && self.object == other.object
        }
    }
}

impl Hash for Edge {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if SYMMETRIC_EDGE_TYPES.contains(&self.element_type) {
            // For symmetric relations, hash the sorted pair
            let (first, second) = if self.subject <= self.object {
                (&self.subject, &self.object)
            } else {
                (&self.object, &self.subject)
            };

            first.hash(state);
            second.hash(state);
            self.element_type.hash(state);
        } else if self.element_type == ElementType::Owl(OwlType::Edge(OwlEdge::ObjectProperty)) {
            self.subject.hash(state);
            self.element_type.hash(state);
            self.object.hash(state);
            self.property.hash(state);
        } else {
            self.subject.hash(state);
            self.element_type.hash(state);
            self.object.hash(state);
        }
    }
}

impl Display for Edge {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Edge{{ {} - {:?} - {} }}",
            self.subject, self.element_type, self.object
        )?;
        Ok(())
    }
}

pub struct SerializationDataBuffer {
    /// Stores all resolved node elements.
    ///
    /// These elements may mutate during serialization
    /// if new information regarding them is found.
    /// This also means an element can be completely removed!
    ///
    /// - Key = The subject IRI of a triple.
    /// - Value = The ElementType of `Key`.
    node_element_buffer: HashMap<Term, ElementType>,
    /// Stores all resolved edge elements.
    ///
    /// These elements may mutate during serialization
    /// if new information regarding them is found.
    /// This also means an element can be completely removed!
    ///
    /// - Key = The subject IRI of a triple.
    /// - Value = The ElementType of `Key`.
    edge_element_buffer: HashMap<Term, ElementType>,
    /// Keeps track of edges that should point to a node different
    /// from their definition.
    ///
    /// Key
    /// ---
    /// The object IRI of an edge triple.
    ///
    /// The object is also called:
    /// - the target of an edge.
    /// - the range of an edge.
    ///
    /// Value
    /// -----
    /// The subject IRI of an edge triple.
    ///
    /// The subject is also called:
    /// - the source of an edge.
    /// - the domain of an edge.
    ///
    /// Example
    /// -------
    /// Consider the triples:
    /// ```sparql
    ///     ex:Mother owl:equivalentClass ex:blanknode1
    ///
    ///     ex:blanknode1 rdf:type owl:Class
    ///     ex:blanknode1 owl:intersectionOf ex:blanknode2
    /// ```
    /// Here `ex:Mother` is equivalent to `ex:blanknode1`,
    /// which means all edges referencing `ex:blanknode1` should
    /// be redirected to `ex:Mother`.
    ///
    /// Thus, the edges are redirected to:
    /// ```sparql
    ///     ex:Mother owl:intersectionOf ex:blanknode2
    /// ```
    /// In this case, `blanknode1` is effectively omitted from serialization.
    edge_redirection: HashMap<Term, Term>,
    /// Maps from element IRI to a set of the edges that include it.
    ///
    /// Used to remap when nodes are merges.
    edges_include_map: HashMap<Term, HashSet<Edge>>,
    /// Stores indices of element instances.
    ///
    /// Used in cases where multiple elements should refer to a particular instance.
    /// E.g. multiple properties referring to the same instance of owl:Thing.
    global_element_mappings: HashMap<ElementType, usize>,

    /// Stores the edges of a property.
    ///
    /// - Key = The property IRI.
    /// - Value = The edges of the property.
    property_edge_map: HashMap<String, Edge>,
    /// Stores the domains of a property.
    ///
    /// - Key = The property IRI.
    /// - Value = The domains of the property.
    property_domain_map: HashMap<String, HashSet<String>>,
    /// Stores the ranges of a property.
    ///
    /// - Key = The property IRI.
    /// - Value = The ranges of the property.
    property_range_map: HashMap<String, HashSet<String>>,
    /// Stores labels of subject/object.
    ///
    /// - Key = The IRI the label belongs to.
    /// - Value = The label.
    label_buffer: HashMap<Term, String>,
    /// Stores labels of edges.
    ///
    /// - Key = The edge.
    /// - Value = The label.
    edge_label_buffer: HashMap<Edge, String>,
    /// Edges in graph, to avoid duplicates
    edge_buffer: HashSet<Edge>,
    /// Maps from edge to its characteristic.
    edge_characteristics: HashMap<Edge, Vec<String>>,
    /// Maps from node iri to its characteristics.
    node_characteristics: HashMap<Term, Vec<String>>,
    /// Stores unresolved triples.
    ///
    /// - Key = The unresolved IRI of the triple
    ///   can be either the subject, object or both (in this case, subject is used)
    /// - Value = The unresolved triples.
    unknown_buffer: HashMap<Term, HashSet<Triple>>,
    /// Stores triples that are impossible to serialize.
    ///
    /// This could be caused by various reasons, such as
    /// visualization of the triple is not supported.
    ///
    /// Each element is a tuple of:
    /// - 0 = The triple (if any).
    /// - 1 = The reason it failed to serialize (or the message if no triple is available).
    failed_buffer: Vec<(Option<Triple>, String)>,
    /// The base IRI of the document.
    ///
    /// For instance: `http://purl.obolibrary.org/obo/envo.owl`
    document_base: Option<String>,
}
impl SerializationDataBuffer {
    pub fn new() -> Self {
        Self {
            node_element_buffer: HashMap::new(),
            edge_element_buffer: HashMap::new(),
            edge_redirection: HashMap::new(),
            edges_include_map: HashMap::new(),
            global_element_mappings: HashMap::new(),
            label_buffer: HashMap::new(),
            edge_label_buffer: HashMap::new(),
            edge_buffer: HashSet::new(),
            property_edge_map: HashMap::new(),
            property_domain_map: HashMap::new(),
            property_range_map: HashMap::new(),
            unknown_buffer: HashMap::new(),
            failed_buffer: Vec::new(),
            document_base: None,
            edge_characteristics: HashMap::new(),
            node_characteristics: HashMap::new(),
        }
    }
}
impl SerializationDataBuffer {
    pub fn add_property_edge(&mut self, property_iri: String, edge: Edge) {
        self.property_edge_map.insert(property_iri, edge);
    }
    pub fn add_property_domain(&mut self, property_iri: String, domain: String) {
        self.property_domain_map
            .entry(property_iri)
            .or_default()
            .insert(domain);
    }
    pub fn add_property_range(&mut self, property_iri: String, range: String) {
        self.property_range_map
            .entry(property_iri)
            .or_default()
            .insert(range);
    }
}

impl Default for SerializationDataBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl From<SerializationDataBuffer> for GraphDisplayData {
    fn from(mut val: SerializationDataBuffer) -> Self {
        let mut display_data = GraphDisplayData::new();
        let mut iricache: HashMap<Term, usize> = HashMap::new();
        for (iri, element) in val.node_element_buffer.into_iter() {
            let label = val.label_buffer.remove(&iri);
            match label {
                Some(label) => {
                    display_data.labels.push(label);
                    display_data.elements.push(element);
                    iricache.insert(iri, display_data.elements.len() - 1);
                }
                None => {
                    error!("Label not found for iri: {}, using default", iri);
                    display_data.labels.push(element.to_string());
                    display_data.elements.push(element);
                    iricache.insert(iri, display_data.elements.len() - 1);
                }
            }
        }

        for edge in val.edge_buffer.iter() {
            let subject_idx = iricache.get(&edge.subject);
            let object_idx = iricache.get(&edge.object);
            let maybe_label = val.edge_label_buffer.remove(edge);
            let characteristics = val.edge_characteristics.remove(edge);

            match (subject_idx, object_idx, maybe_label) {
                (Some(subject_idx), Some(object_idx), Some(label)) => {
                    display_data.elements.push(edge.element_type);
                    display_data.labels.push(label);
                    display_data.edges.push([
                        *subject_idx,
                        display_data.elements.len() - 1,
                        *object_idx,
                    ]);
                    if let Some(characteristics) = characteristics {
                        display_data
                            .characteristics
                            .insert(display_data.elements.len() - 1, characteristics.join("\n"));
                    }
                }
                (Some(_), Some(_), None) => {
                    error!("Label in edge not found in iricache: {}", edge.subject);
                }
                (None, _, _) => {
                    error!("Subject in edge not found in iricache: {}", edge.subject);
                }
                (_, None, _) => {
                    error!("Object in edge not found in iricache: {}", edge.object);
                }
            }
        }

        for (iri, mut characteristics) in val.node_characteristics.into_iter() {
            let idx = iricache.get(&iri);
            match idx {
                Some(idx) => {
                    display_data
                        .characteristics
                        .insert(*idx, characteristics.pop().unwrap());
                }
                None => {
                    error!("Characteristic not found for node in iricache: {}", iri);
                }
            }
        }
        // TODO: handle cardinalities

        display_data
    }
}

impl Display for SerializationDataBuffer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "SerializationDataBuffer {{")?;
        writeln!(
            f,
            "\tdocument_base: {}",
            self.document_base.as_ref().unwrap_or(&"".to_string())
        )?;
        writeln!(f, "\tnode_element_buffer:")?;
        for (iri, element) in self.node_element_buffer.iter() {
            writeln!(f, "\t\t{} : {}", iri, element)?;
        }
        writeln!(f, "\tedge_element_buffer (not used by into()):")?;
        for (iri, element) in self.edge_element_buffer.iter() {
            writeln!(f, "\t\t{} : {}", iri, element)?;
        }
        writeln!(f, "\tedge_redirection:")?;
        for (iri, subject) in self.edge_redirection.iter() {
            writeln!(f, "\t\t{} -> {}", iri, subject)?;
        }
        writeln!(f, "\tedges_include_map: ")?;
        for (iri, edges) in self.edges_include_map.iter() {
            writeln!(f, "\t\t{} : {{", iri)?;
            for edge in edges.iter() {
                writeln!(f, "\t\t\t{}", edge)?;
            }
            writeln!(f, "\t\t}}")?;
        }
        writeln!(f, "\tglobal_element_mappings:")?;
        for (element, index) in self.global_element_mappings.iter() {
            writeln!(f, "\t\t{} : {}", element, index)?;
        }
        writeln!(f, "\tlabel_buffer:")?;
        for (iri, label) in self.label_buffer.iter() {
            writeln!(f, "\t\t{} : {}", iri, label)?;
        }
        writeln!(f, "\tedge_buffer:")?;
        for edge in self.edge_buffer.iter() {
            writeln!(f, "\t\t{}", edge)?;
        }
        writeln!(f, "\tedge_characteristics: {:?}", self.edge_characteristics)?;
        writeln!(f, "\tnode_characteristics: {:?}", self.node_characteristics)?;
        writeln!(f, "\tunknown_buffer:")?;
        for (iri, triples) in self.unknown_buffer.iter() {
            write!(f, "\t\t{} : ", iri)?;
            writeln!(
                f,
                "{}",
                triples
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<String>>()
                    .join("\n")
            )?;
        }
        writeln!(f, "\tfailed_buffer:")?;
        for (triple, reason) in self.failed_buffer.iter() {
            match triple {
                Some(triple) => {
                    writeln!(f, "\t\t{} : {}", triple, reason)?;
                }
                None => {
                    writeln!(f, "\t\tNO TRIPLE : {}", reason)?;
                }
            }
        }
        writeln!(f, "}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_disjoint_with_edge_symmetry() {
        // Create two edges with swapped subject and object
        let edge1 = Edge {
            subject: "_:x".to_string(),
            element_type: ElementType::Owl(OwlType::Edge(OwlEdge::DisjointWith)),
            object: "_:y".to_string(),
            property: None,
        };

        let edge2 = Edge {
            subject: "_:y".to_string(),
            element_type: ElementType::Owl(OwlType::Edge(OwlEdge::DisjointWith)),
            object: "_:x".to_string(),
            property: None,
        };

        // Test that they are equal
        assert_eq!(
            edge1, edge2,
            "DisjointWith edges should be equal regardless of subject/object order"
        );

        // Test that they hash to the same value by inserting into a HashSet
        let mut edge_set = HashSet::new();
        edge_set.insert(edge1.clone());
        edge_set.insert(edge2.clone());

        assert_eq!(
            edge_set.len(),
            1,
            "HashSet should only contain one edge when both are DisjointWith with swapped subject/object"
        );
    }

    #[test]
    fn test_non_symmetric_edge_distinction() {
        // Create two edges with swapped subject and object for a non-symmetric relation
        let edge1 = Edge {
            subject: "_:x".to_string(),
            element_type: ElementType::Owl(OwlType::Edge(OwlEdge::ObjectProperty)),
            object: "_:y".to_string(),
            property: Some("prop1".to_string()),
        };

        let edge2 = Edge {
            subject: "_:y".to_string(),
            element_type: ElementType::Owl(OwlType::Edge(OwlEdge::ObjectProperty)),
            object: "_:x".to_string(),
            property: Some("prop1".to_string()),
        };

        // Test that they are NOT equal
<<<<<<< HEAD
        assert_ne!(
            edge1, edge2,
            "Non-symmetric edges should NOT be equal when subject/object are swapped"
        );
=======
        assert_ne!(edge1, edge2, "Non-symmetric edges should NOT be equal when subject/object are swapped");
>>>>>>> 770dfd0 (fixed Rework Object/Datatype-properties)

        // Test that they both appear in the HashSet
        let mut edge_set = HashSet::new();
        edge_set.insert(edge1.clone());
        edge_set.insert(edge2.clone());

        assert_eq!(
            edge_set.len(),
            2,
            "HashSet should contain both edges when they are non-symmetric"
        );
    }
}
