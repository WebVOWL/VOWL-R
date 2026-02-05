use crate::errors::{VOWLRStoreError, VOWLRStoreErrorKind};
use futures::{StreamExt, stream::BoxStream};
use horned_owl::{
    io::{rdf::reader::ConcreteRDFOntology, *},
    model::{RcAnnotatedComponent, RcStr},
    ontology::component_mapped::RcComponentMappedOntology,
};
use log::info;
use rdf_fusion::{
    execution::results::QuadStream,
    io::{JsonLdProfileSet, RdfFormat, RdfParser, RdfSerializer},
    model::GraphName,
};
use std::io;
use std::{
    fs::File,
    io::{BufReader, Cursor, Write},
    path::Path,
    time::{Duration, Instant},
};
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;
use vowlr_util::datatypes::DataType;

pub enum ParserInput {
    File(Vec<u8>),
    Buffer(Cursor<Vec<u8>>),
}

impl ParserInput {
    pub fn from_path(path: &Path) -> Result<Self, VOWLRStoreError> {
        std::fs::read(path)
            .map(ParserInput::File)
            .map_err(VOWLRStoreError::from)
    }

    pub fn as_slice(&self) -> &[u8] {
        match self {
            ParserInput::Buffer(cursor) => cursor.get_ref().as_slice(),
            ParserInput::File(bytes) => bytes.as_slice(),
        }
    }
}

pub struct PreparedParser {
    pub parser: RdfParser,
    pub input: ParserInput,
}

pub fn path_type(path: &Path) -> Option<DataType> {
    match path.extension().and_then(|s| s.to_str()) {
        Some("ofn") => Some(DataType::OFN),
        Some("owx") => Some(DataType::OWX),
        Some("rdf") => Some(DataType::RDF),
        Some("owl") => Some(DataType::OWL),
        Some("ttl") => Some(DataType::TTL),
        Some("nt") => Some(DataType::NTriples),
        Some("nq") => Some(DataType::NQuads),
        Some("trig") => Some(DataType::TriG),
        Some("jsonld") => Some(DataType::JsonLd),
        Some("n3") => Some(DataType::N3),
        _ => None,
    }
}
pub fn format_from_resource_type(resource_type: &DataType) -> Option<RdfFormat> {
    match resource_type {
        DataType::RDF => Some(RdfFormat::RdfXml),
        DataType::TTL => Some(RdfFormat::Turtle),
        DataType::NTriples => Some(RdfFormat::NTriples),
        DataType::NQuads => Some(RdfFormat::NQuads),
        DataType::TriG => Some(RdfFormat::TriG),
        DataType::JsonLd => Some(RdfFormat::JsonLd {
            profile: JsonLdProfileSet::default(),
        }),
        DataType::N3 => Some(RdfFormat::N3),
        DataType::OWL => Some(RdfFormat::RdfXml),
        _ => None,
    }
}
pub async fn parse_stream_to(
    mut stream: QuadStream,
    output_type: DataType,
) -> Result<BoxStream<'static, Result<Vec<u8>, VOWLRStoreError>>, VOWLRStoreError> {
    match output_type {
        DataType::OFN | DataType::OWX | DataType::OWL => {
            let (tx, rx) = mpsc::unbounded_channel();
            let mut buf = Vec::new();
            let mut serializer =
                RdfSerializer::from_format(format_from_resource_type(&DataType::OWL).ok_or(
                    VOWLRStoreErrorKind::InvalidInput(format!(
                        "Unsupported output type: {:?}",
                        output_type
                    )),
                )?)
                .for_writer(&mut buf);
            while let Some(quad) = stream.next().await {
                serializer.serialize_quad(&quad?)?;
            }
            serializer.finish()?;

            let mut reader = BufReader::new(Cursor::new(buf));
            tokio::task::spawn_blocking(move || {
                let mut writer = ChannelWriter { sender: tx.clone() };
                let result = (|| match output_type {
                    DataType::OFN => {
                        let (ont, prefix): (RcComponentMappedOntology, _) =
                            ofn::reader::read(&mut reader, ParserConfiguration::default())?;
                        ofn::writer::write(&mut writer, &ont, Some(&prefix))?;
                        writer.flush()?;
                        Ok(writer)
                    }
                    DataType::OWX => {
                        let (ont, prefix): (RcComponentMappedOntology, _) =
                            owx::reader::read(&mut reader, ParserConfiguration::default())?;
                        owx::writer::write(&mut writer, &ont, Some(&prefix))?;
                        writer.flush()?;
                        Ok(writer)
                    }
                    DataType::OWL => {
                        let (ont, _): (ConcreteRDFOntology<RcStr, RcAnnotatedComponent>, _) =
                            rdf::reader::read(&mut reader, ParserConfiguration::default())?;
                        rdf::writer::write(&mut writer, &ont.into())?;
                        writer.flush()?;
                        Ok(writer)
                    }
                    _ => Err(VOWLRStoreError::from(VOWLRStoreErrorKind::InvalidInput(
                        format!("Unsupported output type: {:?}", output_type),
                    ))),
                })();

                if let Err(e) = result {
                    let _ = tx.send(Err(e.into()));
                }
            });
            Ok(UnboundedReceiverStream::new(rx)
                .map(|result| result.map_err(VOWLRStoreError::from))
                .boxed())
        }
        _ => {
            let (tx, rx) = mpsc::unbounded_channel();
            tokio::task::spawn(async move {
                let mut writer = ChannelWriter { sender: tx.clone() };
                let result = (|| async {
                    let mut serializer =
                        RdfSerializer::from_format(format_from_resource_type(&output_type).ok_or(
                            VOWLRStoreErrorKind::InvalidInput(format!(
                                "Unsupported output type: {:?}",
                                output_type
                            )),
                        )?)
                        .for_writer(&mut writer);
                    while let Some(quad) = stream.next().await {
                        serializer.serialize_quad(&quad?)?;
                    }
                    serializer.finish()?;
                    Ok::<ChannelWriter, VOWLRStoreError>(writer)
                })();

                if let Err(e) = result.await {
                    let _ = tx.send(Err(e.into()));
                }
            });
            Ok(UnboundedReceiverStream::new(rx)
                .map(|result| result.map_err(VOWLRStoreError::from))
                .boxed())
        }
    }
}

