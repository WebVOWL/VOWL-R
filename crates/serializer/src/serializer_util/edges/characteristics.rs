//! Funxtions related to characteristics.

use grapher::prelude::{Characteristic, ElementType, OwlEdge, OwlNode, OwlType};
use log::{debug, warn};
use vowlgrapher_util::prelude::ErrorRecord;

use crate::{
    datastructures::{
        ArcTriple, SerializationStatus, serialization_data_buffer::SerializationDataBuffer,
    },
    errors::{SerializationError, SerializationErrorKind},
    serializer_util::{
        buffers::{
            add_term_to_element_buffer, add_to_unknown_buffer, check_unknown_buffer,
            insert_edge_include, remove_edge_include, resolve,
        },
        edges::merge_properties,
        entity_creation::{create_edge_from_id, get_or_create_anchor_thing},
        is_reserved,
        labels::merge_optional_labels,
    },
};

pub fn normalize_inverse_endpoint(
    data_buffer: &mut SerializationDataBuffer,
    endpoint_term_id: &usize,
    opposite_term_id: &usize,
) -> Result<usize, SerializationError> {
    let Some(element_type) = ({
        data_buffer
            .node_element_buffer
            .read()?
            .get(endpoint_term_id)
            .copied()
    }) else {
        return Ok(*endpoint_term_id);
    };

    match element_type {
        ElementType::Owl(OwlType::Node(
            OwlNode::Complement
            | OwlNode::IntersectionOf
            | OwlNode::UnionOf
            | OwlNode::DisjointUnion
            | OwlNode::EquivalentClass,
        )) => get_or_create_anchor_thing(data_buffer, opposite_term_id),
        _ => Ok(*endpoint_term_id),
    }
}

pub fn inverse_edge_endpoints(
    data_buffer: &mut SerializationDataBuffer,
    property_term_id: &usize,
) -> Result<Option<(usize, usize)>, SerializationError> {
    let domain = {
        data_buffer
            .property_domain_map
            .read()?
            .get(property_term_id)
            .and_then(|domains| domains.iter().next())
            .copied()
    };
    let range = {
        data_buffer
            .property_range_map
            .read()?
            .get(property_term_id)
            .and_then(|ranges| ranges.iter().next())
            .copied()
    };

    match (&domain, &range) {
        (Some(domain), Some(range)) => {
            let subject = normalize_inverse_endpoint(data_buffer, domain, range)?;
            let object = normalize_inverse_endpoint(data_buffer, range, domain)?;
            Ok(Some((subject, object)))
        }
        _ => Ok(None),
    }
}

