use std::{
    collections::{HashMap, HashSet},
    fmt::format,
    mem::{swap, take},
    time::{Duration, Instant},
};

use super::{Edge, SerializationDataBuffer, Triple};
use crate::{
    serializers::util::{get_reserved_iris, trim_tag_circumfix},
    vocab::owl,
};
use fluent_uri::Iri;
use futures::StreamExt;
use grapher::prelude::{
    Characteristic, ElementType, GenericEdge, GenericNode, GenericType, GraphDisplayData, OwlEdge,
    OwlNode, OwlType, RdfEdge, RdfType, RdfsEdge, RdfsNode, RdfsType,
};
use log::{debug, error, info, trace, warn};
use oxrdf::{BlankNode, IriParseError, NamedNode, vocab::rdf};
use rdf_fusion::{
    execution::results::QuerySolutionStream,
    model::{Term, vocab::rdfs},
};
use vowlr_parser::errors::VOWLRStoreError;

pub struct GraphDisplayDataSolutionSerializer {
    pub resolvable_iris: HashSet<String>,
}

impl GraphDisplayDataSolutionSerializer {
    pub fn new() -> Self {
        Self {
            resolvable_iris: get_reserved_iris(),
        }
    }

    pub async fn serialize_nodes_stream(
        &self,
        data: &mut GraphDisplayData,
        mut solution_stream: QuerySolutionStream,
    ) -> Result<(), VOWLRStoreError> {
        let mut count: u32 = 0;
        info!("Serializing query solution stream...");
        let start_time = Instant::now();
        let mut data_buffer = SerializationDataBuffer::new();
        while let Some(solution) = solution_stream.next().await {
            let solution = solution?;
            let Some(id_term) = solution.get("id") else {
                continue;
            };
            let Some(node_type_term) = solution.get("nodeType") else {
                continue;
            };

            self.extract_label(&mut data_buffer, solution.get("label"), id_term);

            let triple: Triple = Triple {
                id: id_term.to_owned(),
                element_type: node_type_term.to_owned(),
                target: solution.get("target").map(|term| term.to_owned()),
            };
            self.write_node_triple(&mut data_buffer, triple);
            count += 1;
        }
        self.check_external_classes(&mut data_buffer);
        self.try_resolve_unknown_edges(&mut data_buffer);
        self.check_all_unknowns(&mut data_buffer);

        let finish_time = Instant::now()
            .checked_duration_since(start_time)
            .unwrap_or(Duration::new(0, 0))
            .as_secs_f32();
        info!(
            "Serialization completed in {} s\n \
            \tTotal solutions: {count}\n \
            \tElements       : {}\n \
            \tEdges          : {}\n \
            \tLabels         : {}\n \
            \tCardinalities  : {}\n \
            \tCharacteristics: {}\n\n \
        ",
            finish_time,
            data_buffer.node_element_buffer.len(),
            data_buffer.edge_buffer.len(),
            data_buffer.label_buffer.len(),
            data_buffer.edge_characteristics.len() + data_buffer.node_characteristics.len(),
            0
        );
        if !data_buffer.failed_buffer.is_empty() {
            let mut f = String::from("[\n");
            for (triple, reason) in data_buffer.failed_buffer.iter() {
                match triple {
                    Some(triple) => {
                        f.push_str(format!("\t\t{} : {}\n", triple, reason).as_str());
                    }
                    None => {
                        f.push_str(format!("\t\tNO TRIPLE : {}\n", reason).as_str());
                    }
                }
            }
            f.push(']');
            error!("Failed to serialize: {}", f);
        }
        debug!("{}", data_buffer);
        *data = data_buffer.into();
        debug!("{}", data);
        Ok(())
    }

    /// Extract label info from the query solution and store until
    /// they can be mapped to their ElementType.
    fn extract_label(
        &self,
        data_buffer: &mut SerializationDataBuffer,
        label: Option<&Term>,
        id_term: &Term,
    ) {
        let iri = id_term.to_string();

        // Prevent overriding labels
        if data_buffer.label_buffer.contains_key(&iri) {
            return;
        }

        match label {
            // Case 1: Label is a rdfs:label OR rdfs:Resource OR rdf:ID
            Some(label) => {
                if label.to_string() != "" {
                    data_buffer
                        .label_buffer
                        .insert(id_term.to_string(), label.to_string());
                } else {
                    debug!("Empty label detected for iri '{iri}'");
                }
            }
            // Case 2: Try parsing the iri
            None => {
                match Iri::parse(self.trim_tag_circumfix(&iri)) {
                    // Case 2.1: Look for fragments in the iri
                    Ok(id_iri) => match id_iri.fragment() {
                        Some(frag) => {
                            data_buffer
                                .label_buffer
                                .insert(id_term.to_string(), frag.to_string());
                        }
                        // Case 2.2: Look for path in iri
                        None => {
                            debug!("No fragment found in iri '{iri}'");
                            match id_iri.path().rsplit_once('/') {
                                Some(path) => {
                                    data_buffer
                                        .label_buffer
                                        .insert(id_term.to_string(), path.1.to_string());
                                }
                                None => {
                                    debug!("No path found in iri '{iri}'");
                                }
                            }
                        }
                    },
                    Err(e) => {
                        // Do not make a 'warn!'. A parse error is allowed to happen (e.g. on blank nodes).
                        debug!("Failed to parse iri '{}':\n{:?}", iri, e);
                    }
                }
            }
        };
    }

