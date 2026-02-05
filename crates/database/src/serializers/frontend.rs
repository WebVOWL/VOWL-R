use std::{
    collections::{HashMap, HashSet},
    fmt::format,
    mem::{swap, take},
    time::{Duration, Instant},
};

use super::{Edge, SerializationDataBuffer, Triple};
use crate::{
    serializers::{EdgeDirectionHint, EdgeDirections},
    vocab::owl,
};
use fluent_uri::Iri;
use futures::StreamExt;
use grapher::prelude::{
    Characteristic, ElementType, GenericEdge, GenericNode, GenericType, GraphDisplayData, OwlEdge,
    OwlNode, OwlType, RdfEdge, RdfType, RdfsEdge, RdfsNode, RdfsType,
};
use log::{debug, error, info, trace, warn};
use oxrdf::{IriParseError, NamedNode, vocab::rdf};
use rdf_fusion::{
    execution::results::QuerySolutionStream,
    model::{Term, vocab::rdfs},
};
use vowlr_parser::errors::VOWLRStoreError;

pub struct GraphDisplayDataSolutionSerializer {
    pub resolvable_iris: HashMap<String, (NamedNode, ElementType)>,
}

impl GraphDisplayDataSolutionSerializer {
    pub fn new() -> Self {
        let resolvables = HashMap::from_iter([
            (
                rdfs::LITERAL.to_string(),
                (
                    rdfs::LITERAL.into_owned(),
                    ElementType::Rdfs(RdfsType::Node(RdfsNode::Literal)),
                ),
            ),
            (
                owl::THING.to_string(),
                (
                    owl::THING.into_owned(),
                    ElementType::Owl(OwlType::Node(OwlNode::Thing)),
                ),
            ),
        ]);
        Self {
            resolvable_iris: resolvables,
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
                // Remove '<' and '>' from iri string to
                // comply with https://www.ietf.org/rfc/rfc3987.html (p. 12)
                let compliant_iri = iri[1..iri.len() - 1].to_string();
                match Iri::parse(compliant_iri) {
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
            element_buffer.insert(subj_iri, element_type);
        }
    }

    /// Add a subject edge IRI to the partially resolved edge buffer.
    fn add_to_unknown_edge_buffer(
        &self,
        data_buffer: &mut SerializationDataBuffer,
        subject_iri: String,
        triple: Triple,
        hint: EdgeDirectionHint,
    ) {
        if let Some(direction) = data_buffer.unknown_edge_buffer.get_mut(&subject_iri) {
            match hint {
                EdgeDirectionHint::Domain => {
                    direction.domains.insert(triple);
                }
                EdgeDirectionHint::Range => {
                    direction.ranges.insert(triple);
                }
            }
        } else {
            let mut direction = EdgeDirections::new();

            match hint {
                EdgeDirectionHint::Domain => {
                    direction.domains.insert(triple);
                }
                EdgeDirectionHint::Range => {
                    direction.ranges.insert(triple);
                }
            }

            data_buffer
                .unknown_edge_buffer
                .insert(subject_iri, direction);
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

        let new_type = if self.is_external(data_buffer, &triple.id) {
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
        let new_type =
            if edge_type != ElementType::NoDraw && self.is_external(data_buffer, &triple.id) {
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

    fn is_external(&self, data_buffer: &SerializationDataBuffer, iri: &Term) -> bool {
        !iri.is_blank_node()
            && match &data_buffer.document_base {
                Some(base) => !iri.to_string().starts_with(base),
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

    fn create_node(
        &self,
        id: String,
        node_type: NamedNode,
        object_iri: Option<String>,
    ) -> Result<Triple, IriParseError> {
        let subject = NamedNode::new(id)?;
        let object = match object_iri {
            Some(iri) => {
                let obj = NamedNode::new(iri)?;
                Some(Term::NamedNode(obj))
            }
            None => None,
        };

        let t = Triple::new(Term::NamedNode(subject), Term::NamedNode(node_type), object);
        debug!("Created new triple: {}", t);
        Ok(t)
    }

    fn try_resolve_unknown_node(
        &self,
        node_element_buffer: &mut HashMap<String, ElementType>,
        label_buffer: &mut HashMap<String, String>,
        iri: &String,
        suffix: String,
    ) -> Result<Triple, String> {
        match self.resolvable_iris.get(iri) {
            Some((node_type, element_type)) => {
                // Remove "<" and ">" from IRI.
                let mut new_iri = iri[1..iri.len() - 1].to_string();
                // Add unique identifier to IRI.
                new_iri.push_str(suffix.as_str());
                // Create a range node.
                let triple = self
                    .create_node(new_iri, node_type.clone(), None)
                    .map_err(|e| format!("{:#?}", e))?;

                label_buffer.insert(triple.id.to_string(), element_type.to_string());
                self.add_to_element_buffer(node_element_buffer, &triple, *element_type);
                debug!("Resolved unknown iri '{}' : {}", iri, element_type);
                Ok(triple)
            }
            // Case 2.1.2: We're unable to resolve the range with the current serializer logic.
            None => Err(format!("Cannot resolve '{}'", iri)),
        }
    }

    /// Attempts to resolve edges which couldn't be mapped to a domain and/or range during serialization.
    ///
    /// If no domain and/or range is defined for a property, owl:Thing is used as domain and/or range.
    /// EXCEPT datatype properties without a defined range. Here, rdfs:Literal is used as range instead.
    ///
    /// Procedure
    /// ---------
    /// If NOT rdfs:Datatype:
    /// - Property missing domain OR range:
    ///   - Create new owl:Thing for this property.
    /// - Property missing domain AND range:
    ///   - Create a new owl:Thing (if not created previously) and use this instance for ALL edges in this category.
    ///
    /// If rdfs:Datatype:
    /// - Property missing domain AND/OR range:
    ///   - Create new rdfs:Literal for this property
    fn try_resolve_unknown_edges(&self, data_buffer: &mut SerializationDataBuffer) {
        info!("Second pass: Resolving unknown edges");

        // Creates a node IRI based on the Procedure described in the docstring of this function.
        let create_iri = |suffix: Option<String>, is_global: bool, edge_type| {
            match edge_type {
                ElementType::Owl(OwlType::Edge(OwlEdge::DatatypeProperty)) => {
                    let mut iri = rdfs::LITERAL.as_str().to_string();
                    match suffix {
                        Some(suffix) => {
                            iri.push_str(suffix.as_str());
                            Ok((
                                iri,
                                rdfs::LITERAL,
                                ElementType::Rdfs(RdfsType::Node(RdfsNode::Literal)),
                            ))
                        }
                        None => Err(format!(
                            "An IRI suffix is required when creating node type '{}'",
                            iri
                        )),
                    }
                }
                _ => {
                    let mut iri = owl::THING.as_str().to_string();
                    let element = owl::THING;
                    let element_type = ElementType::Owl(OwlType::Node(OwlNode::Thing));
                    match (suffix, is_global) {
                        //
                        (Some(suffix), false) => {
                            iri.push_str(suffix.as_str());
                            Ok((iri, element, element_type))
                        }
                        (_, true) => Ok((iri, element, element_type)),
                        _ => Err("Tried to create a local node with a global iri".to_string()),
                    }
                }
            }
        };

        // Composes functions `create_iri` and `create_node`.
        let create_node_from_suffix = |suffix: Option<String>, is_global: bool, edge_type| {
            create_iri(suffix, is_global, edge_type).and_then(|(iri, node_type, element_type)| {
                self.create_node(iri, node_type.into_owned(), None)
                    .map(|triple| (triple, element_type))
                    .map_err(|e: IriParseError| format!("{:#?}", e))
            })
        };

        for (i, (edge_iri, edge_type)) in data_buffer.edge_element_buffer.iter().enumerate() {
            let resolved_edge = data_buffer.resolved_edge_map.get(edge_iri);
            let unknown_edge = data_buffer.unknown_edge_buffer.get(edge_iri);
            let unknown = data_buffer.unknown_buffer.get(edge_iri);

            match (resolved_edge, unknown_edge, unknown) {
                // TODO: Handle case where the domain triple is missing from input file.
                // Maybe compare size of domains and ranges maps.

                // TODO: Handle case where the range triple is missing from input file.
                // Maybe compare size of domains and ranges maps.

                // Case 1: Missing domain AND range.
                (None, None, None) => {
                    let domain = create_node_from_suffix(None, true, *edge_type);
                    let range = create_node_from_suffix(None, true, *edge_type);
                    match (&domain, &range) {
                        (Ok((domain, domain_type)), Ok((range, range_type))) => {
                            // We can resolve the edge now.
                            self.add_to_element_buffer(
                                &mut data_buffer.node_element_buffer,
                                domain,
                                *domain_type,
                            );
                            data_buffer
                                .label_buffer
                                .insert(domain.id.to_string(), domain_type.to_string());
                            self.add_to_element_buffer(
                                &mut data_buffer.node_element_buffer,
                                range,
                                *range_type,
                            );
                            data_buffer
                                .label_buffer
                                .insert(range.id.to_string(), range_type.to_string());
                        }
                        (Err(d), Err(r)) => {
                            let d_msg = format!(
                                "Failed to create global domain for edge '{}' : {}",
                                edge_iri, d
                            );
                            let r_msg = format!(
                                "Failed to create global range for edge '{}' : {}",
                                edge_iri, r
                            );
                            data_buffer
                                .failed_buffer
                                .extend([(None, d_msg), (None, r_msg)]);
                        }
                        (Err(d), _) => {
                            let d_msg = format!(
                                "Failed to create global domain for edge '{}' : {}",
                                edge_iri, d
                            );
                            data_buffer.failed_buffer.push((None, d_msg));
                        }
                        (_, Err(r)) => {
                            let r_msg = format!(
                                "Failed to create global range for edge '{}' : {}",
                                edge_iri, r
                            );
                            data_buffer.failed_buffer.push((None, r_msg));
                        }
                    }
                }

                // Case 2: Unresolved domain OR range.
                (None, Some(direction), None) => {
                    // Clone domains/ranges as we're updating the map in the for-loop.
                    let domains = direction.domains.clone();
                    let ranges = direction.ranges.clone();

                    for mut domain_triple in domains {
                        match &domain_triple.target {
                            Some(target_term) => {
                                match self.resolve(data_buffer, target_term.to_string()) {
                                    // Case 2.1: Domain is unresolved. Range is still unknown.
                                    None => {
                                        let target_iri = target_term.to_string();
                                        let resolved = self.try_resolve_unknown_node(
                                            &mut data_buffer.node_element_buffer,
                                            &mut data_buffer.label_buffer,
                                            &target_iri,
                                            format!("_d{}", i),
                                        );
                                        match resolved {
                                            // Case 2.1.1: We know how to resolve the domain.
                                            // By adding the unresolved domain as element_type in a triple and calling
                                            // write_node_triple() with it.
                                            Ok(new_domain_triple) => {
                                                // SAFETY: unwrap is safe because we checked beforehand.
                                                // (it's impossible to reach this point if the value doesn't exist)
                                                let update_direction = data_buffer
                                                    .unknown_edge_buffer
                                                    .get_mut(edge_iri)
                                                    .unwrap();

                                                // Update the IRI of the domain to the new domain's IRI.
                                                // Step 1: Remove old domain triple.
                                                update_direction.domains.remove(&domain_triple);
                                                // Step 2: Override target of old triple.
                                                domain_triple.target = Some(new_domain_triple.id);
                                                // Step 3: Insert overridden old triple.
                                                update_direction.domains.insert(domain_triple);
                                            }
                                            Err(e) => {
                                                let msg = format!(
                                                    "Failed to create local domain for edge '{}' : {}",
                                                    edge_iri, e
                                                );
                                                data_buffer
                                                    .failed_buffer
                                                    .push((Some(domain_triple.clone()), msg));
                                            }
                                        }
                                    }
                                    // Case 2.2: We know the domain (do nothing here). Range must be unresolved or missing.
                                    _ => {}
                                }
                            }
                            // Case 2.3: A domain tuple is illegal. We must have a triple.
                            None => {
                                let msg = format!(
                                    "Failed to create local domain for edge '{}' : object not found",
                                    edge_iri
                                );
                                data_buffer
                                    .failed_buffer
                                    .push((Some(domain_triple.clone()), msg));
                            }
                        }
                    }

                    for mut range_triple in ranges {
                        match &range_triple.target {
                            Some(target_term) => {
                                match self.resolve(data_buffer, target_term.to_string()) {
                                    // Case 2.1: range is unresolved. Range is still unknown.
                                    None => {
                                        let target_iri = target_term.to_string();
                                        let resolved = self.try_resolve_unknown_node(
                                            &mut data_buffer.node_element_buffer,
                                            &mut data_buffer.label_buffer,
                                            &target_iri,
                                            format!("_r{}", i),
                                        );
                                        match resolved {
                                            Ok(new_range_triple) => {
                                                // SAFETY: unwrap is safe because we checked beforehand.
                                                // (it's impossible to reach this point if the value doesn't exist)
                                                let update_direction = data_buffer
                                                    .unknown_edge_buffer
                                                    .get_mut(edge_iri)
                                                    .unwrap();

                                                // Update the IRI of the range to the new range's IRI.
                                                // Step 1: Remove old range triple.
                                                update_direction.ranges.remove(&range_triple);
                                                // Step 2: Override target of old triple.
                                                range_triple.target = Some(new_range_triple.id);
                                                // Step 3: Insert overridden old triple.
                                                update_direction.ranges.insert(range_triple);
                                            }
                                            Err(e) => {
                                                let msg = format!(
                                                    "Failed to create local range for edge '{}' : {}",
                                                    edge_iri, e
                                                );
                                                data_buffer
                                                    .failed_buffer
                                                    .push((Some(range_triple.clone()), msg));
                                            }
                                        }
                                    }
                                    // Case 2.2: We know the range (do nothing here). Range must be unresolved or missing.
                                    _ => {}
                                }
                            }
                            // Case 2.3: A range tuple is illegal. We must have a triple.
                            None => {
                                let msg = format!(
                                    "Failed to create local range for edge '{}' : object not found",
                                    edge_iri
                                );
                                data_buffer
                                    .failed_buffer
                                    .push((Some(range_triple.clone()), msg));
                            }
                        }
                    }
                }

                // Case 3: Unresolved subject AND/OR object
                (None, None, Some(triples)) => {
                    // Clone as we're updating the set's values in the for-loop.
                    for mut triple in triples.clone() {
                        match self.resolve_so(data_buffer, &triple) {
                            // Case 3.1: Unresolved subject AND object
                            (None, None) => {
                                // TODO
                                data_buffer.failed_buffer.push((
                                    Some(triple.clone()),
                                    "Failed to resolve triple : Someone implement Case 3.1"
                                        .to_string(),
                                ));
                            }

                            // Case 3.2: Unresolved subject
                            (None, Some(_)) => {
                                let resolved = self.try_resolve_unknown_node(
                                    &mut data_buffer.node_element_buffer,
                                    &mut data_buffer.label_buffer,
                                    &triple.id.to_string(),
                                    format!("_s{}", i),
                                );
                                match resolved {
                                    Ok(resolved_triple) => {
                                        // SAFETY: unwrap is safe because we checked beforehand.
                                        // (it's impossible to reach this point if the value doesn't exist)
                                        let update_triples =
                                            data_buffer.unknown_buffer.get_mut(edge_iri).unwrap();

                                        // Update the IRI of the domain to the new domain's IRI.
                                        // Step 1: Remove old domain triple.
                                        update_triples.remove(&triple);
                                        // Step 2: Override id of old triple.
                                        triple.id = resolved_triple.id;
                                        // Step 3: Insert overridden old triple.
                                        update_triples.insert(triple);
                                    }
                                    Err(e) => {
                                        let msg = format!("Failed to resolve subject : {}", e);
                                        data_buffer.failed_buffer.push((Some(triple.clone()), msg));
                                    }
                                }
                            }

                            // Case 3.3: Unresolved object
                            (Some(_), None) => match &triple.target {
                                Some(obj_term) => {
                                    let resolved = self.try_resolve_unknown_node(
                                        &mut data_buffer.node_element_buffer,
                                        &mut data_buffer.label_buffer,
                                        &obj_term.to_string(),
                                        format!("_s{}", i),
                                    );
                                    match resolved {
                                        Ok(resolved_triple) => {
                                            // SAFETY: unwrap is safe because we checked beforehand.
                                            // (it's impossible to reach this point if the value doesn't exist)
                                            let update_triples = data_buffer
                                                .unknown_buffer
                                                .get_mut(edge_iri)
                                                .unwrap();

                                            // Update the IRI of the domain to the new domain's IRI.
                                            // Step 1: Remove old domain triple.
                                            update_triples.remove(&triple);
                                            // Step 2: Override target of old triple.
                                            triple.target = Some(resolved_triple.id);
                                            // Step 3: Insert overridden old triple.
                                            update_triples.insert(triple);
                                        }
                                        Err(e) => {
                                            let msg = format!("Failed to resolve object : {}", e);
                                            data_buffer
                                                .failed_buffer
                                                .push((Some(triple.clone()), msg));
                                        }
                                    }
                                }
                                None => {
                                    data_buffer.failed_buffer.push((
                                        Some(triple.clone()),
                                        "Failed to resolve object : object not found".to_string(),
                                    ));
                                }
                            },
                            // Case 3.4: A triple in the unknown buffer was somehow resolvable (do nothing).
                            (Some(_), Some(_)) => {}
                        }
                    }
                }

                _ => {
                    warn!("Check if this should happen : {}", edge_iri);
                }
            }
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
                    rdfs::DOMAIN => {
                        match self.resolve_so(data_buffer, &triple) {
                            // Case 1: We have the edge_iri (thus the edge_type) and the domain.
                            (Some(subj_iri), Some(_)) => {
                                // Look for range in the unknown_edge_buffer and attempt to resolve the edge.
                                match data_buffer.unknown_edge_buffer.remove(&subj_iri) {
                                    Some(direction) => {
                                        // TODO: Handle multiple domains. It is part of RDF spec
                                        if direction.domains.len() > 0 {
                                            let msg = format!(
                                                "Cannot resolve a property '{}' with multiple domains: {:#?}",
                                                subj_iri, direction.domains
                                            );
                                            data_buffer.failed_buffer.push((Some(triple), msg));
                                            return;
                                        }

                                        match direction.ranges.len() {
                                            // Range not yet resolved.
                                            0 => {
                                                self.add_to_unknown_edge_buffer(
                                                    data_buffer,
                                                    subj_iri,
                                                    triple,
                                                    EdgeDirectionHint::Domain,
                                                );
                                            }
                                            // We've found at least one range. Now we must ensure all are well-defined.
                                            _ => {
                                                // Fetch the edge type. We need it to create edges.
                                                let element_type = match data_buffer
                                                    .edge_element_buffer
                                                    .get(&subj_iri)
                                                {
                                                    Some(elem) => *elem,
                                                    None => {
                                                        data_buffer.failed_buffer.push((Some(triple), "Failed to find the element type of a resolved property subject".to_string()));
                                                        return;
                                                    }
                                                };
                                                // Get label here to please borrow checker
                                                let wrapped_label =
                                                    match data_buffer.label_buffer.get(&subj_iri) {
                                                        Some(label) => Some(label.clone()),
                                                        None => None,
                                                    };

                                                for target_triple in direction.ranges.iter() {
                                                    match (&triple.target, &target_triple.target) {
                                                        (Some(tt), Some(ttt)) => {
                                                            let edge_triple = Triple::new(
                                                                tt.clone(),
                                                                // Arbitrary element_type which doesn't matter as long as we're repacking.
                                                                target_triple.id.clone(),
                                                                // Repacking as we must ensure target of new triple is Some().
                                                                Some(ttt.clone()),
                                                            );
                                                            self.insert_edge(
                                                                data_buffer,
                                                                &edge_triple,
                                                                element_type,
                                                                wrapped_label.clone(),
                                                            );
                                                        }
                                                        (None, _) => {
                                                            data_buffer.failed_buffer.push((
                                                            Some(triple),
                                                            "Failed to find domain for property"
                                                                .to_string(),
                                                        ));
                                                            return;
                                                        }
                                                        (_, None) => {
                                                            data_buffer.failed_buffer.push((
                                                                Some(target_triple.clone()),
                                                                "Failed to find range for property"
                                                                    .to_string(),
                                                            ));
                                                            return;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    // Range not yet resolved.
                                    None => {
                                        self.add_to_unknown_edge_buffer(
                                            data_buffer,
                                            subj_iri,
                                            triple,
                                            EdgeDirectionHint::Domain,
                                        );
                                    }
                                }
                            }

                            // Case 2: We don't know what the edge is pointing to.
                            (Some(subj_iri), None) => {
                                self.add_to_unknown_edge_buffer(
                                    data_buffer,
                                    subj_iri,
                                    triple,
                                    EdgeDirectionHint::Domain,
                                );
                            }

                            // Case 3: We don't know enough to resolve any of it.
                            _ => {
                                self.add_to_unknown_buffer(
                                    data_buffer,
                                    triple.id.to_string(),
                                    triple,
                                );
                            }
                        }
                    }

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
                    rdfs::RANGE => {
                        match self.resolve_so(data_buffer, &triple) {
                            // Case 1: We have the edge_iri (thus the edge_type) and the domain.
                            (Some(subj_iri), Some(_)) => {
                                // Look for range in the unknown_edge_buffer and attempt to resolve the edge.
                                match data_buffer.unknown_edge_buffer.remove(&subj_iri) {
                                    Some(direction) => {
                                        // TODO: Handle multiple ranges. It is part of RDF spec
                                        if direction.ranges.len() > 0 {
                                            let msg = format!(
                                                "Cannot resolve a property '{}' with multiple ranges: {:#?}",
                                                subj_iri, direction.domains
                                            );
                                            data_buffer.failed_buffer.push((Some(triple), msg));
                                            return;
                                        }

                                        match direction.domains.len() {
                                            // Domain not yet resolved.
                                            0 => {
                                                self.add_to_unknown_edge_buffer(
                                                    data_buffer,
                                                    subj_iri,
                                                    triple,
                                                    EdgeDirectionHint::Range,
                                                );
                                            }
                                            // We've found at least one domain. Now we must ensure all are well-defined.
                                            _ => {
                                                // Fetch the edge type. We need it to create edges.
                                                let element_type = match data_buffer
                                                    .edge_element_buffer
                                                    .get(&subj_iri)
                                                {
                                                    Some(elem) => *elem,
                                                    None => {
                                                        data_buffer.failed_buffer.push((Some(triple), "Failed to find the element type of a resolved property subject".to_string()));
                                                        return;
                                                    }
                                                };
                                                // Get label here to please borrow checker
                                                let wrapped_label =
                                                    match data_buffer.label_buffer.get(&subj_iri) {
                                                        Some(label) => Some(label.clone()),
                                                        None => None,
                                                    };

                                                for target_triple in direction.domains.iter() {
                                                    match (&triple.target, &target_triple.target) {
                                                        (Some(tt), Some(ttt)) => {
                                                            let edge_triple = Triple::new(
                                                                ttt.clone(),
                                                                // Arbitrary element_type which doesn't matter as long as we're repacking.
                                                                target_triple.id.clone(),
                                                                // Repacking as we must ensure target of new triple is Some().
                                                                Some(tt.clone()),
                                                            );
                                                            self.insert_edge(
                                                                data_buffer,
                                                                &edge_triple,
                                                                element_type,
                                                                wrapped_label.clone(),
                                                            );
                                                        }
                                                        (None, _) => {
                                                            data_buffer.failed_buffer.push((
                                                                Some(triple),
                                                                "Failed to find range for property"
                                                                    .to_string(),
                                                            ));
                                                            return;
                                                        }
                                                        (_, None) => {
                                                            data_buffer.failed_buffer.push((
                                                               Some( target_triple.clone()),
                                                                "Failed to find domain for property"
                                                                    .to_string(),
                                                            ));
                                                            return;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    // Range not yet resolved.
                                    None => {
                                        self.add_to_unknown_edge_buffer(
                                            data_buffer,
                                            subj_iri,
                                            triple,
                                            EdgeDirectionHint::Range,
                                        );
                                    }
                                }
                            }

                            // Case 2: We don't know what the edge is pointing to.
                            (Some(subj_iri), None) => {
                                self.add_to_unknown_edge_buffer(
                                    data_buffer,
                                    subj_iri,
                                    triple,
                                    EdgeDirectionHint::Range,
                                );
                            }

                            // Case 3: We don't know enough to resolve any of it.
                            _ => {
                                self.add_to_unknown_buffer(
                                    data_buffer,
                                    triple.id.to_string(),
                                    triple,
                                );
                            }
                        }
                    }
                    rdfs::RESOURCE => {
                        self.insert_node(
                            data_buffer,
                            &triple,
                            ElementType::Rdfs(RdfsType::Node(RdfsNode::Resource)),
                        );
                    }
                    // rdfs::SEE_ALSO => {}
                    rdfs::SUB_CLASS_OF => {
                        self.insert_edge(
                            data_buffer,
                            &triple,
                            ElementType::Rdfs(RdfsType::Edge(RdfsEdge::SubclassOf)),
                            None,
                        );
                    }
                    // rdfs::SUB_PROPERTY_OF => {},

                    // ----------- OWL 2 ----------- //

                    // owl::ALL_DIFFERENT => {},
                    // owl::ALL_DISJOINT_CLASSES => {},
                    // owl::ALL_DISJOINT_PROPERTIES => {},
                    // owl::ALL_VALUES_FROM => {}
                    // owl::ANNOTATED_PROPERTY => {},
                    // owl::ANNOTATED_SOURCE => {},
                    // owl::ANNOTATED_TARGET => {},
                    // owl::ANNOTATION => {},
                    // owl::ANNOTATION_PROPERTY => {},
                    // owl::ASSERTION_PROPERTY => {},
                    // owl::ASYMMETRIC_PROPERTY => {},
                    // owl::AXIOM => {},
                    // owl::BACKWARD_COMPATIBLE_WITH => {},
                    // owl::BOTTOM_DATA_PROPERTY => {},
                    // owl::BOTTOM_OBJECT_PROPERTY => {},
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
                    // TODO owl::DATATYPE_COMPLEMENT_OF => {}
                    owl::DATATYPE_PROPERTY => {
                        self.insert_edge(
                            data_buffer,
                            &triple,
                            ElementType::Owl(OwlType::Edge(OwlEdge::DatatypeProperty)),
                            None,
                        );
                    }
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
                    // TODO owl::FUNCTIONAL_PROPERTY => {}
                    // owl::HAS_KEY => {}
                    // owl::HAS_SELF => {}
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
                    // TODO owl::INVERSE_FUNCTIONAL_PROPERTY => {
                    //     //self.try_insert_characteristic(
                    //     // data_buffer,
                    //     // term,
                    //     // Characteristic::InverseFunctionalProperty)
                    //     // TODO: Implement
                    // }
                    // TODO owl::INVERSE_OF => {}
                    // owl::IRREFLEXIVE_PROPERTY => {}
                    // owl::MAX_CARDINALITY => {}
                    // owl::MAX_QUALIFIED_CARDINALITY => {}
                    // owl::MEMBERS => {}
                    // owl::MIN_CARDINALITY => {}
                    // owl::MIN_QUALIFIED_CARDINALITY => {}
                    // owl::NAMED_INDIVIDUAL => {}
                    // owl::NEGATIVE_PROPERTY_ASSERTION => {}
                    // TODO owl::NOTHING => {}
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
                    // owl::ONTOLOGY_PROPERTY => {}
                    // owl::ON_CLASS => {}
                    // owl::ON_DATARANGE => {}
                    // owl::ON_DATATYPE => {}
                    // owl::ON_PROPERTIES => {}
                    // owl::ON_PROPERTY => {}
                    // owl::PRIOR_VERSION => {}
                    // owl::PROPERTY_CHAIN_AXIOM => {}
                    // owl::PROPERTY_DISJOINT_WITH => {}
                    // owl::QUALIFIED_CARDINALITY => {}
                    // owl::REFLEXIVE_PROPERTY => {}
                    // owl::RESTRICTION => {}
                    // owl::SAME_AS => {}
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
                    // TODO owl::TRANSITIVE_PROPERTY => {
                    //     match self.insert_edge(data_buffer, &triple, Some(ElementType::NoDraw)) {
                    //         Some(edge) => {
                    //             data_buffer
                    //                 .edge_characteristics
                    //                 .insert(edge, vec![Characteristic::Transitive.to_string()]);
                    //         }
                    //         _ => {}
                    //     }
                    // }
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
