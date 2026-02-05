use futures::StreamExt;
use gloo_timers::callback::Interval;
use grapher::prelude::GraphDisplayData;
use leptos::prelude::*;
use leptos::server_fn::ServerFnError;
use leptos::server_fn::codec::{MultipartData, MultipartFormData, Rkyv, StreamingText, TextStream};
use leptos::task::spawn_local;
use log::{debug, info};
#[cfg(feature = "server")]
use reqwest::Client;
use std::cell::RefCell;
#[cfg(feature = "server")]
use std::path::Path;
use std::rc::Rc;
#[cfg(feature = "server")]
use vowlr_database::prelude::{GraphDisplayDataSolutionSerializer, QueryResults, VOWLRStore};
use vowlr_util::datatypes::DataType;
use web_sys::{FileList, FormData};

#[cfg(feature = "ssr")]
mod progress {
    use async_broadcast::{Receiver, Sender, broadcast};
    use dashmap::DashMap;
    use futures::Stream;
    use std::sync::LazyLock;

    struct File {
        total: usize,
        tx: Sender<usize>,
        rx: Receiver<usize>,
    }

    static FILES: LazyLock<DashMap<String, File>> = LazyLock::new(DashMap::new);

    pub async fn add_chunk(filename: &str, len: usize) {
        let mut entry = FILES.entry(filename.to_string()).or_insert_with(|| {
            let (mut tx, rx) = broadcast(128);
            tx.set_overflow(true);
            File { total: 0, tx, rx }
        });
        entry.total += len;
        let new_total = entry.total;

        let tx = entry.tx.clone();
        drop(entry);

        let _ = tx.broadcast(new_total).await;
    }

    pub fn reset(filename: &str) {
        if let Some(mut entry) = FILES.get_mut(filename) {
            entry.total = 0;
        }
    }

    pub fn remove(filename: &str) {
        if FILES.remove(filename).is_some() {
            // println!("Removed progress entry for '{}'", filename);
        }
    }

    pub fn for_file(filename: String) -> impl Stream<Item = usize> {
        let entry = FILES.entry(filename).or_insert_with(|| {
            let (mut tx, rx) = broadcast(2048);
            tx.set_overflow(true);
            File { total: 0, tx, rx }
        });
        entry.rx.clone()
    }
}

#[server(output = StreamingText)]
pub async fn ontology_progress(filename: String) -> Result<TextStream, ServerFnError> {
    debug!("Initializing progress counter with file {filename}");
    let progress = progress::for_file(filename);
    debug!("Mapping usize to String");
    let progress = progress.map(|bytes| Ok(format!("{bytes}\n")));
    debug!("Creating text stream");
    let ts = TextStream::new(progress);
    debug!("OK");
    Ok(ts)
}

#[server(
    input = MultipartFormData,
)]
pub async fn handle_local(data: MultipartData) -> Result<(DataType, usize), ServerFnError> {
    let mut session = VOWLRStore::default();
    let mut data = data.into_inner().unwrap();
    let mut count = 0;
    let mut dtype = DataType::UNKNOWN;
    while let Ok(Some(mut field)) = data.next_field().await {
        let name = field.file_name().unwrap_or_default().to_string();

        if !name.is_empty() {
            info!("Receiving file '{}'", name);
            progress::reset(&name);
            debug!("Resetting progress");

            session
                .start_upload(&name)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;

            dtype = Path::new(&name).into();
        } else {
            return Err(ServerFnError::new("Received empty file string"));
        }

        while let Ok(Some(chunk)) = field.chunk().await {
            let len = chunk.len();
            count += len;
            session
                .upload_chunk(&chunk)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            progress::add_chunk(&name, len).await;
        }

        if !name.is_empty() {
            progress::remove(&name);
        }
    }

    session
        .complete_upload()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok((dtype, count))
}