    fn resolve(&self, data_buffer: &SerializationDataBuffer, mut x: String) -> Option<String> {
        if let Some(elem) = data_buffer.node_element_buffer.get(&x) {
            debug!("Resolved: {}: {}", x, elem);
            return Some(x);
        } else {
            if let Some(elem) = data_buffer.edge_element_buffer.get(&x) {
                debug!("Resolved: {}: {}", x, elem);
                return Some(x);
            }
        }

        while let Some(redirected) = data_buffer.edge_redirection.get(&x) {
            trace!("Redirected: {} -> {}", x, redirected);
            let new_x = redirected.clone();
            if let Some(elem) = data_buffer.node_element_buffer.get(&new_x) {
                debug!("Resolved: {}: {}", new_x, elem);
                return Some(new_x);
            } else if let Some(elem) = data_buffer.edge_element_buffer.get(&new_x) {
                debug!("Resolved: {}: {}", new_x, elem);
                return Some(new_x);
            }
            debug!("Checked: {} ", new_x);
            x = new_x;
        }
        None
    }
    fn resolve_so(
        &self,
        data_buffer: &SerializationDataBuffer,
        triple: &Triple,
    ) -> (Option<String>, Option<String>) {
        let resolved_subject = self.resolve(data_buffer, triple.id.to_string());
        let resolved_object = match &triple.target {
            Some(target) => self.resolve(data_buffer, target.to_string()),
            None => {
                warn!("Cannot resolve object of triple:\n {}", triple);
                None
            }
        };
        (resolved_subject, resolved_object)
    }

    /// Add subject of triple to the element buffer.
    ///
    /// In the future, this function will handle cases where an element
    /// identifies itself as multiple elements. E.g. an element is both an rdfs:Class and a owl:class.
    fn add_to_element_buffer(
        &self,
        element_buffer: &mut HashMap<String, ElementType>,
        triple: &Triple,
        element_type: ElementType,
    ) {
        let subj_iri = triple.id.to_string();
        if let Some(element) = element_buffer.get(&subj_iri) {
            warn!(
                "Attempted to register '{}' to subject '{}' already registered as '{}'. Skipping",
                element_type, subj_iri, element
            );
        } else {
            element_buffer.insert(triple.id.clone(), element_type);
        }
    }
    /// Add an IRI to the unresolved, unknown buffer.
    fn add_to_unknown_buffer(
        &self,
        data_buffer: &mut SerializationDataBuffer,
        element_iri: String,
        triple: Triple,
    ) {
        if let Some(id_unknowns) = data_buffer.unknown_buffer.get_mut(&element_iri) {
            id_unknowns.insert(triple);
        } else {
            let mut id_unknowns = HashSet::new();
            id_unknowns.insert(triple);
            data_buffer.unknown_buffer.insert(element_iri, id_unknowns);
        }
    }

    /// Insert an edge into the element's edge set.
    fn insert_edge_include(
        &self,
        data_buffer: &mut SerializationDataBuffer,
        element_iri: String,
        edge: Edge,
    ) {
        if data_buffer.edges_include_map.contains_key(&element_iri) {
            data_buffer
                .edges_include_map
                .get_mut(&element_iri)
                .unwrap()
                .insert(edge);
        } else {
            data_buffer
                .edges_include_map
                .insert(element_iri, HashSet::from([edge]));
        }
    }

    pub fn redirect_iri(
        &self,
        data_buffer: &mut SerializationDataBuffer,
        old: &String,
        new: &String,
    ) {
        debug!("Redirecting '{}' to '{}'", old, new);
        data_buffer
            .edge_redirection
            .insert(old.to_string(), new.to_string());
        self.check_unknown_buffer(data_buffer, old);
    }

    pub fn check_unknown_buffer(&self, data_buffer: &mut SerializationDataBuffer, iri: &String) {
        let triple = data_buffer.unknown_buffer.remove(iri);
        if let Some(triples) = triple {
            for triple in triples {
                self.write_node_triple(data_buffer, triple);
            }
        }
    }

    fn insert_node(
        &self,
        data_buffer: &mut SerializationDataBuffer,
        triple: &Triple,
        node_type: ElementType,
    ) {
        // Skip insertion if this node was already merged into another node
        if data_buffer
            .edge_redirection
            .contains_key(&triple.id.to_string())
        {
            debug!(
                "Skipping insert_node for '{}': already redirected",
                triple.id
            );
            return;
        }

        let new_type = if self.is_external(data_buffer, &triple.id.to_string()) {
            ElementType::Owl(OwlType::Node(OwlNode::ExternalClass))
        } else {
            node_type
        };
        self.add_to_element_buffer(&mut data_buffer.node_element_buffer, triple, new_type);
        self.check_unknown_buffer(data_buffer, &triple.id.to_string());
    }

