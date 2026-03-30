use futures::{StreamExt, stream::BoxStream};
use grapher::prelude::GraphDisplayData;
use log::{debug, info, warn};
use rdf_fusion::execution::results::QueryResults;
use rdf_fusion::store::Store;
use std::path::Path;
use std::time::Duration;
use std::{fs::File, time::Instant};

use vowlr_parser::{
    errors::{VOWLRStoreError, VOWLRStoreErrorKind},
    parser_util::{parse_stream_to, parser_from_path, path_type},
};
use vowlr_util::prelude::{DataType, VOWLRError};

use crate::errors::SerializationErrorKind;
use crate::serializers::frontend::GraphDisplayDataSolutionSerializer;

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

    /// Executes a SPARQL query and serializes the result.
    ///
    /// This method tries to continue serializing despite errors.
    /// As such, the `Ok` value contains non-fatal errors encountered during serialization.
    pub async fn query(
        &self,
        query: String,
    ) -> Result<(GraphDisplayData, Option<VOWLRError>), VOWLRError> {
        let solution_serializer = GraphDisplayDataSolutionSerializer::new();
        let query_stream = self
            .session
            .query(query.as_str())
            .await
            .map_err(|e| <VOWLRStoreError as Into<VOWLRError>>::into(e.into()))?;

        match query_stream {
            QueryResults::Solutions(query_solution_stream) => {
                let mut data_buffer = GraphDisplayData::new();

                let maybe_errors = solution_serializer
                    .serialize_nodes_stream(&mut data_buffer, query_solution_stream)
                    .await?;
                Ok((data_buffer, maybe_errors))
            }
            QueryResults::Boolean(_result) => Err(SerializationErrorKind::UnsupportedQueryType(
                "Query stream is not a SELECT query".to_string(),
            )
            .into()),
            QueryResults::Graph(_query_triple_stream) => {
                // TODO: Implement to support user-defined SPARQL queries
                Err(SerializationErrorKind::UnsupportedQueryType(
                    "Query stream is not a SELECT query".to_string(),
                )
                .into())
            }
        }
    }

    // TTL format -> (oxittl) RDF XML quads -> (horned_owl) Normalize OWL/RDF -> Quads -> Insert into Oxigraph
    pub async fn insert_file(&self, fs: &Path, lenient: bool) -> Result<(), VOWLRStoreError> {
        let format = path_type(fs)
            .ok_or_else(|| VOWLRStoreErrorKind::InvalidFileType("Unknown file extension".into()))?;
        let parser = parser_from_path(fs, format, lenient)?;
        info!("Loading input into database...");
        let start_time = Instant::now();
        self.session
            .load_from_reader(parser.parser, parser.input.as_slice())
            .await?;
        info!(
            "Loaded {} quads in {} s",
            self.session.len().await?,
            Instant::now()
                .checked_duration_since(start_time)
                .unwrap_or(Duration::new(0, 0))
                .as_secs_f32()
        );
        Ok(())
    }

    async fn try_load_fallback(
        &self,
        path: &Path,
        lenient: bool,
        skip_format: Option<DataType>,
    ) -> Result<DataType, VOWLRStoreError> {
        let all_formats = [
            DataType::OFN,
            DataType::OWX,
            DataType::OWL,
            DataType::RDF,
            DataType::TTL,
            DataType::NTriples,
            DataType::NQuads,
            DataType::TriG,
            DataType::JsonLd,
            DataType::N3,
        ];

        for format in all_formats {
            if Some(format) == skip_format {
                continue;
            }
            let path_clone = path.to_path_buf();
            let parser_res =
                std::panic::catch_unwind(|| parser_from_path(&path_clone, format, lenient));

            if let Ok(Ok(parser)) = parser_res
                && self
                    .session
                    .load_from_reader(parser.parser, parser.input.as_slice())
                    .await
                    .is_ok()
            {
                return Ok(format);
            }
        }

        Err(VOWLRStoreErrorKind::InvalidFileType(
            "Could not parse file with any format".to_string(),
        )
        .into())
    }

    pub async fn serialize_to_file(&self, path: &Path) -> Result<(), VOWLRStoreError> {
        let mut file = File::create(path)?;
        let mut results = parse_stream_to(self.session.stream().await?, DataType::OWL).await?;
        while let Some(result) = results.next().await {
            std::io::Write::write_all(&mut file, &result?)?;
        }

        Ok(())
    }

    pub async fn serialize_stream(
        &self,
        resource_type: DataType,
    ) -> Result<BoxStream<'static, Result<Vec<u8>, VOWLRStoreError>>, VOWLRStoreError> {
        debug!(
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

    pub async fn complete_upload(&mut self) -> Result<DataType, VOWLRStoreError> {
        let path = if let Some(file) = &mut self.upload_handle {
            std::io::Write::flush(file)?;
            Some(file.path().to_owned())
        } else {
            None
        };

        let mut loaded_format = DataType::UNKNOWN;

        if let Some(path) = path {
            info!("Loading input into database...");
            let start_time = Instant::now();

            let path_clone = path.clone();
            let explicit_format = path_type(&path_clone);

            let mut success_format = None;

            if let Some(format) = explicit_format {
                let parser_res =
                    std::panic::catch_unwind(|| parser_from_path(&path_clone, format, false));

                if let Ok(Ok(parser)) = parser_res
                    && self
                        .session
                        .load_from_reader(parser.parser, parser.input.as_slice())
                        .await
                        .is_ok()
                {
                    success_format = Some(format);
                }
            }

            loaded_format = if let Some(format) = success_format {
                format
            } else {
                match self.try_load_fallback(&path, false, explicit_format).await {
                    Ok(fmt) => fmt,
                    Err(e) => {
                        return Err(e);
                    }
                }
            };

            info!(
                "Loaded {} quads in {} s",
                self.session.len().await?,
                Instant::now()
                    .checked_duration_since(start_time)
                    .unwrap_or(Duration::new(0, 0))
                    .as_secs_f32()
            );
        }
        self.upload_handle = None;
        Ok(loaded_format)
    }
}

impl Default for VOWLRStore {
    fn default() -> Self {
        let session = GLOBAL_STORE.get_or_init(Store::default).clone();
        Self::new(session)
    }
}

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
