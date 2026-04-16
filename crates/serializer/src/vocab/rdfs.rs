//! [RDFS](https://www.w3.org/TR/rdf-schema/) vocabulary.
use oxrdf::NamedNodeRef;

/// The class of classes.
pub const CLASS: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/2000/01/rdf-schema#Class");
/// A description of the subject resource.
pub const COMMENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/2000/01/rdf-schema#comment");
/// The class of RDF containers.
pub const CONTAINER: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/2000/01/rdf-schema#Container");
/// The class of container membership properties, `rdf:_1`, `rdf:_2`, ..., all of which are sub-properties of `member`.
pub const CONTAINER_MEMBERSHIP_PROPERTY: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/2000/01/rdf-schema#ContainerMembershipProperty");
/// The class of RDF datatypes.
pub const DATATYPE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/2000/01/rdf-schema#Datatype");
/// A domain of the subject property.
pub const DOMAIN: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/2000/01/rdf-schema#domain");
/// The definition of the subject resource.
pub const IS_DEFINED_BY: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/2000/01/rdf-schema#isDefinedBy");
/// A human-readable name for the subject.
pub const LABEL: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/2000/01/rdf-schema#label");
/// The class of literal values, e.g. textual strings and integers.
pub const LITERAL: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/2000/01/rdf-schema#Literal");
/// A member of the subject resource.
pub const MEMBER: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/2000/01/rdf-schema#member");
/// A range of the subject property.
pub const RANGE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/2000/01/rdf-schema#range");
/// The class resource, everything.
pub const RESOURCE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/2000/01/rdf-schema#Resource");
/// Further information about the subject resource.
pub const SEE_ALSO: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/2000/01/rdf-schema#seeAlso");
/// The subject is a subclass of a class.
pub const SUB_CLASS_OF: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/2000/01/rdf-schema#subClassOf");
/// The subject is a subproperty of a property.
pub const SUB_PROPERTY_OF: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/2000/01/rdf-schema#subPropertyOf");
