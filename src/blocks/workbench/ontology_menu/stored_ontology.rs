use grapher::prelude::GraphDisplayData;
use leptos::prelude::*;
use leptos::server_fn::ServerFnError;
use leptos::server_fn::codec::Rkyv;
use std::path::Path;
#[cfg(feature = "server")]
use vowlr_database::prelude::{GraphDisplayDataSolutionSerializer, QueryResults, VOWLRStore};
use vowlr_sparql_queries::prelude::DEFAULT_QUERY;

fn ontology_file_path(name: &str) -> Result<&'static str, ServerFnError<String>> {
    match name {
        "Clinical Trials Ontology (CTO) (273 classes)" => {
            Ok("src/assets/data/ClinicalTrialOntology-merged.owl")
        }
        "Friend of a Friend (FOAF) vocabulary (22 classes)" => Ok("src/assets/data/foaf.ttl"),
        "VOWL-R Benchmark Ontology (2.5k nodes)" => Ok("src/assets/data/vowlr-benchmark-2500.ofn"),
        "The Environment Ontology (6.9k classes)" => Ok("src/assets/data/envo.owl"),
        _ => Err(ServerFnError::ServerError(format!(
            "Unknown ontology: {name}"
        ))),
    }
}

#[server(input = Rkyv, output = Rkyv)]
pub async fn load_stored_ontology(name: String) -> Result<GraphDisplayData, ServerFnError<String>> {
    let file_path = ontology_file_path(&name)?;
    let path = Path::new(file_path);

    let store = VOWLRStore::default();
    store
        .insert_file(path, false)
        .await
        .map_err(|e| ServerFnError::ServerError(format!("Failed to load ontology file: {e}")))?;

    let mut data_buffer = GraphDisplayData::new();
    let solution_serializer = GraphDisplayDataSolutionSerializer::new();
    let query_stream = store
        .session
        .query(DEFAULT_QUERY.as_str())
        .await
        .map_err(|e| ServerFnError::ServerError(format!("SPARQL query failed: {e}")))?;

    if let QueryResults::Solutions(solutions) = query_stream {
        solution_serializer
            .serialize_nodes_stream(&mut data_buffer, solutions)
            .await
            .map_err(|e| ServerFnError::ServerError(e.to_string()))?;
    } else {
        return Err(ServerFnError::ServerError(
            "Query stream is not a solutions stream".to_string(),
        ));
    }
    Ok(data_buffer)
}