pub fn parser_from_format(path: &Path, lenient: bool) -> Result<PreparedParser, VOWLRStoreError> {
    let make_parser = |fmt| {
        let path_str = path.to_str().unwrap();
        // TODO: Handle non default graph
        let parser = RdfParser::from_format(fmt).with_default_graph(GraphName::DefaultGraph);
        //.with_default_graph(NamedNode::new(format!("file:://{}", path_str)).unwrap());
        if lenient { parser.lenient() } else { parser }
    };
    let t_pat: Option<DataType> = path_type(path);
    let prepared = match t_pat {
        Some(DataType::OFN) => {
            let file = File::open(path)?;
            let mut reader = BufReader::new(file);

            info!("Parsing OFN input...");
            let start_time = Instant::now();

            let (ont, _): (RcComponentMappedOntology, _) =
                ofn::reader::read(&mut reader, ParserConfiguration::default())?;

            info!(
                "Parsing completed in {} s",
                Instant::now()
                    .checked_duration_since(start_time)
                    .unwrap_or(Duration::new(0, 0))
                    .as_secs_f32()
            );

            info!("Writing to RDF...");
            let start_time = Instant::now();

            let mut buf = Vec::new();
            rdf::writer::write(&mut buf, &ont)?;

            info!(
                "Writing completed in {} s",
                Instant::now()
                    .checked_duration_since(start_time)
                    .unwrap_or(Duration::new(0, 0))
                    .as_secs_f32()
            );

            Ok(PreparedParser {
                parser: make_parser(RdfFormat::RdfXml),
                input: ParserInput::Buffer(Cursor::new(buf)),
            })
        }
        Some(DataType::OWX) => {
            let file = File::open(path)?;
            let mut reader = BufReader::new(file);

            info!("Parsing OWX input...");
            let start_time = Instant::now();

            let ontology = owx::reader::read::<
                RcStr,
                ConcreteRDFOntology<RcStr, RcAnnotatedComponent>,
                _,
            >(&mut reader, ParserConfiguration::default())?;

            info!(
                "Parsing completed in {} s",
                Instant::now()
                    .checked_duration_since(start_time)
                    .unwrap_or(Duration::new(0, 0))
                    .as_secs_f32()
            );

            info!("Writing to RDF...");
            let start_time = Instant::now();

            let mut buf = Vec::new();
            rdf::writer::write(&mut buf, &ontology.0.into())?;

            info!(
                "Writing completed in {} s",
                Instant::now()
                    .checked_duration_since(start_time)
                    .unwrap_or(Duration::new(0, 0))
                    .as_secs_f32()
            );
            Ok(PreparedParser {
                parser: make_parser(RdfFormat::RdfXml),
                input: ParserInput::Buffer(Cursor::new(buf)),
            })
        }
        Some(DataType::OWL) => {
            info!("Parsing OWL input...");
            let start_time = Instant::now();

            let b = horned_owl::model::Build::<RcStr>::new();
            let iri = horned_owl::resolve::path_to_file_iri(&b, path);
            let (ontology, _) = rdf::closure_reader::read::<
                RcStr,
                RcAnnotatedComponent,
                ConcreteRDFOntology<RcStr, RcAnnotatedComponent>,
            >(&iri, ParserConfiguration::default())?;

            info!(
                "Parsing completed in {} s",
                Instant::now()
                    .checked_duration_since(start_time)
                    .unwrap_or(Duration::new(0, 0))
                    .as_secs_f32()
            );

            info!("Writing to RDF...");
            let start_time = Instant::now();

            let mut buf = Vec::new();
            rdf::writer::write(&mut buf, &ontology.into())?;

            info!(
                "Writing completed in {} s",
                Instant::now()
                    .checked_duration_since(start_time)
                    .unwrap_or(Duration::new(0, 0))
                    .as_secs_f32()
            );

            Ok(PreparedParser {
                parser: make_parser(RdfFormat::RdfXml),
                input: ParserInput::Buffer(Cursor::new(buf)),
            })
        }
        Some(DataType::TTL) => {
            let input = ParserInput::from_path(path)?;
            Ok(PreparedParser {
                parser: make_parser(RdfFormat::Turtle),
                input,
            })
        }
        Some(DataType::NTriples) => {
            let input = ParserInput::from_path(path)?;
            Ok(PreparedParser {
                parser: make_parser(RdfFormat::NTriples),
                input,
            })
        }
        Some(DataType::NQuads) => {
            let input = ParserInput::from_path(path)?;
            Ok(PreparedParser {
                parser: make_parser(RdfFormat::NQuads),
                input,
            })
        }
        Some(DataType::TriG) => {
            let input = ParserInput::from_path(path)?;
            Ok(PreparedParser {
                parser: make_parser(RdfFormat::TriG),
                input,
            })
        }
        Some(DataType::JsonLd) => {
            let input = ParserInput::from_path(path)?;
            Ok(PreparedParser {
                parser: make_parser(RdfFormat::JsonLd {
                    profile: JsonLdProfileSet::default(),
                }),
                input,
            })
        }
        Some(DataType::N3) => {
            let input = ParserInput::from_path(path)?;
            Ok(PreparedParser {
                parser: make_parser(RdfFormat::N3),
                input,
            })
        }
        _ => Err(VOWLRStoreErrorKind::InvalidInput(format!(
            "Unsupported parser: {}",
            path.display()
        ))),
    };
    Ok(prepared?)
}
struct ChannelWriter {
    sender: UnboundedSender<Result<Vec<u8>, io::Error>>,
}