/// Remote reads url and calls for the datatype label and returns (label, data content)
#[server]
pub async fn handle_remote(url: String) -> Result<(DataType, usize), ServerFnError> {
    debug!("Sending request to remote: '{url}'");
    let client = Client::new();
    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            return Err(ServerFnError::ServerError(format!(
                "Error fetching URL: {e}"
            )));
        }
    };

    let mut session = VOWLRStore::default();
    let progress_key = url.clone();
    progress::reset(&progress_key);
    session
        .start_upload(&url)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let mut total = 0;
    let dtype = Path::new(&url).into();

    let mut stream = resp.bytes_stream();
    while let Some(chunk_result) = stream.next().await {
        let chunk =
            chunk_result.map_err(|e| ServerFnError::new(format!("Error reading chunk: {e}")))?;

        total += chunk.len();
        session
            .upload_chunk(&chunk)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        progress::add_chunk(&progress_key, chunk.len()).await;
    }

    progress::remove(&progress_key);
    session
        .complete_upload()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok((dtype, total))
}

/// Sparql reads (endpoint + query) and calls for the datatype label and returns (label, data content)
#[server]
pub async fn handle_sparql(
    endpoint: String,
    query: String,
    format: Option<String>,
) -> Result<(DataType, usize), ServerFnError> {
    let client = Client::new();
    let mut session = VOWLRStore::default();

    let accept_type = match format.as_deref() {
        Some("xml") => DataType::SPARQLXML.mime_type(),
        _ => DataType::SPARQLJSON.mime_type(),
    };

    let resp = client
        .post(&endpoint)
        .header("Accept", accept_type)
        .form(&[("query", query)])
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("Error querying SPARQL endpoint: {e}")))?;

    let progress_key = format!("sparql-{}", endpoint);
    progress::reset(&progress_key);
    session
        .start_upload(&progress_key)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let mut total = 0;
    let mut stream = resp.bytes_stream();
    while let Some(chunk_result) = stream.next().await {
        let chunk =
            chunk_result.map_err(|e| ServerFnError::new(format!("Error reading chunk: {e}")))?;

        total += chunk.len();
        session
            .upload_chunk(&chunk)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        progress::add_chunk(&progress_key, chunk.len()).await;
    }

    progress::remove(&progress_key);
    session
        .complete_upload()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let dtype = if accept_type.contains("xml") {
        DataType::SPARQLXML
    } else {
        DataType::SPARQLJSON
    };
    Ok((dtype, total))
}

#[server (input = Rkyv, output = Rkyv)]
pub async fn handle_internal_sparql(query: String) -> Result<GraphDisplayData, ServerFnError> {
    let vowlr = VOWLRStore::default();

    let mut data_buffer = GraphDisplayData::new();
    let solution_serializer = GraphDisplayDataSolutionSerializer::new();
    let query_stream = vowlr.session.query(query.as_str()).await.unwrap();
    if let QueryResults::Solutions(solutions) = query_stream {
        solution_serializer
            .serialize_nodes_stream(&mut data_buffer, solutions)
            .await
            .unwrap();
    } else {
        return Err(ServerFnError::ServerError(
            "Query stream is not a solutions stream".to_string(),
        ));
    }
    Ok(data_buffer)
}

pub struct UploadProgress {
    pub filename: RwSignal<String>,
    pub file_size: RwSignal<usize>,
    pub upload_progress: RwSignal<i32>,
    pub parsing_status: RwSignal<String>,
    pub parsing_done: RwSignal<bool>,
    pub interval_handle: Rc<RefCell<Option<Interval>>>,
}
impl UploadProgress {
    pub fn new() -> Self {
        Self {
            filename: RwSignal::new("Select File".to_string()),
            file_size: RwSignal::new(0),
            upload_progress: RwSignal::new(0),
            parsing_status: RwSignal::new(String::new()),
            parsing_done: RwSignal::new(false),
            interval_handle: Rc::new(RefCell::new(None)),
        }
    }

