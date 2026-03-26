use grapher::prelude::{EVENT_DISPATCHER, GraphDisplayData, RenderEvent};
use leptos::{prelude::*, server_fn::codec::Rkyv};
#[cfg(feature = "server")]
use vowlr_database::prelude::VOWLRStore;
use vowlr_util::prelude::VOWLRError;

use crate::{
    blocks::workbench::GraphDataContext,
    errors::{ClientErrorKind, ErrorLogContext},
};

#[server (input = Rkyv, output = Rkyv)]
pub async fn handle_internal_sparql(
    query: String,
) -> Result<(GraphDisplayData, Option<VOWLRError>), VOWLRError> {
    let store = VOWLRStore::default();
    store.query(query).await
}

pub async fn load_graph(query: String) {
    let GraphDataContext {
        graph_data,
        total_graph_data,
    } = expect_context::<GraphDataContext>();

    let error_context = expect_context::<ErrorLogContext>();

    match handle_internal_sparql(query).await {
        Ok((result, non_fatal_error)) => {
            graph_data.set(result.clone());
            total_graph_data.set(result.clone());
            if let Err(e) = EVENT_DISPATCHER
                .rend_write_chan
                .send(RenderEvent::LoadGraph(result))
            {
                error_context.push(ClientErrorKind::EventHandlingError(e.to_string()).into());
            }
            if let Some(e) = non_fatal_error {
                error_context.extend(e.records);
            }
        }
        Err(e) => {
            error_context.extend(e.records);
        }
    }
}