pub fn insert_inverse_of(
    data_buffer: &mut SerializationDataBuffer,
    triple: ArcTriple,
) -> Result<SerializationStatus, SerializationError> {
    let left_property_raw = triple.subject_term_id;
    let Some(right_property_raw) = triple.object_term_id else {
        let msg = format!(
            "owl:inverseOf triple is missing a target: {}",
            data_buffer.term_index.display_triple(&triple)?
        );
        let e = SerializationErrorKind::SerializationWarning(msg.to_string());
        warn!("{msg}");
        data_buffer
            .failed_buffer
            .write()?
            .push(<SerializationError as Into<ErrorRecord>>::into(e.into()));

        return Ok(SerializationStatus::Serialized);
    };

    ensure_object_property_registration(data_buffer, left_property_raw)?;
    ensure_object_property_registration(data_buffer, right_property_raw)?;

    let Some(left_property) = resolve(data_buffer, left_property_raw)? else {
        add_to_unknown_buffer(data_buffer, left_property_raw, triple)?;
        return Ok(SerializationStatus::Deferred);
    };

    let Some(right_property) = resolve(data_buffer, right_property_raw)? else {
        add_to_unknown_buffer(data_buffer, right_property_raw, triple)?;
        return Ok(SerializationStatus::Deferred);
    };

    if left_property == right_property {
        return Ok(SerializationStatus::Serialized);
    }

    let (left_subject, left_object) = match inverse_edge_endpoints(data_buffer, &left_property)? {
        Some(endpoints) => endpoints,
        None => {
            add_to_unknown_buffer(data_buffer, left_property, triple)?;
            return Ok(SerializationStatus::Deferred);
        }
    };

    let (right_subject, right_object) = match inverse_edge_endpoints(data_buffer, &right_property)?
    {
        Some(endpoints) => endpoints,
        None => {
            add_to_unknown_buffer(data_buffer, right_property, triple)?;
            return Ok(SerializationStatus::Deferred);
        }
    };

    let compatible = left_subject == right_object && left_object == right_subject;
    if !compatible {
        let msg = format!(
            "Cannot merge owl:inverseOf '{}'<->'{}': normalized edges do not align ({} -> {}, {} -> {})",
            data_buffer.term_index.get(&left_property)?,
            data_buffer.term_index.get(&right_property)?,
            data_buffer.term_index.get(&left_subject)?,
            data_buffer.term_index.get(&left_object)?,
            data_buffer.term_index.get(&right_subject)?,
            data_buffer.term_index.get(&right_object)?
        );
        let e = SerializationErrorKind::SerializationWarning(msg.to_string());
        warn!("{msg}");
        data_buffer
            .failed_buffer
            .write()?
            .push(<SerializationError as Into<ErrorRecord>>::into(e.into()));

        return Ok(SerializationStatus::Serialized);
    }

    let (merged_label, merged_characteristics) = {
        let left_edge = {
            data_buffer
                .property_edge_map
                .read()?
                .get(&left_property)
                .cloned()
        };
        let right_edge = {
            data_buffer
                .property_edge_map
                .read()?
                .get(&right_property)
                .cloned()
        };

        let merged_label = {
            let edge_label_buffer = data_buffer.edge_label_buffer.read()?;
            let label_buffer = data_buffer.label_buffer.read()?;
            let left_label = left_edge
                .as_ref()
                .and_then(|edge| edge_label_buffer.get(edge))
                .or_else(|| label_buffer.get(&left_property));

            let right_label = right_edge
                .as_ref()
                .and_then(|edge| edge_label_buffer.get(edge))
                .or_else(|| label_buffer.get(&right_property));

            merge_optional_labels(left_label, right_label)
        };

        merge_properties(data_buffer, &right_property, &left_property)?;

        if let Some(ref left_edge) = left_edge {
            remove_edge_include(data_buffer, &left_edge.domain_term_id, left_edge)?;
            remove_edge_include(data_buffer, &left_edge.range_term_id, left_edge)?;
            data_buffer.edge_buffer.write()?.remove(left_edge);
            data_buffer.edge_label_buffer.write()?.remove(left_edge);
        }

        if let Some(ref right_edge) = right_edge {
            remove_edge_include(data_buffer, &right_edge.domain_term_id, right_edge)?;
            remove_edge_include(data_buffer, &right_edge.range_term_id, right_edge)?;
            data_buffer.edge_buffer.write()?.remove(right_edge);
            data_buffer.edge_label_buffer.write()?.remove(right_edge);
        }

        let merged_characteristics = {
            let mut edge_characteristics = data_buffer.edge_characteristics.write()?;
            let mut merged_characteristics = left_edge
                .and_then(|edge| edge_characteristics.remove(&edge))
                .unwrap_or_default();

            if let Some(right_characteristics) =
                right_edge.and_then(|edge| edge_characteristics.remove(&edge))
            {
                merged_characteristics.extend(right_characteristics);
            }
            merged_characteristics
        };
        (merged_label, merged_characteristics)
    };

    let inverse_property = Some(left_property);
    let edge_type = ElementType::Owl(OwlType::Edge(OwlEdge::InverseOf));
    let inverse_edges = [
        create_edge_from_id(
            &data_buffer.term_index,
            left_subject,
            edge_type,
            left_object,
            inverse_property,
        )?,
        create_edge_from_id(
            &data_buffer.term_index,
            left_object,
            edge_type,
            left_subject,
            inverse_property,
        )?,
    ];

    let canonical_edge = inverse_edges[0].clone();

    for edge in inverse_edges {
        {
            data_buffer.edge_buffer.write()?.insert(edge.clone());
        }
        insert_edge_include(data_buffer, edge.domain_term_id, edge.clone())?;
        insert_edge_include(data_buffer, edge.range_term_id, edge.clone())?;
        if let Some(ref label) = merged_label {
            data_buffer
                .edge_label_buffer
                .write()?
                .insert(edge.clone(), label.clone());
        }

        if !merged_characteristics.is_empty() {
            data_buffer
                .edge_characteristics
                .write()?
                .insert(edge, merged_characteristics.clone());
        }
    }

    let mut property_edge_map = data_buffer.property_edge_map.write()?;
    property_edge_map.insert(left_property, canonical_edge);
    property_edge_map.remove(&right_property);

    Ok(SerializationStatus::Serialized)
}

