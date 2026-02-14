//! Provides SPARQL query snippets for generic querying across vocabularies.

/// Flatten collections. Currently only supports select OWL types.
pub const COLLECTIONS: &str = r#"{
            ?id ?nodeType ?intermediate .
            ?intermediate rdf:first ?firstItem .
            ?intermediate rdf:rest*/rdf:first ?target .
            FILTER(?nodeType IN (
                owl:intersectionOf, 
                owl:unionOf, 
                owl:oneOf,
                owl:disjointUnionOf
            ))

            # 6. Safety: Remove nil to avoid phantom edges
            # FILTER(?label != rdf:nil)
            }"#;

/// External classes.
///
/// 1. Elements whose base URI differs from that of the visualized ontology.
///    p. 6 of https://www.semantic-web-journal.net/system/files/swj1114.pdf
/// 2. A base URI is EITHER `xml:base` OR that of the document.
///    https://www.w3.org/TR/rdf-syntax-grammar/#section-Syntax-ID-xml-base
pub const XML_BASE: &str = r#"{
            ?id xml:base ?base .
            BIND(xml:base AS ?nodeType)
            }"#;

/// External classes.
///
/// Get the base URI of the document.
pub const ONTOLOGY: &str = r#"{
            ?id a owl:Ontology .
            BIND(owl:Ontology AS ?nodeType)
            }"#;

/// Generic, deprecated OWL elements.
///
/// This query is still work-in-progress.
/// We need to figure out what type the deprecated element is.
/// It could be a class or a property!
pub const OWL_DEPRECATED: &str = r#"{
            # WIP: Identify OWL deprecated properties
            # ?id owl:deprecated "true"^^<http://www.w3.org/2001/XMLSchema#boolean>
            # BIND("DeprecatedProperty" AS ?nodeType)
        
            # DEPRECATED
            ?id owl:deprecated ?target .
            BIND(owl:deprecated AS ?nodeType)
            }"#;

/// Find labels for elements in the following order:
/// 1. Use rdfs:label, if exists.
///    https://www.w3.org/TR/rdf-schema/#ch_label
/// 2. Use rdf:resource, if exists.
///    https://www.w3.org/TR/rdf-syntax-grammar/#section-Syntax-empty-property-elements
/// 3. Use rdf:ID, if exists.
///    https://www.w3.org/TR/rdf-syntax-grammar/#section-Syntax-ID-xml-base
pub const LABEL: &str = r#"{
            OPTIONAL { ?id rdfs:label ?theLabel }
            OPTIONAL { ?id rdf:resource ?resLabel }
            OPTIONAL { ?id rdf:ID ?idLabel }
            BIND (
                COALESCE(
                    IF( BOUND(?theLabel), ?theLabel, 1/0 ),
                    IF( BOUND(?resLabel), ?resLabel, 1/0 ),
                    IF( BOUND(?idLabel), ?idLabel, 1/0 ),
                    ""
                ) AS ?label
            )
            }"#;

/// Find the domain and range of any property.
pub const DOMAIN_AND_RANGE: &str = r#"{
            {
                # Domain
                ?id rdfs:domain ?target
                BIND(rdfs:domain AS ?nodeType)
            }
            UNION
            {
                # Range
                ?id rdfs:range ?target
                BIND(rdfs:range AS ?nodeType)
            }
            }"#;
pub const DOMAIN_RANGES: &str = r#"{
        {
            VALUES ?property {
                owl:DeprecatedProperty
                owl:DatatypeProperty
                owl:ObjectProperty
                rdf:Property
            }
            ?nodeType a ?property .
            ?nodeType rdfs:range ?id .
            ?nodeType rdfs:domain ?target .
        } UNION {
            VALUES ?property {
                owl:DeprecatedProperty
                owl:DatatypeProperty
                owl:ObjectProperty
                rdf:Property
            }
            ?nodeType a ?property . 
            ?nodeType rdfs:range ?id 
            FILTER NOT EXISTS(?nodeType rdfs:domain ?id)
            BIND(?target AS owl:Thing)
        } UNION {
            VALUES ?property {
                owl:DeprecatedProperty
                owl:DatatypeProperty
                owl:ObjectProperty
                rdf:Property
            }
            ?nodeType a ?property .
            ?nodeType rdfs:domain ?id 
            FILTER NOT EXISTS(?nodeType rdfs:range ?id)
            BIND(?target AS owl:Thing)
        }
        
        }"#;