    /// Inserts an edge triple into the serialization buffer,
    /// where subject and object are both nodes.
    ///
    /// Note that tuples or any triple where the subject is an edge iri,
    /// not present in the element buffer, will NEVER be resolved!
    fn insert_edge(
        &self,
        data_buffer: &mut SerializationDataBuffer,
        triple: &Triple,
        edge_type: ElementType,
        label: Option<String>,
    ) -> Option<Edge> {
        // Skip external check for NoDraw edges - they should always retain their type
        let new_type = if edge_type != ElementType::NoDraw
            && self.is_external(data_buffer, &triple.id.to_string())
        {
            ElementType::Owl(OwlType::Edge(OwlEdge::ExternalProperty))
        } else {
            edge_type
        };

        match self.resolve_so(data_buffer, &triple) {
            (Some(sub_iri), Some(obj_iri)) => {
                let edge = Edge {
                    subject: sub_iri.clone(),
                    element_type: new_type,
                    object: obj_iri.clone(),
                };
                data_buffer.edge_buffer.insert(edge.clone());
                self.insert_edge_include(data_buffer, sub_iri, edge.clone());
                self.insert_edge_include(data_buffer, obj_iri, edge.clone());

                data_buffer
                    .edge_label_buffer
                    .insert(edge.clone(), label.unwrap_or(new_type.to_string()));
                return Some(edge);
            }
            (None, Some(_)) => {
                warn!("Cannot resolve subject of triple:\n {}", triple);
                self.add_to_unknown_buffer(data_buffer, triple.id.to_string(), triple.clone());
            }
            (Some(_), None) => {
                if let Some(obj_iri) = &triple.target {
                    // resolve_so already warns about unresolved object. No need to repeat it here.
                    self.add_to_unknown_buffer(data_buffer, obj_iri.to_string(), triple.clone());
                }
            }
            _ => {
                self.add_to_unknown_buffer(data_buffer, triple.id.to_string(), triple.clone());
            }
        }
        None
    }

    fn is_external(&self, data_buffer: &SerializationDataBuffer, iri: &String) -> bool {
        match &data_buffer.document_base {
            Some(base) => !clean_iri.contains(base) && !self.resolvable_iris.contains(&clean_iri),
            None => {
                warn!("Cannot determine externals: Missing document base!");
                false
            }
        }
    }

    fn merge_nodes(&self, data_buffer: &mut SerializationDataBuffer, old: String, new: String) {
        debug!("Merging node '{old}' into '{new}'");
        data_buffer.node_element_buffer.remove(&old);
        self.update_edges(data_buffer, &old, &new);
        self.redirect_iri(data_buffer, &old, &new);
    }

    fn update_edges(&self, data_buffer: &mut SerializationDataBuffer, old: &String, new: &String) {
        let old_edges = data_buffer.edges_include_map.remove(old);
        if let Some(old_edges) = old_edges {
            debug!("Updating edges from '{}' to '{}'", old, new);
            // info!("old_edges: ");
            // for edge in old_edges.iter() {
            //     info!("edge: {} ", edge);
            // }

            for mut edge in old_edges.into_iter() {
                data_buffer.edge_buffer.remove(&edge);
                if edge.object == *old {
                    edge.object = new.clone();
                }
                if edge.subject == *old {
                    edge.subject = new.clone();
                }
                data_buffer.edge_buffer.insert(edge.clone());
                self.insert_edge_include(data_buffer, new.clone(), edge.clone());
            }
            // info!("new_edges: ");
            // for edge in data_buffer.edge_buffer.iter() {
            //     info!("edge: {} ", edge);
            // }
        }
    }

    fn upgrade_node_type(
        &self,
        data_buffer: &mut SerializationDataBuffer,
        iri: String,
        new_element: ElementType,
    ) {
        let old_elem_opt = data_buffer.node_element_buffer.get(&iri).cloned();
        match old_elem_opt {
            Some(old_elem) => {
                if old_elem == ElementType::Owl(OwlType::Node(OwlNode::Class)) {
                    data_buffer
                        .node_element_buffer
                        .insert(iri.clone(), new_element);
                }
                debug!(
                    "Upgraded subject '{}' from {} to {}",
                    iri, old_elem, new_element
                )
            }
            None => {
                warn!("Upgraded unresolved subject '{}' to {}", iri, new_element)
            }
        }
    }

    /// Appends a string to an element's label.
    fn extend_element_label(
        &self,
        data_buffer: &mut SerializationDataBuffer,
        element: String,
        label_to_append: String,
    ) {
        debug!(
            "Extending element '{}' with label '{}'",
            element, label_to_append
        );
        if let Some(label) = data_buffer.label_buffer.get_mut(&element) {
            label.push_str(format!("\n{}", label_to_append).as_str());
        } else {
            data_buffer
                .label_buffer
                .insert(element.clone(), label_to_append.clone());
        }
    }

