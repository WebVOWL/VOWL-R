use futures::{StreamExt, stream::BoxStream};
use log::{info, warn};
use rdf_fusion::store::Store;
use std::path::Path;
use std::time::Duration;
use std::{fs::File, time::Instant};

use vowlr_parser::{
    errors::VOWLRStoreError,
    parser_util::{parse_stream_to, parser_from_format},
};
use vowlr_util::datatypes::DataType;

static GLOBAL_STORE: std::sync::OnceLock<Store> = std::sync::OnceLock::new();

pub struct VOWLRStore {
    pub session: Store,
    upload_handle: Option<tempfile::NamedTempFile>,
}
impl VOWLRStore {
    pub fn new(session: Store) -> Self {
        Self {
            session,
            upload_handle: None,
        }
    }

    pub fn default() -> Self {
        let session = GLOBAL_STORE.get_or_init(Store::default).clone();
        Self {
            session,
            upload_handle: None,
        }
    }

    // TTL format -> (oxittl) RDF XML quads -> (horned_owl) Normalize OWL/RDF -> Quads -> Insert into Oxigraph
    pub async fn insert_file(&self, fs: &Path, lenient: bool) -> Result<(), VOWLRStoreError> {
        let parser = parser_from_format(fs, lenient)?;
        info!("Loading input into database...");
        let start_time = Instant::now();
        self.session
            .load_from_reader(parser.parser, parser.input.as_slice())
            .await?;
        info!(
            "Loaded {} quads in {} s",
            self.session.len().await.unwrap(),
            Instant::now()
                .checked_duration_since(start_time)
                .unwrap_or(Duration::new(0, 0))
                .as_secs_f32()
        );
        Ok(())
    }

    pub async fn serialize_to_file(&self, path: &Path) -> Result<(), VOWLRStoreError> {
        let mut file = File::create(path)?;
        let mut results = parse_stream_to(self.session.stream().await?, DataType::OWL).await?;
        while let Some(result) = results.next().await {
            let result = result.unwrap();
            std::io::Write::write_all(&mut file, &result)?;
        }

        Ok(())
    }

    pub async fn serialize_stream(
        &self,
        resource_type: DataType,
    ) -> Result<BoxStream<'static, Result<Vec<u8>, VOWLRStoreError>>, VOWLRStoreError> {
        info!(
            "Store size before export: {}",
            self.session.len().await.unwrap_or(0)
        );
        let results = parse_stream_to(self.session.stream().await?, resource_type).await?;
        Ok(results)
    }

    pub async fn start_upload(&mut self, filename: &str) -> Result<(), VOWLRStoreError> {
        let extension = Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("owl");
        let file = tempfile::Builder::new()
            .suffix(&format!(".{}", extension))
            .tempfile()?;
        self.upload_handle = Some(file);
        Ok(())
    }

    pub async fn upload_chunk(&mut self, data: &[u8]) -> Result<(), VOWLRStoreError> {
        if let Some(file) = &mut self.upload_handle {
            std::io::Write::write_all(file, data)?;
            Ok(())
        } else {
            warn!("upload_chunk called without start_upload");
            Ok(())
        }
    }

    pub async fn complete_upload(&mut self) -> Result<(), VOWLRStoreError> {
        if let Some(file) = &mut self.upload_handle {
            std::io::Write::flush(file)?;
            let path = file.path();
            let parser = parser_from_format(path, false)?;

            info!("Loading input into database...");
            let start_time = Instant::now();
            self.session
                .load_from_reader(parser.parser, parser.input.as_slice())
                .await?;
            info!(
                "Loaded {} quads in {} s",
                self.session.len().await.unwrap(),
                Instant::now()
                    .checked_duration_since(start_time)
                    .unwrap_or(Duration::new(0, 0))
                    .as_secs_f32()
            );
        }
        self.upload_handle = None;
        Ok(())
    }
}

