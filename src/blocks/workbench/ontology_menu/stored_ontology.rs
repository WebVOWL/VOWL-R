use grapher::prelude::GraphDisplayData;
use leptos::prelude::*;
use leptos::server_fn::ServerFnError;
use leptos::server_fn::codec::Rkyv;
#[cfg(feature = "server")]
use lovet_database::prelude::{GraphDisplayDataSolutionSerializer, LOVETStore, QueryResults};
#[cfg(feature = "server")]
use lovet_parser::errors::LOVETStoreError;
use lovet_sparql_queries::prelude::DEFAULT_QUERY;
use lovet_util::prelude::LOVETError;
use std::path::Path;

fn ontology_file_path(name: &str) -> Result<&'static str, LOVETError> {
    match name {
        "Clinical Trials Ontology (CTO) (273 classes)" => {
            Ok("src/assets/data/ClinicalTrialOntology-merged.owl")
        }
        "Friend of a Friend (FOAF) vocabulary (22 classes)" => Ok("src/assets/data/foaf.ttl"),
        "LOVET Benchmark Ontology (2.5k nodes)" => Ok("src/assets/data/lovet-benchmark-2500.ofn"),
        "The Environment Ontology (6.9k classes)" => Ok("src/assets/data/envo.owl"),
        _ => Err(ServerFnError::ServerError(format!("Unknown ontology: {name}")).into()),
    }
}

#[server(input = Rkyv, output = Rkyv)]
pub async fn load_stored_ontology(name: String) -> Result<GraphDisplayData, LOVETError> {
    let file_path = ontology_file_path(&name)?;
    let path = Path::new(file_path);

    let store = LOVETStore::default();
    store.insert_file(path, false).await?;

    let mut data_buffer = GraphDisplayData::new();
    let solution_serializer = GraphDisplayDataSolutionSerializer::new();
    let query_stream = store
        .session
        .query(DEFAULT_QUERY.as_str())
        .await
        .map_err(|e| <LOVETStoreError as Into<LOVETError>>::into(e.into()))?;

    if let QueryResults::Solutions(solutions) = query_stream {
        solution_serializer
            .serialize_nodes_stream(&mut data_buffer, solutions)
            .await?;
    } else {
        return Err(ServerFnError::ServerError(
            "Query stream is not a solutions stream (only SELECT queries are supported)"
                .to_string(),
        )
        .into());
    }
    Ok(data_buffer)
}