    fn check_all_unknowns(&self, data_buffer: &mut SerializationDataBuffer) {
        info!("Third pass: Resolving all possible unknowns");

        let unknowns = take(&mut data_buffer.unknown_buffer);
        for (_, triples) in unknowns {
            for triple in triples {
                self.write_node_triple(data_buffer, triple);
            }
        }

        let unknown_edges = take(&mut data_buffer.unknown_edge_buffer);
        for (_, directions) in unknown_edges {
            for triple in directions.domains {
                self.write_node_triple(data_buffer, triple);
            }
            for triple in directions.ranges {
                self.write_node_triple(data_buffer, triple);
            }
        }
    }

    fn check_external_classes(&self, data_buffer: &mut SerializationDataBuffer) {
        let mut triples_to_add = Vec::new();
        for (idx, _) in data_buffer.unknown_buffer.iter() {
            if self.is_external(data_buffer, idx) {
                // dummy triple, only subject matters.
                let triple = Triple::new(
                    Term::NamedNode(NamedNode::new(self.trim_tag_circumfix(idx)).unwrap()),
                    Term::BlankNode(BlankNode::new("_:external_class").unwrap()),
                    None,
                );
                triples_to_add.push(triple);
            }
        }
        for triple in triples_to_add {
            self.insert_node(
                data_buffer,
                &triple,
                ElementType::Owl(OwlType::Node(OwlNode::ExternalClass)),
            );
        }
    }