    fn track_progress<F>(&self, key: String, total_size: Option<usize>, dispatch: F)
    where
        F: FnOnce() + 'static,
    {
        self.filename.set(key.clone());
        self.upload_progress.set(0);
        self.parsing_status.set(String::new());
        self.parsing_done.set(false);

        let progress = self.upload_progress.clone();
        let status = self.parsing_status.clone();
        let done = self.parsing_done.clone();
        let interval_handle = Rc::clone(&self.interval_handle);

        spawn_local(async move {
            dispatch();
            // Code below is progress bar and it only works on Chromium-based browsers (sometimes)
            // match ontology_progress(key).await {
            //     Ok(stream_result) => {
            //         debug!("Dispatching");
            //         dispatch();
            //         let mut stream = stream_result.into_inner();
            //         while let Some(result) = stream.next().await {
            //             match result {
            //                 Ok(chunk) => {
            //                     if let Ok(bytes) = chunk.trim().parse::<usize>() {
            //                         if let Some(total) = total_size {
            //                             let percent = (bytes as f64 / total as f64) * 100.0;
            //                             progress.set(percent as i32);
            //                         } else {
            //                             let current = progress.get();
            //                             progress.set((current + 5).min(95));
            //                             // progress.set(new_progress);
            //                         }
            //                     }
            //                 }
            //                 Err(e) => error!("{}", e),
            //             }
            //         }

            //         progress.set(100);
            //         status.set("Parsing".to_string());

            //         let interval = Interval::new(1500, move || {
            //             status.update(|s| {
            //                 if s.ends_with("......") {
            //                     *s = "Parsing".to_string();
            //                 } else {
            //                     s.push('.');
            //                 }
            //             });
            //         });

            //         let mut handle = interval_handle.borrow_mut();
            //         if let Some(existing) = handle.take() {
            //             existing.cancel();
            //         }
            //         *handle = Some(interval);
            //         done.set(true);
            //     }
            //     Err(e) => {
            //         error!("Failed to connect to progress stream: {:?}", e);
            //         dispatch();
            //     }
            // }
        });
    }

    pub fn upload_files<F>(&self, file_list: FileList, dispatch: F)
    where
        F: FnOnce(FormData) + 'static,
    {
        let len = file_list.length();
        let form = FormData::new().unwrap();
        info!("Preparing filelist with {len} files");

        // let mut total_size = 0;
        if let Some(file) = file_list.item(0) {
            self.filename.set(file.name());
            self.file_size.set(file.size() as usize);
        }

        for i in 0..len {
            if let Some(file) = file_list.item(i) {
                form.append_with_blob("file_to_upload", &file).unwrap();
            }
        }

        let fname = self.filename.get_untracked();
        self.track_progress(fname, Some(self.file_size.get()), move || dispatch(form));
    }

    pub fn upload_url<F>(&self, url: String, dispatch: F)
    where
        F: FnOnce(String) + 'static,
    {
        self.track_progress(url.clone(), None, move || dispatch(url));
    }

    pub fn upload_sparql<F>(&self, endpoint: String, query: String, dispatch: F)
    where
        F: FnOnce((String, String, Option<String>)) + 'static,
    {
        let key = format!("sparql-{}", endpoint);
        let ep = endpoint.clone();
        let q = query.clone();
        let fmt = Some("json".to_string());
        self.track_progress(key, None, move || dispatch((ep, q, fmt)));
    }
}

/// handles what server side function to use (local, remote or sparql)
#[derive(Clone)]
pub struct FileUpload {
    pub mode: RwSignal<String>,
    pub local_action: Action<FormData, Result<(DataType, usize), ServerFnError>>,
    pub remote_action: Action<String, Result<(DataType, usize), ServerFnError>>,
    pub sparql_action:
        Action<(String, String, Option<String>), Result<(DataType, usize), ServerFnError>>,
    pub tracker: Rc<UploadProgress>,
}

impl FileUpload {
    pub fn new() -> Self {
        let mode = RwSignal::new("local".to_string());

        let local_action =
            Action::<FormData, Result<(DataType, usize), ServerFnError>>::new_local(|data| {
                handle_local(data.clone().into())
            });

        let remote_action =
            Action::<String, Result<(DataType, usize), ServerFnError>>::new(|url| {
                handle_remote(url.clone())
            });

        let sparql_action = Action::<
            (String, String, Option<String>),
            Result<(DataType, usize), ServerFnError>,
        >::new(|(endpoint, query, format)| {
            handle_sparql(endpoint.clone(), query.clone(), format.clone())
        });

        let tracker = Rc::new(UploadProgress::new());

        Self {
            mode,
            local_action,
            remote_action,
            sparql_action,
            tracker,
        }
    }

    pub fn get_result(&self) -> Option<Result<(DataType, usize), ServerFnError>> {
        match self.mode.get().as_str() {
            "local" => self.local_action.value().get(),
            "remote" => self.remote_action.value().get(),
            "sparql" => self.sparql_action.value().get(),
            _ => None,
        }
    }
}
