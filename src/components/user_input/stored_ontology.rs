use leptos::prelude::*;
use leptos::server_fn::ServerFnError;
use leptos::server_fn::codec::Rkyv;
use std::path::Path;
#[cfg(feature = "server")]
use vowlr_database::prelude::VOWLRStore;
use vowlr_util::prelude::VOWLRError;

use crate::components::user_input::internal_sparql::load_graph;

fn ontology_file_path(name: &str) -> Result<&'static str, VOWLRError> {
    match name {
        "Clinical Trials Ontology (CTO) (273 classes)" => {
            Ok("src/assets/data/ClinicalTrialOntology-merged.owl")
        }
        "Friend of a Friend (FOAF) vocabulary (22 classes)" => Ok("src/assets/data/foaf.ttl"),
        "VOWL-R Benchmark Ontology (2.5k nodes)" => Ok("src/assets/data/vowlr-benchmark-2500.ofn"),
        "The Environment Ontology (6.9k classes)" => Ok("src/assets/data/envo.owl"),
        _ => Err(ServerFnError::ServerError(format!("Unknown ontology: {name}")).into()),
    }
}

#[server(input = Rkyv, output = Rkyv)]
pub async fn load_stored_ontology(name: String, query: String) -> Result<(), VOWLRError> {
    let file_path = ontology_file_path(&name)?;
    let path = Path::new(file_path);
    let store = VOWLRStore::default();

    store.insert_file(path, false).await?;
    load_graph(query).await;
    Ok(())
}