impl Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let data = buf.to_vec();

        self.sender
            .send(Ok(data))
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "Stream receiver dropped"))?;

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(()) // No internal buffering to flush
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_ofn_parser() {
        let resources = resources_with_suffix("data/owl-functional", "ofn");
        use rdf_fusion::store::Store;
        let session = Store::default();
        for resource in resources {
            let parser = parser_from_format(Path::new(&resource), false).unwrap();
            let _ = session
                .load_from_reader(parser.parser, parser.input.as_slice())
                .await;
            assert_ne!(
                session.len().await.unwrap(),
                0,
                "Expected non-zero quads for: {}",
                resource
            );
        }
    }

    #[tokio::test]
    async fn test_owl_parser() {
        use rdf_fusion::store::Store;
        let session = Store::default();
        let resources = resources_with_suffix("data/owl-rdf", "owl");
        for resource in resources {
            let parser = parser_from_format(Path::new(&resource), false).unwrap();
            let _ = session
                .load_from_reader(parser.parser, parser.input.as_slice())
                .await;
            assert_ne!(
                session.len().await.unwrap(),
                0,
                "Expected non-zero quads for: {}",
                resource
            );
        }
    }
    #[tokio::test]
    async fn test_ttl_parser() {
        use rdf_fusion::store::Store;
        let session = Store::default();
        let resources = resources_with_suffix("data/owl-ttl", "ttl");
        for resource in resources {
            let parser = parser_from_format(Path::new(&resource), false).unwrap();
            let _ = session
                .load_from_reader(parser.parser, parser.input.as_slice())
                .await;
            assert_ne!(
                session.len().await.unwrap(),
                0,
                "Expected non-zero quads for: {}",
                resource
            );
        }
    }
    fn resources_with_suffix(relative_dir: &str, suffix: &str) -> Vec<String> {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let base_dir = Path::new(manifest_dir).join(relative_dir);
        let suffix = suffix.trim_start_matches(|c| c == '.' || c == '*');
        let mut resources = Vec::new();

        let entries = std::fs::read_dir(&base_dir).unwrap_or_else(|err| {
            panic!(
                "Failed to read resources directory {}: {}",
                base_dir.display(),
                err
            )
        });

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == suffix)
                    .unwrap_or(false)
            {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    let dir = relative_dir.trim_end_matches('/');
                    resources.push(format!("{}/{}", dir, file_name));
                }
            }
        }

        resources.sort();
        resources
    }
}