    /// Serialize a triple to `data_buffer`.
    fn write_node_triple(&self, data_buffer: &mut SerializationDataBuffer, triple: Triple) {
        // TODO: Collect errors and show to frontend
        debug!("{}", triple);
        match &triple.element_type {
            Term::BlankNode(bnode) => {
                // The query must never put blank nodes in the ?nodeType variable
                let msg = format!(
                    "Illegal blank node during serialization: '{}'",
                    bnode.to_string()
                );
                data_buffer.failed_buffer.push((Some(triple), msg));
                return;
            }
            Term::Literal(literal) => {
                // NOTE: Any string literal goes here, e.g. 'EquivalentClass'.
                // That is, every BIND("someString" AS ?nodeType)
                let value = literal.value();
                match value {
                    "blanknode" => {
                        info!("Visualizing blank node: {}", triple.id);
                        self.insert_node(
                            data_buffer,
                            &triple,
                            ElementType::Owl(OwlType::Node(OwlNode::AnonymousClass)),
                        );
                    }
                    &_ => {
                        warn!("Visualization of literal '{value}' is not supported");
                    }
                }
            }
            Term::NamedNode(uri) => {
                // NOTE: Only supports RDF 1.1
                match uri.as_ref() {
                    // ----------- RDF ----------- //

                    // rdf::ALT => {}
                    // rdf::BAG => {}
                    // rdf::FIRST => {}
                    // rdf::HTML => {}
                    // rdf::LANG_STRING => {}
                    // rdf::LIST => {}
                    // rdf::NIL => {}
                    // rdf::OBJECT => {}
                    // rdf::PREDICATE => {}
                    rdf::PROPERTY => {
                        self.insert_edge(
                            data_buffer,
                            &triple,
                            ElementType::Rdf(RdfType::Edge(RdfEdge::RdfProperty)),
                            None,
                        );
                    }
                    // rdf::REST => {}
                    // rdf::SEQ => {}
                    // rdf::STATEMENT => {}
                    // rdf::SUBJECT => {}
                    // rdf::TYPE => {}
                    // rdf::VALUE => {}
                    // rdf::XML_LITERAL => {}

                    // ----------- RDFS ----------- //
                    rdfs::CLASS => self.insert_node(
                        data_buffer,
                        &triple,
                        ElementType::Rdfs(RdfsType::Node(RdfsNode::Class)),
                    ),

                    //TODO: OWL1
                    // rdfs::COMMENT => {}

                    // rdfs::CONTAINER => {}
                    // rdfs::CONTAINER_MEMBERSHIP_PROPERTY => {}
                    rdfs::DATATYPE => {
                        self.insert_node(
                            data_buffer,
                            &triple,
                            ElementType::Rdfs(RdfsType::Node(RdfsNode::Datatype)),
                        );
                    }

                    // NOTE: Domain is handled in the SPARQL query.
                    // Thus, we don't need to match it here.
                    // rdfs::DOMAIN => {}

                    //TODO: OWL1
                    // rdfs::IS_DEFINED_BY => {}

                    // rdfs::LABEL => {}
                    rdfs::LITERAL => {
                        self.insert_node(
                            data_buffer,
                            &triple,
                            ElementType::Rdfs(RdfsType::Node(RdfsNode::Literal)),
                        );
                    }
                    // rdfs::MEMBER => {}

                    // NOTE: Range is handled in the SPARQL query.
                    // Thus, we don't need to match it here.
                    // rdfs::RANGE => {}
                    rdfs::RESOURCE => {
                        self.insert_node(
                            data_buffer,
                            &triple,
                            ElementType::Rdfs(RdfsType::Node(RdfsNode::Resource)),
                        );
                    }

                    //TODO: OWL1
                    // rdfs::SEE_ALSO => {}
                    rdfs::SUB_CLASS_OF => {
                        self.insert_edge(
                            data_buffer,
                            &triple,
                            ElementType::Rdfs(RdfsType::Edge(RdfsEdge::SubclassOf)),
                            None,
                        );
                    }
                    //TODO: OWL1
                    //rdfs::SUB_PROPERTY_OF => {},

                    // ----------- OWL 2 ----------- //

                    //TODO: OWL1
                    // owl::ALL_DIFFERENT => {},

                    // owl::ALL_DISJOINT_CLASSES => {},
                    // owl::ALL_DISJOINT_PROPERTIES => {},

                    //TODO: OWL1
                    // owl::ALL_VALUES_FROM => {}

                    // owl::ANNOTATED_PROPERTY => {},
                    // owl::ANNOTATED_SOURCE => {},
                    // owl::ANNOTATED_TARGET => {},
                    // owl::ANNOTATION => {},

                    //TODO: OWL1
                    // owl::ANNOTATION_PROPERTY => {},

                    // owl::ASSERTION_PROPERTY => {},

                    //TODO: OWL1
                    // owl::ASYMMETRIC_PROPERTY => {},

                    // owl::AXIOM => {},
                    // owl::BACKWARD_COMPATIBLE_WITH => {},
                    // owl::BOTTOM_DATA_PROPERTY => {},
                    // owl::BOTTOM_OBJECT_PROPERTY => {},

                    //TODO: OWL1
                    // owl::CARDINALITY => {}
                    owl::CLASS => self.insert_node(
                        data_buffer,
                        &triple,
                        ElementType::Owl(OwlType::Node(OwlNode::Class)),
                    ),
                    owl::COMPLEMENT_OF => {
                        self.insert_edge(data_buffer, &triple, ElementType::NoDraw, None);
                        if let Some(_) = triple.target {
                            if let Some(index) = self.resolve(data_buffer, triple.id.to_string()) {
                                self.upgrade_node_type(
                                    data_buffer,
                                    index,
                                    ElementType::Owl(OwlType::Node(OwlNode::Complement)),
                                );
                            }
                        }
                    }

                    //TODO: OWL1
                    //owl::DATATYPE_COMPLEMENT_OF => {}
                    owl::DATATYPE_PROPERTY => {
                        self.insert_edge(
                            data_buffer,
                            &triple,
                            ElementType::Owl(OwlType::Edge(OwlEdge::DatatypeProperty)),
                            None,
                        );
                    }

                    //TODO: OWL1 (deprecated in OWL2, replaced by rdfs:datatype)
                    // owl::DATA_RANGE => {}

                    // owl::DEPRECATED => {}
                    owl::DEPRECATED_CLASS => self.insert_node(
                        data_buffer,
                        &triple,
                        ElementType::Owl(OwlType::Node(OwlNode::DeprecatedClass)),
                    ),
                    owl::DEPRECATED_PROPERTY => {
                        self.insert_edge(
                            data_buffer,
                            &triple,
                            ElementType::Owl(OwlType::Edge(OwlEdge::DeprecatedProperty)),
                            None,
                        );
                    }

                    //TODO: OWL1
                    // owl::DIFFERENT_FROM => {}
                    owl::DISJOINT_UNION_OF => {
                        self.insert_edge(data_buffer, &triple, ElementType::NoDraw, None);
                        if let Some(_) = triple.target {
                            if let Some(index) = self.resolve(data_buffer, triple.id.to_string()) {
                                self.upgrade_node_type(
                                    data_buffer,
                                    index,
                                    ElementType::Owl(OwlType::Node(OwlNode::DisjointUnion)),
                                );
                            }
                        }
                    }
                    owl::DISJOINT_WITH => {
                        self.insert_edge(
                            data_buffer,
                            &triple,
                            ElementType::Owl(OwlType::Edge(OwlEdge::DisjointWith)),
                            None,
                        );
                    }

                    //TODO: OWL1
                    // owl::DISTINCT_MEMBERS => {}
                    owl::EQUIVALENT_CLASS => {
                        match &triple.target {
                            Some(target) => {
                                if target.is_named_node() {
                                    // Case 1:
                                    // The subject of an equivalentClass relation should
                                    // become a full-fledged equivalent class. This happens
                                    // if the subject and object of the equivalentClass relation
                                    // are both named classes (i.e. not blank nodes).
                                    //
                                    // In other words, the object must be removed from existence,
                                    // and have all references to it (incl. labels) point to
                                    // the subject.
                                    let target_str = target.to_string();

                                    // Move object label to subject.
                                    if let Some(label) =
                                        data_buffer.label_buffer.remove(&target_str)
                                    {
                                        debug!("Removed label: {}", label);
                                        self.extend_element_label(
                                            data_buffer,
                                            triple.id.to_string(),
                                            label,
                                        );
                                    }

                                    // Remove object from existence.
                                    match data_buffer.node_element_buffer.remove(&target_str) {
                                        // Case 1.1: Object exists in the elememt buffer
                                        Some(_) => {
                                            self.merge_nodes(
                                                data_buffer,
                                                target_str,
                                                triple.id.to_string(),
                                            );
                                        }
                                        // Case 1.2: Look in the unknown buffer
                                        None => {
                                            match data_buffer.unknown_buffer.remove(&target_str) {
                                                Some(items) => {
                                                    if items.len() > 0 {
                                                        warn!(
                                                            "Removed unresolved triples for object '{}' during merge into equivalent subject '{}':\n\t{:#?}",
                                                            target_str, triple.id, items
                                                        );
                                                    }
                                                }
                                                None => {
                                                    data_buffer.failed_buffer.push((Some(triple), "Failed to merge object of equivalence relation into subject: object not found".to_string()));
                                                    return;
                                                }
                                            }
                                        }
                                    }
                                    self.upgrade_node_type(
                                        data_buffer,
                                        triple.id.to_string(),
                                        ElementType::Owl(OwlType::Node(OwlNode::EquivalentClass)),
                                    );
                                } else if target.is_blank_node() {
                                    // Case 2:
                                    // The subject of an equivalentClass relation should
                                    // could either be start of a collection or anon class
                                    let (index_s, index_o) = self.resolve_so(data_buffer, &triple);
                                    match (index_s, index_o) {
                                        (Some(index_s), Some(index_o)) => {
                                            self.merge_nodes(data_buffer, index_o, index_s);
                                        }
                                        (Some(index_s), None) => {
                                            self.redirect_iri(
                                                data_buffer,
                                                &triple.target.unwrap().to_string(),
                                                &index_s,
                                            );
                                        }
                                        _ => {
                                            self.add_to_unknown_buffer(
                                                data_buffer,
                                                target.to_string(),
                                                triple,
                                            );
                                        }
                                    }
                                } else {
                                    data_buffer.failed_buffer.push((Some(triple), "Visualization of equivalence relations between classes and literals is not supported".to_string()));
                                }
                            }
                            None => {
                                data_buffer.failed_buffer.push((
                                    Some(triple),
                                    "Subject of equivalence relation is missing an object"
                                        .to_string(),
                                ));
                            }
                        }
                    }
                    // owl::EQUIVALENT_PROPERTY => {}

                    //TODO: OWL1
                    //owl::FUNCTIONAL_PROPERTY => {}

                    // owl::HAS_KEY => {}
                    // owl::HAS_SELF => {}

                    //TODO: OWL1
                    // owl::HAS_VALUE => {}

                    // owl::IMPORTS => {}
                    // owl::INCOMPATIBLE_WITH => {}
                    owl::INTERSECTION_OF => {
                        let edge =
                            self.insert_edge(data_buffer, &triple, ElementType::NoDraw, None);
                        if let Some(edge) = edge {
                            self.upgrade_node_type(
                                data_buffer,
                                edge.subject,
                                ElementType::Owl(OwlType::Node(OwlNode::IntersectionOf)),
                            );
                        }
                    }
                    //TODO: OWL1
                    // owl::INVERSE_FUNCTIONAL_PROPERTY => {}

                    //TODO: OWL1
                    // owl::INVERSE_OF => {}

                    //TODO: OWL1
                    // owl::IRREFLEXIVE_PROPERTY => {}

                    //TODO: OWL1
                    // owl::MAX_CARDINALITY => {}

                    // owl::MAX_QUALIFIED_CARDINALITY => {}
                    // owl::MEMBERS => {}

                    //TODO: OWL1
                    // owl::MIN_CARDINALITY => {}
                    // owl::MIN_QUALIFIED_CARDINALITY => {}
                    // owl::NAMED_INDIVIDUAL => {}
                    // owl::NEGATIVE_PROPERTY_ASSERTION => {}

                    //TODO: OWL1
                    //owl::NOTHING => {}
                    owl::OBJECT_PROPERTY => {
                        let e = ElementType::Owl(OwlType::Edge(OwlEdge::ObjectProperty));
                        self.add_to_element_buffer(
                            &mut data_buffer.edge_element_buffer,
                            &triple,
                            e,
                        );
                    }
                    // owl::ONE_OF => {}
                    owl::ONTOLOGY => {
                        if let Some(base) = &data_buffer.document_base {
                            warn!(
                                "Attempting to override document base '{}' with new base '{}'. Skipping",
                                base,
                                triple.id.to_string()
                            );
                        } else {
                            // Remove ">" to enable substring matching
                            let id = triple.id.to_string();
                            let base = id[0..id.len() - 1].to_string();
                            info!("Using document base: '{}'", base);
                            data_buffer.document_base = Some(base);
                        }
                    }

                    //TODO: OWL1
                    // owl::ONTOLOGY_PROPERTY => {}

                    // owl::ON_CLASS => {}
                    // owl::ON_DATARANGE => {}
                    // owl::ON_DATATYPE => {}
                    // owl::ON_PROPERTIES => {}

                    //TODO: OWL1
                    // owl::ON_PROPERTY => {}

                    // owl::PRIOR_VERSION => {}
                    // owl::PROPERTY_CHAIN_AXIOM => {}
                    // owl::PROPERTY_DISJOINT_WITH => {}
                    // owl::QUALIFIED_CARDINALITY => {}

                    //TODO: OWL1
                    // owl::REFLEXIVE_PROPERTY => {}

                    //TODO: OWL1
                    // owl::RESTRICTION => {}

                    //TODO: OWL1
                    // owl::SAME_AS => {}

                    //TODO: OWL1
                    // owl::SOME_VALUES_FROM => {}
                    // owl::SOURCE_INDIVIDUAL => {}
                    // owl::SYMMETRIC_PROPERTY => {}
                    // owl::TARGET_INDIVIDUAL => {}
                    // owl::TARGET_VALUE => {}
                    owl::THING => self.insert_node(
                        data_buffer,
                        &triple,
                        ElementType::Owl(OwlType::Node(OwlNode::Thing)),
                    ),
                    // owl::TOP_DATA_PROPERTY => {}
                    // owl::TOP_OBJECT_PROPERTY => {}

                    //TODO: OWL1
                    //owl::TRANSITIVE_PROPERTY => {}
                    owl::UNION_OF => {
                        let edge =
                            self.insert_edge(data_buffer, &triple, ElementType::NoDraw, None);
                        if let Some(edge) = edge {
                            self.upgrade_node_type(
                                data_buffer,
                                edge.subject,
                                ElementType::Owl(OwlType::Node(OwlNode::UnionOf)),
                            );
                        }
                    }
                    // owl::VERSION_INFO => {}
                    // owl::VERSION_IRI => {}
                    // owl::WITH_RESTRICTIONS => {}
                    _ => {
                        // Visualization of this element is not supported
                        warn!("Visualization of term '{}' is not supported", uri);
                    }
                };
            }
        }
    }
}

