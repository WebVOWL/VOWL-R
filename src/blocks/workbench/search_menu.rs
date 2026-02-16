use super::WorkbenchMenuItems;
use grapher::prelude::ElementType;
use leptos::prelude::*;
use strum::IntoEnumIterator;

fn format_node_type_name(node_type_str: &str) -> String {
    let mut result = String::new();

    for (i, ch) in node_type_str.chars().enumerate() {
        if i > 0 && ch.is_uppercase() {
            result.push(' ');
        }
        result.push(ch);
    }

    result
}

fn get_mock_instances(node_type: NodeType) -> Vec<String> {
    match node_type {
        NodeType::Class => vec![
            "Person",
            "Organization",
            "Document",
            "Project",
            "Agent",
            "Event",
            "Place",
            "Activity",
            "Building",
            "Company",
            "Department",
            "Team",
            "User",
            "Role",
            "Group",
            "Community",
            "Service",
            "Resource",
            "Asset",
            "Product",
            "Category",
            "Topic",
            "Concept",
            "Entity",
            "Thing",
            "Subject",
            "Object",
            "Artifact",
            "Work",
            "Publication",
            "CreativeWork",
            "Software",
            "Application",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        NodeType::ExternalClass => vec![
            "ExternalEntity",
            "RemoteResource",
            "ForeignClass",
            "ImportedThing",
            "LinkedEntity",
            "ExternalReference",
            "OutsideObject",
            "ForeignConcept",
            "ExternalDefinition",
            "RemoteClass",
            "WebResource",
            "ExternalService",
            "LinkedData",
            "FederatedEntity",
            "ExternalSchema",
            "RemoteOntology",
            "ExternalType",
            "ImportedClass",
            "ExternalConcept",
            "RemoteThing",
            "VocabularyTerm",
            "ExternalModel",
            "LinkedClass",
            "ExternalArtifact",
            "ForeignResource",
            "RemoteDefinition",
            "ExternalMapping",
            "PeerResource",
            "DistributedEntity",
            "FederatedClass",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        NodeType::Thing => vec![
            "Entity",
            "Object",
            "Item",
            "Element",
            "Instance",
            "Value",
            "Attribute",
            "Property",
            "Characteristic",
            "Feature",
            "Aspect",
            "Component",
            "Part",
            "Whole",
            "Collection",
            "Set",
            "Group",
            "Aggregate",
            "Unit",
            "Member",
            "Participant",
            "Container",
            "Holder",
            "Owner",
            "Subject",
            "Predicate",
            "Target",
            "Relation",
            "Connection",
            "Link",
            "Reference",
            "Definition",
            "Representation",
            "Abstraction",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        NodeType::EquivalentClass => vec![
            "SameThing",
            "Alias",
            "Synonym",
            "Equivalent",
            "Parallel",
            "Mirror",
            "Counterpart",
            "Duplicate",
            "Copy",
            "Alternative",
            "Variant",
            "Version",
            "Equivalent",
            "Identical",
            "Matching",
            "Corresponding",
            "Analogous",
            "Similar",
            "Comparable",
            "Equal",
            "Same",
            "Identical",
            "Equivalent",
            "Congruent",
            "Compatible",
            "Isomorphic",
            "Homomorphic",
            "Parallel",
            "Equivalent",
            "Corresponding",
            "Analogous",
            "Equivalent",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        NodeType::Union => vec![
            "CombinedClass",
            "UnionType",
            "OrClass",
            "MultiClass",
            "Combination",
            "Composite",
            "Compound",
            "Aggregate",
            "Collection",
            "SetUnion",
            "Merger",
            "Blend",
            "Mix",
            "Joined",
            "Linked",
            "Combined",
            "Unified",
            "Integrated",
            "Synthesized",
            "Merged",
            "Pooled",
            "Collected",
            "Bundled",
            "Grouped",
            "Assembled",
            "Gathered",
            "Accumulated",
            "Compiled",
            "Aggregated",
            "Consolidated",
            "Unified",
            "Composite",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        NodeType::DisjointUnion => vec![
            "ExclusiveClass",
            "SeparateUnion",
            "DiscreteSet",
            "PartitionedClass",
            "Exclusive",
            "Disjoint",
            "NonOverlapping",
            "Distinct",
            "Separate",
            "Individual",
            "Independent",
            "Autonomous",
            "Isolated",
            "Partitioned",
            "Segregated",
            "Decomposed",
            "Divided",
            "Split",
            "Separated",
            "Distinguished",
            "Differentiated",
            "Unique",
            "Singular",
            "Exclusive",
            "Mutual",
            "Exclusive",
            "Alternative",
            "Either",
            "Neither",
            "One",
            "Partition",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        NodeType::Intersection => vec![
            "SharedClass",
            "CommonGround",
            "Overlap",
            "IntersectionType",
            "Shared",
            "Common",
            "Joint",
            "Mutual",
            "Intersecting",
            "Overlapping",
            "Convergent",
            "Meeting",
            "Touching",
            "Crossing",
            "Intersecting",
            "Common",
            "Shared",
            "Mutual",
            "Reciprocal",
            "Collective",
            "Combined",
            "Unified",
            "Integrated",
            "Coherent",
            "Consistent",
            "Aligned",
            "Congruent",
            "Matching",
            "Corresponding",
            "Correlating",
            "Associated",
            "Connected",
            "Linked",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        NodeType::Complement => vec![
            "InverseClass",
            "NegatedClass",
            "OppositeType",
            "Complement",
            "Opposite",
            "Inverse",
            "Negation",
            "Contrary",
            "Antonym",
            "Reverse",
            "Mirror",
            "Flip",
            "Invert",
            "Negate",
            "Exclude",
            "Exclude",
            "Exclude",
            "Negated",
            "NotIncluded",
            "Missing",
            "Absent",
            "Omitted",
            "Excluded",
            "Expelled",
            "Removed",
            "Denied",
            "Rejected",
            "Refused",
            "Contradicted",
            "Opposed",
            "Conflicting",
            "Incompatible",
            "Inconsistent",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        NodeType::DeprecatedClass => vec![
            "OldClass",
            "ArchivedClass",
            "ObsoleteType",
            "LegacyClass",
            "OutdatedThing",
            "RetiredClass",
            "FormerClass",
            "PreviousClass",
            "PriorClass",
            "AncientClass",
            "HistoricalClass",
            "VintageClass",
            "CeaseClass",
            "AbandondClass",
            "ForgottenClass",
            "ReplacedClass",
            "SupersededClass",
            "OutmodeClass",
            "DiscontinuedClass",
            "WithdrawnClass",
            "ClosedClass",
            "InactiveClass",
            "DefunctClass",
            "ExtinctClass",
            "DisusedClass",
            "ArchaicClass",
            "ObsoleteClass",
            "VoidClass",
            "NullClass",
            "EmptyClass",
            "NoneClass",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        NodeType::AnonymousClass => vec![
            "UnnamedClass",
            "BlankClass",
            "AnonType",
            "NoNameClass",
            "IncognitoClass",
            "MysteryClass",
            "UnknownClass",
            "IdentifierlessClass",
            "GenericClass",
            "AbstractClass",
            "VagueClass",
            "IndefiniteClass",
            "ImplicitClass",
            "HiddenClass",
            "ConcealedClass",
            "PrivateClass",
            "SecretClass",
            "SilentClass",
            "QuietClass",
            "ReservedClass",
            "RestrictedClass",
            "LockedClass",
            "SealedClass",
            "ClosedClass",
            "InvisibleClass",
            "TransparentClass",
            "ShadowClass",
            "SpecterClass",
            "PhantomClass",
            "GhostClass",
            "NullClass",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        NodeType::Literal => vec![
            "StringValue",
            "IntegerValue",
            "FloatValue",
            "BooleanValue",
            "DateValue",
            "TimeValue",
            "DateTimeValue",
            "TextContent",
            "NumericContent",
            "ByteContent",
            "BitContent",
            "HexValue",
            "OctalValue",
            "BinaryValue",
            "URIValue",
            "URLValue",
            "EmailValue",
            "PhoneValue",
            "AddressValue",
            "GeometryValue",
            "PointValue",
            "PolygonValue",
            "ColorValue",
            "FontValue",
            "ImageValue",
            "AudioValue",
            "VideoValue",
            "MediaValue",
            "FileValue",
            "PathValue",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        NodeType::RdfsClass => vec![
            "RdfResource",
            "RdfType",
            "RdfProperty",
            "RdfsResource",
            "RdfsLiteral",
            "RdfStatement",
            "RdfBag",
            "RdfSeq",
            "RdfAlt",
            "RdfList",
            "RdfContainer",
            "RdfValue",
            "RdfFirstItem",
            "RdfRest",
            "RdfNil",
            "RdfSubject",
            "RdfPredicate",
            "RdfObject",
            "RdfTriple",
            "RdfQuad",
            "RdfGraph",
            "RdfDataset",
            "RdfNamedGraph",
            "RdfBlankNode",
            "RdfIRI",
            "RdfURI",
            "RdfURI",
            "RdfNode",
            "RdfTerm",
            "RdfEntity",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        NodeType::RdfsResource => vec![
            "Resource",
            "Element",
            "Item",
            "Component",
            "Asset",
            "Artifact",
            "Construct",
            "Building",
            "Structure",
            "Framework",
            "Infrastructure",
            "Foundation",
            "Base",
            "Core",
            "Kernel",
            "Entity",
            "Being",
            "Existence",
            "Instance",
            "Member",
            "Participant",
            "Agent",
            "Actor",
            "Thing",
            "Object",
            "Subject",
            "Target",
            "Container",
            "Holder",
            "Storage",
            "Repository",
            "Archive",
            "Collection",
            "Accumulation",
            "Set",
            "Group",
            "Cluster",
            "Category",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        _ => vec![],
    }
}

#[component]
pub fn SearchMenu() -> impl IntoView {
    let search_query = RwSignal::new(String::new());
    let expanded_category = RwSignal::new(Option::<String>::None);
    let filtered_results = Memo::new(move |_| {
        let query = search_query.get().to_lowercase();

        if query.is_empty() {
            return vec![];
        }

        NodeType::iter()
            .filter_map(|node_type| {
                let category_name = format!("{:?}", node_type);
                let instances = get_mock_instances(node_type);

                let matches: Vec<String> = instances
                    .into_iter()
                    .filter(|inst| inst.to_lowercase().starts_with(&query))
                    .collect();

                if !matches.is_empty() {
                    Some((category_name, matches))
                } else {
                    None
                }
            })
            .collect::<Vec<(String, Vec<String>)>>()
    });

    view! {
        <WorkbenchMenuItems title="Search">
            <div class="flex relative flex-col gap-2">
                <div class="w-full">
                    <input
                        type="text"
                        placeholder="Search (try 'Flip', 'Node', or 'has')..."
                        class="py-2 px-3 w-full rounded-md border border-gray-300 focus:ring-2 focus:ring-blue-500 focus:outline-none"
                        prop:value=move || search_query.get()
                        on:input=move |ev| {
                            let value = event_target_value(&ev);
                            search_query.set(value);
                        }
                    />
                </div>

                <Show
                    when=move || !filtered_results.get().is_empty()
                    fallback=|| view! { <div></div> }
                >
                    <div class="overflow-y-auto absolute top-12 z-50 w-full bg-white rounded-lg">
                        <div class="flex flex-col divide-y divide-gray-200">
                            <For
                                each=move || filtered_results.get()
                                key=|(category_name, instances)| {
                                    format!("{}-{}", category_name, instances.join(","))
                                }
                                children=move |(category_name, matching_instances)| {
                                    let stored_instances = StoredValue::new(matching_instances);
                                    let cat_name_for_click = category_name.clone();

                                    view! {
                                        <div>
                                            <div
                                                class="flex sticky top-0 z-10 justify-between items-center p-3 bg-white border-b border-gray-100 transition-colors cursor-pointer hover:bg-gray-100"
                                                on:click=move |_| {
                                                    let current = expanded_category.get();
                                                    if current == Some(cat_name_for_click.clone()) {
                                                        expanded_category.set(None);
                                                    } else {
                                                        expanded_category.set(Some(cat_name_for_click.clone()));
                                                    }
                                                }
                                            >
                                                <div class="flex gap-2 items-center">
                                                    <img
                                                        src=format!("/node_legends/{}.png", category_name)
                                                        alt=format!("{} icon", category_name)
                                                        class="object-contain w-8 h-8"
                                                    />
                                                    <h4 class="font-semibold text-gray-700">
                                                        {format_node_type_name(&category_name)}
                                                    </h4>
                                                </div>
                                                <span class="text-xs text-gray-400">
                                                    {move || {
                                                        format!(
                                                            "{} matches",
                                                            stored_instances.with_value(|v| v.len()),
                                                        )
                                                    }}
                                                </span>
                                            </div>

                                            <Show
                                                when=move || {
                                                    expanded_category.get() == Some(category_name.clone())
                                                }
                                                fallback=|| view! { <div></div> }
                                            >
                                                <div class="bg-gray-50 border-t border-gray-100">
                                                    <For
                                                        each=move || stored_instances.get_value()
                                                        key=|instance| instance.clone()
                                                        children=move |instance| {
                                                            view! {
                                                                <div class="p-2 pl-6 text-sm text-gray-600 cursor-pointer hover:text-blue-600 hover:bg-blue-50">
                                                                    {instance}
                                                                </div>
                                                            }
                                                        }
                                                    />
                                                </div>
                                            </Show>
                                        </div>
                                    }
                                }
                            />
                        </div>
                    </div>
                </Show>
            </div>
        </WorkbenchMenuItems>
    }
}