pub fn ensure_object_property_registration(
    data_buffer: &mut SerializationDataBuffer,
    property_term_id: usize,
) -> Result<(), SerializationError> {
    let already_registered = {
        data_buffer
            .edge_element_buffer
            .read()?
            .contains_key(&property_term_id)
    };
    if already_registered {
        return Ok(());
    }

    let property_iri = data_buffer.term_index.get(&property_term_id)?;
    if is_reserved(&property_iri) {
        return Ok(());
    }

    add_term_to_element_buffer(
        &data_buffer.term_index,
        &mut data_buffer.edge_element_buffer,
        property_term_id,
        ElementType::Owl(OwlType::Edge(OwlEdge::ObjectProperty)),
    )?;

    check_unknown_buffer(data_buffer, &property_term_id)?;
    Ok(())
}

pub fn insert_characteristic(
    data_buffer: &mut SerializationDataBuffer,
    triple: ArcTriple,
    characteristic: Characteristic,
) -> Result<SerializationStatus, SerializationError> {
    match characteristic {
        Characteristic::AsymmetricProperty
        | Characteristic::InverseFunctionalProperty
        | Characteristic::IrreflexiveProperty
        | Characteristic::ReflexiveProperty
        | Characteristic::SymmetricProperty
        | Characteristic::TransitiveProperty => {
            ensure_object_property_registration(data_buffer, triple.subject_term_id)?;
        }
        Characteristic::FunctionalProperty | Characteristic::HasKey => {}
    }

    let Some(resolved_property_term_id) = resolve(data_buffer, triple.subject_term_id)? else {
        let property_iri = data_buffer.term_index.get(&triple.subject_term_id)?;
        if is_reserved(&property_iri) {
            debug!(
                "Skipping characteristic '{}' for reserved built-in '{}'",
                characteristic, property_iri
            );
            return Ok(SerializationStatus::Serialized);
        }

        debug!(
            "Deferring characteristic '{}' for '{}': property unresolved",
            characteristic,
            data_buffer.term_index.get(&triple.subject_term_id)?
        );
        add_to_unknown_buffer(data_buffer, triple.subject_term_id, triple)?;
        return Ok(SerializationStatus::Deferred);
    };

    // Characteristic can attach only after a concrete edge exists
    let maybe_edge = {
        data_buffer
            .property_edge_map
            .read()?
            .get(&resolved_property_term_id)
            .cloned()
    };
    if let Some(edge) = maybe_edge {
        debug!(
            "Inserting edge characteristic: {} -> {}",
            data_buffer.term_index.get(&resolved_property_term_id)?,
            characteristic
        );

        let target_edges = if edge.edge_type == ElementType::Owl(OwlType::Edge(OwlEdge::InverseOf))
        {
            data_buffer
                .edge_buffer
                .read()?
                .iter()
                .filter(|candidate| {
                    candidate.edge_type == ElementType::Owl(OwlType::Edge(OwlEdge::InverseOf))
                        && candidate.property_term_id.as_ref() == Some(&resolved_property_term_id)
                })
                .cloned()
                .collect()
        } else {
            vec![edge]
        };

        let mut edge_characteristics = data_buffer.edge_characteristics.write()?;
        for target_edge in target_edges {
            edge_characteristics
                .entry(target_edge)
                .or_default()
                .insert(characteristic);
        }
        return Ok(SerializationStatus::Serialized);
    }

    // Property is known, but edge not materialized yet
    let property_is_known = {
        data_buffer
            .edge_element_buffer
            .read()?
            .contains_key(&resolved_property_term_id)
    };
    if property_is_known {
        debug!(
            "Deferring characteristic '{}' for '{}': property known, edge not materialized yet",
            characteristic,
            data_buffer.term_index.get(&resolved_property_term_id)?
        );
        add_to_unknown_buffer(data_buffer, resolved_property_term_id, triple)?;
        return Ok(SerializationStatus::Deferred);
    }

    let resolved_iri = data_buffer.term_index.get(&resolved_property_term_id)?;
    if is_reserved(&resolved_iri) {
        debug!(
            "Skipping characteristic '{}' for reserved built-in '{}'",
            characteristic, resolved_iri
        );
        return Ok(SerializationStatus::Serialized);
    }

    // No attach point yet
    debug!(
        "Deferring characteristic '{}' for '{}': no attach point available yet",
        characteristic,
        data_buffer.term_index.get(&resolved_property_term_id)?
    );
    add_to_unknown_buffer(data_buffer, resolved_property_term_id, triple)?;
    Ok(SerializationStatus::Deferred)
}