pub const DEFAULT_QUERY_1: &str = r#"
    PREFIX owl: <http://www.w3.org/2002/07/owl#>
    PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
    PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
    PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>
    PREFIX vowlr: <http://www.example.com/iri#>

    SELECT ?id ?nodeType ?label
    WHERE {
        {
            # 1. Identify Named Classes
            ?id a owl:Class .
            FILTER(isIRI(?id))
            BIND(owl:Class AS ?nodeType)
            OPTIONAL { ?id rdfs:label ?label }
        }
        UNION
        {
            ?id rdf:type owl:Ontology .
            BIND(owl:Ontology AS ?nodeType)
        }
        UNION
        {
            ?id rdf:type owl:ObjectProperty .
            BIND(owl:ObjectProperty AS ?nodeType)
        }
        UNION
        {
            ?id rdfs:domain ?label .
            BIND(rdfs:domain AS ?nodeType)
        }
        UNION
        {
            ?id rdfs:range ?label .
            BIND(rdfs:range AS ?nodeType)
        }
        UNION
        {
            # 2. Identify Intersections
            # Any node (usually blank) that is the subject of an intersectionOf list
            ?id owl:intersectionOf ?label .
            BIND(owl:intersectionOf AS ?nodeType)
        }
        UNION
        {
            # 3. Identify Unions
            ?id owl:unionOf ?label .
            BIND(owl:unionOf AS ?nodeType)
        }
        UNION
        {
            ?id owl:Restriction ?label .
            BIND(owl:Restriction AS ?nodeType)
        }
        UNION
        {
            ?id owl:equivalentClass ?label .
            BIND(owl:equivalentClass AS ?nodeType)
        }
        # Edges
        UNION
        {
            # 2. Identify subclasses
            ?id rdfs:subClassOf ?label .
            BIND(rdfs:subClassOf AS ?nodeType)
        }
        UNION {
            # 1. Find ANY subject/property that points to the start of a list
            ?subject ?nodeType ?id .

            # 2. Verify ?listHead is actually the start of a list (it must have an rdf:first)
            ?id rdf:first ?anyFirst .

            # 3. FLATTEN THE LIST
            # Traverse from the list head to find every member
            ?id rdf:rest*/rdf:first ?label .

            # 4. (Optional) Safety Filter
            # Ensure we only expand relevant OWL logic operators, not random RDF lists.
            FILTER(?property IN (
                owl:intersectionOf, 
                owl:unionOf, 
                owl:oneOf, 
                owl:disjointUnionOf
            ))
        }
        UNION
        {
            # 3. Identify datatypes
            ?id rdfs:datatype ?label .
            BIND(owl:datatype AS ?nodeType)
        }
        UNION
        {
            # 4. Identify OWL datatype properties
            ?id owl:DatatypeProperty ?label .
            BIND(owl:DatatypeProperty AS ?nodeType)
        }
        UNION
        {
            # 5. Identify OWL disjoint with
            ?id owl:disjointWith ?label
            BIND(owl:disjointWith AS ?nodeType)
        }
        UNION {
            # BRIDGE: Start at the Named Class, jump to the intermediate node
            ?id ?connector ?intermediateNode .
            FILTER(isIRI(?id)) 

            # Match the logic property (unionOf, etc) on the intermediate node
            ?intermediateNode ?nodeType ?blanknode .

            # Flatten the list from the blanknode
            ?blanknode rdf:rest*/rdf:first ?label .

            # Filter for Logic Types
            # FILTER(?nodeType IN (owl:intersectionOf, owl:unionOf, owl:oneOf))
            FILTER(?label != rdf:nil)
        }
        UNION
        {
            # 6. WIP Identify OWL deprecated properties
            ?id owl:deprecated "true"^^<http://www.w3.org/2001/XMLSchema#boolean>
            BIND("DeprecatedProperty" AS ?nodeType)
        }
        BIND(
            IF(?nodeType = owl:Class, 1, 2)
            AS ?weight)
    }
    ORDER BY ?weight
    "#;

#[cfg(test)]
#[allow(unused_must_use)]
mod test {
    use super::*;
    use test_generator::test_resources;

