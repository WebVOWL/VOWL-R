pub mod util {
    use grapher::prelude::GraphDisplayData;
    use lovet_database::prelude::{GraphDisplayDataSolutionSerializer, LOVETStore, QueryResults};

    pub async fn query(query: String) -> Result<GraphDisplayData, String> {
        let store = LOVETStore::default();

        let mut data_buffer = GraphDisplayData::new();
        let solution_serializer = GraphDisplayDataSolutionSerializer::new();
        let query_stream = store.session.query(query.as_str()).await.unwrap();
        if let QueryResults::Solutions(solutions) = query_stream {
            solution_serializer
                .serialize_nodes_stream(&mut data_buffer, solutions)
                .await
                .unwrap();
        } else {
            return Err("Query stream is not a solutions stream".to_string());
        }
        Ok(data_buffer)
    }
}
