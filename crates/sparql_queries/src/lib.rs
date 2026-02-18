//! Shared SPARQL query strings for VOWL-R.
//!
//! This crate is intentionally dependency-free and WASM-safe so it can be used by:
//! - the SSR/server side (via `vowlr-database`)
//! - the client/wasm side (via `vowlr`)

mod assembly;
mod snippets;

/// Exports all the core types of the library.
pub mod prelude {
    use grapher::prelude::{Characteristic, OwlEdge, OwlNode, RdfEdge, RdfsEdge, RdfsNode};
    use std::sync::LazyLock;

    use crate::assembly::DEFAULT_PREFIXES;
    pub use crate::assembly::QueryAssembler;
    use crate::snippets::general::{
        COLLECTIONS, DOMAIN_RANGES, LABEL, ONTOLOGY, OWL_DEPRECATED, XML_BASE,
    };
    use crate::snippets::snippets_from_enum;

    /// SPARQL snippets that should generally be included in all queries.
    pub static GENERAL_SNIPPETS: [&str; 6] = [
        ONTOLOGY,
        XML_BASE,
        COLLECTIONS,
        DOMAIN_RANGES,
        OWL_DEPRECATED,
        LABEL,
    ];

    // PERF: this could maybe be a thread_local instead?
    /// The default query contains all classes and properties supported by VOWL-R.
    pub static DEFAULT_QUERY: LazyLock<String> = LazyLock::new(|| {
        let snippets = [
            snippets_from_enum::<OwlNode>(),
            snippets_from_enum::<OwlEdge>(),
            snippets_from_enum::<RdfEdge>(),
            snippets_from_enum::<RdfsNode>(),
            snippets_from_enum::<RdfsEdge>(),
            snippets_from_enum::<Characteristic>(),
            GENERAL_SNIPPETS.into(),
        ]
        .concat();

        QueryAssembler::assemble_query(DEFAULT_PREFIXES.into(), snippets)
    });
}