    #[test_resources("crates/database/data/owl-functional/*.ofn")]
    async fn test_ofn_parser_format(resource: &str) -> Result<(), VOWLRStoreError> {
        let store = VOWLRStore::default();
        store
            .insert_file(Path::new(&resource), false)
            .await
            .unwrap();
        assert_ne!(
            store.session.len().await.unwrap(),
            0,
            "Expected non-zero quads for: {}",
            resource
        );
        store.session.clear().await?;
        Ok(())
    }
    #[test_resources("crates/database/data/owl-rdf/*.owl")]
    async fn test_owl_parser_format(resource: &str) -> Result<(), VOWLRStoreError> {
        let store = VOWLRStore::default();
        store
            .insert_file(Path::new(&resource), false)
            .await
            .unwrap();
        assert_ne!(
            store.session.len().await.unwrap(),
            0,
            "Expected non-zero quads for: {}",
            resource
        );
        store.session.clear().await?;
        Ok(())
    }
    #[test_resources("crates/database/data/owl-ttl/*.ttl")]
    async fn test_ttl_parser_format(resource: &str) -> Result<(), VOWLRStoreError> {
        let store = VOWLRStore::default();
        store
            .insert_file(Path::new(&resource), false)
            .await
            .unwrap();
        assert_ne!(
            store.session.len().await.unwrap(),
            0,
            "Expected non-zero quads for: {}",
            resource
        );
        store.session.clear().await?;
        Ok(())
    }
    #[test_resources("crates/database/data/owl-xml/*.owx")]
    async fn test_owx_parser_format(resource: &str) -> Result<(), VOWLRStoreError> {
        let store = VOWLRStore::default();
        store
            .insert_file(Path::new(&resource), false)
            .await
            .unwrap();
        assert_ne!(
            store.session.len().await.unwrap(),
            0,
            "Expected non-zero quads for: {}",
            resource
        );
        store.session.clear().await?;
        Ok(())
    }

    #[test_resources("crates/database/data/owl-functional/*.ofn")]
    async fn test_ofn_parser_stream(resource: &str) -> Result<(), VOWLRStoreError> {
        let mut out = vec![];
        let store = VOWLRStore::default();
        store.insert_file(Path::new(&resource), false).await?;
        let mut results = parse_stream_to(store.session.stream().await?, DataType::OWL).await?;
        while let Some(result) = results.next().await {
            out.extend(result?);
        }

        assert_ne!(out.len(), 0, "Expected non-zero quads for: {}", resource);
        store.session.clear().await?;
        Ok(())
    }
    #[test_resources("crates/database/data/owl-rdf/*.owl")]
    async fn test_owl_parser_stream(resource: &str) -> Result<(), VOWLRStoreError> {
        let mut out = vec![];
        let store = VOWLRStore::default();
        store.insert_file(Path::new(&resource), false).await?;
        let mut results = parse_stream_to(store.session.stream().await?, DataType::OWL).await?;
        while let Some(result) = results.next().await {
            out.extend(result?);
        }

        assert_ne!(out.len(), 0, "Expected non-zero quads for: {}", resource);
        store.session.clear().await?;
        Ok(())
    }
    #[test_resources("crates/database/data/owl-ttl/*.ttl")]
    async fn test_ttl_parser_stream(resource: &str) -> Result<(), VOWLRStoreError> {
        let mut out = vec![];
        let store = VOWLRStore::default();
        store.insert_file(Path::new(&resource), false).await?;
        let mut results = parse_stream_to(store.session.stream().await?, DataType::OWL).await?;
        while let Some(result) = results.next().await {
            out.extend(result?);
        }

        assert_ne!(out.len(), 0, "Expected non-zero quads for: {}", resource);
        store.session.clear().await?;
        Ok(())
    }
    #[test_resources("crates/database/data/owl-xml/*.owx")]
    async fn test_owx_parser_stream(resource: &str) -> Result<(), VOWLRStoreError> {
        let mut out = vec![];
        let store = VOWLRStore::default();
        store.insert_file(Path::new(&resource), false).await?;
        let mut results = parse_stream_to(store.session.stream().await?, DataType::OWL).await?;
        while let Some(result) = results.next().await {
            out.extend(result?);
        }

        assert_ne!(out.len(), 0, "Expected non-zero quads for: {}", resource);
        store.session.clear().await?;
        Ok(())
    }
}