#[cfg(test)]
#[allow(unused_must_use)]
mod test {
    use super::*;
    use oxrdf::{BlankNode, Literal, NamedNode};

    #[test]
    fn test_replace_node() {
        let _ = env_logger::builder().is_test(true).try_init();
        let serializer = GraphDisplayDataSolutionSerializer::new();
        let mut data_buffer = SerializationDataBuffer::new();
        serializer.write_node_triple(
            &mut data_buffer,
            Triple {
                id: Term::NamedNode(NamedNode::new("http://example.com#").unwrap()),
                element_type: Term::NamedNode(
                    NamedNode::new("http://www.w3.org/2002/07/owl#Ontology").unwrap(),
                ),
                target: None,
            },
        );
        serializer.write_node_triple(
            &mut data_buffer,
            Triple {
                id: Term::NamedNode(NamedNode::new("http://example.com#Parent").unwrap()),
                element_type: Term::NamedNode(
                    NamedNode::new("http://www.w3.org/2002/07/owl#Class").unwrap(),
                ),
                target: None,
            },
        );
        serializer.write_node_triple(
            &mut data_buffer,
            Triple {
                id: Term::NamedNode(NamedNode::new("http://example.com#Mother").unwrap()),
                element_type: Term::NamedNode(
                    NamedNode::new("http://www.w3.org/2002/07/owl#Class").unwrap(),
                ),
                target: None,
            },
        );
        serializer.write_node_triple(
            &mut data_buffer,
            Triple {
                id: Term::NamedNode(NamedNode::new("http://example.com#Guardian").unwrap()),
                element_type: Term::NamedNode(
                    NamedNode::new("http://www.w3.org/2002/07/owl#Class").unwrap(),
                ),
                target: None,
            },
        );
        serializer.write_node_triple(
            &mut data_buffer,
            Triple {
                id: Term::NamedNode(NamedNode::new("http://example.com#Warden").unwrap()),
                element_type: Term::NamedNode(
                    NamedNode::new("http://www.w3.org/2002/07/owl#Class").unwrap(),
                ),
                target: None,
            },
        );
        serializer.write_node_triple(
            &mut data_buffer,
            Triple {
                id: Term::NamedNode(NamedNode::new("http://example.com#Warden1").unwrap()),
                element_type: Term::NamedNode(
                    NamedNode::new("http://www.w3.org/2002/07/owl#Class").unwrap(),
                ),
                target: None,
            },
        );
        serializer.write_node_triple(
            &mut data_buffer,
            Triple {
                id: Term::NamedNode(NamedNode::new("http://example.com#Warden").unwrap()),
                element_type: Term::NamedNode(
                    NamedNode::new("http://www.w3.org/2000/01/rdf-schema#subClassOf").unwrap(),
                ),
                target: Some(Term::NamedNode(
                    NamedNode::new("http://example.com#Guardian").unwrap(),
                )),
            },
        );
        serializer.write_node_triple(
            &mut data_buffer,
            Triple {
                id: Term::NamedNode(NamedNode::new("http://example.com#Mother").unwrap()),
                element_type: Term::NamedNode(
                    NamedNode::new("http://www.w3.org/2000/01/rdf-schema#subClassOf").unwrap(),
                ),
                target: Some(Term::NamedNode(
                    NamedNode::new("http://example.com#Parent").unwrap(),
                )),
            },
        );
        serializer.write_node_triple(
            &mut data_buffer,
            Triple {
                id: Term::BlankNode(BlankNode::new("e1013e66f734c508511575854b0c9396").unwrap()),
                element_type: Term::Literal(Literal::new_simple_literal("blanknode".to_string())),
                target: None,
            },
        );
        serializer.write_node_triple(
            &mut data_buffer,
            Triple {
                id: Term::NamedNode(NamedNode::new("http://example.com#Warden1").unwrap()),
                element_type: Term::NamedNode(
                    NamedNode::new("http://www.w3.org/2002/07/owl#unionOf").unwrap(),
                ),
                target: Some(Term::NamedNode(
                    NamedNode::new("http://example.com#Warden").unwrap(),
                )),
            },
        );

        print_graph_display_data(&data_buffer);
        println!("--------------------------------");

        let triple = Triple {
            id: Term::NamedNode(NamedNode::new("http://example.com#Guardian").unwrap()),
            element_type: Term::NamedNode(
                NamedNode::new("http://www.w3.org/2002/07/owl#equivalentClass").unwrap(),
            ),
            target: Some(Term::NamedNode(
                NamedNode::new("http://example.com#Warden").unwrap(),
            )),
        };
        serializer.write_node_triple(&mut data_buffer, triple);
        for (k, v) in data_buffer.node_element_buffer.iter() {
            println!("element_buffer: {} -> {}", k, v);
        }
        for (k, v) in data_buffer.edges_include_map.iter() {
            println!("edges_include_map: {} -> {:?}", k, v);
        }
        for (k, v) in data_buffer.edge_redirection.iter() {
            println!("edge_redirection: {} -> {}", k, v);
        }
        assert!(
            data_buffer
                .node_element_buffer
                .contains_key("<http://example.com#Guardian>")
        );
        assert!(
            !data_buffer
                .node_element_buffer
                .contains_key("<http://example.com#Warden>")
        );
        assert!(
            data_buffer
                .node_element_buffer
                .contains_key("<http://example.com#Warden1>")
        );
        assert!(
            data_buffer
                .edges_include_map
                .contains_key("<http://example.com#Warden1>")
        );
        assert!(
            *data_buffer
                .edge_redirection
                .get("<http://example.com#Warden>")
                .unwrap()
                == "<http://example.com#Guardian>".to_string()
        );
        assert!(data_buffer.edge_buffer.contains(&Edge {
            subject: "<http://example.com#Warden1>".to_string(),
            element_type: ElementType::NoDraw,
            object: "<http://example.com#Guardian>".to_string()
        }));
        assert!(
            data_buffer
                .edge_redirection
                .contains_key("<http://example.com#Warden>")
        );
        assert_eq!(
            data_buffer
                .edge_redirection
                .get("<http://example.com#Warden>")
                .unwrap(),
            "<http://example.com#Guardian>"
        );
        serializer.write_node_triple(
            &mut data_buffer,
            Triple {
                id: Term::NamedNode(NamedNode::new("http://example.com#Guardian").unwrap()),
                element_type: Term::NamedNode(
                    NamedNode::new("http://www.w3.org/2002/07/owl#equivalentClass").unwrap(),
                ),
                target: Some(Term::BlankNode(
                    BlankNode::new("e1013e66f734c508511575854b0c9396").unwrap(),
                )),
            },
        );
        let s = serializer.resolve(
            &mut data_buffer,
            "_:e1013e66f734c508511575854b0c9396".to_string(),
        );
        assert!(s.is_some());
        for (k, v) in data_buffer.node_element_buffer.iter() {
            println!("element_buffer: {} -> {}", k, v);
        }
        for (k, v) in data_buffer.edge_redirection.iter() {
            println!("edge_redirection: {} -> {}", k, v);
        }
        assert!(s.unwrap() == "<http://example.com#Guardian>".to_string());
        assert!(
            !data_buffer
                .edges_include_map
                .contains_key("_:e1013e66f734c508511575854b0c9396")
        );
        assert!(!data_buffer.edges_include_map.contains_key("Warden"));
        print_graph_display_data(&data_buffer);
        println!("data_buffer: {}", data_buffer);
    }

    pub fn print_graph_display_data(data_buffer: &SerializationDataBuffer) {
        for (index, (element, label)) in data_buffer.node_element_buffer.iter().enumerate() {
            println!("{index}: {label} -> {element:?}");
        }
        for edge in data_buffer.edge_buffer.iter() {
            println!(
                "{} -> {:?} -> {}",
                edge.subject, edge.element_type, edge.object
            );
        }
    }
}
