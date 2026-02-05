use super::WorkbenchMenuItems;
use crate::components::icon::Icon;
use futures::StreamExt;
use leptos::prelude::*;
use leptos::server_fn::codec::{ByteStream, Streaming};
#[cfg(feature = "server")]
use vowlr_database::prelude::VOWLRStore;

#[server(output = Streaming)]
pub async fn export_owl(resource_type: String) -> Result<ByteStream<ServerFnError>, ServerFnError> {
    let store = VOWLRStore::default();
    let stream = store.serialize_stream(resource_type.into()).await?;
    Ok(ByteStream::new(stream.map(|chunk| {
        chunk
            .map_err(|e| ServerFnError::new(e.to_string()))
            .map(|bytes| bytes::Bytes::from(bytes))
    })))
}

#[component]
pub fn ExportButton(
    #[prop(into)] label: String,
    #[prop(into)] icon: icondata::Icon,
    #[prop(optional)] on_click: Option<Callback<()>>,
) -> impl IntoView {
    let onclick_handler = move |_| {
        if let Some(cb) = &on_click {
            cb.run(());
        }
    };

    view! {
        <button
            class="flex relative gap-1 justify-center items-center m-2 w-40 h-10 font-semibold bg-gray-200 rounded-sm transition-colors cursor-pointer text-[#000000] hover:bg-[#dd9900]"
            on:click=onclick_handler
        >
            <Icon icon=icon />
            {label}
        </button>
    }
}

#[cfg(target_arch = "wasm32")]
pub fn download_ontology(resource_type: &str, progress_message: RwSignal<String>) {
    let resource = resource_type.to_string();
    let (mime_type, download_name) = download_metadata(resource_type);

    leptos::task::spawn_local(async move {
        match export_owl(resource.clone()).await {
            Ok(byte_stream) => {
                progress_message.set("Downloaded: 0 MB".to_string());
                let mut stream = byte_stream.into_inner();
                let mut data = Vec::new();
                let mut downloaded = 0;
                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(bytes) => {
                            downloaded += bytes.len();
                            let mb = downloaded as f64 / 1_024.0 / 1_024.0;
                            progress_message.set(format!("Downloaded: {:.2} MB", mb));
                            data.extend(bytes);
                        }
                        Err(e) => leptos::logging::error!("Error in stream: {:?}", e),
                    }
                }
                progress_message.set("Processing...".to_string());
                leptos::logging::log!(
                    "Export success for {}, data length: {}",
                    resource,
                    data.len()
                );
                use web_sys::wasm_bindgen::JsCast;
                use web_sys::{Blob, HtmlAnchorElement, Url};

                let window = web_sys::window().unwrap();
                let document = window.document().unwrap();
                let body = document.body().unwrap();

                let blob_parts = web_sys::js_sys::Array::new();
                let uint8_array = web_sys::js_sys::Uint8Array::from(data.as_slice());
                blob_parts.push(&uint8_array.into());

                let mut blob_options = web_sys::BlobPropertyBag::new();
                blob_options.set_type(mime_type);

                let blob =
                    Blob::new_with_str_sequence_and_options(&blob_parts, &blob_options).unwrap();
                let url = Url::create_object_url_with_blob(&blob).unwrap();
                leptos::logging::log!("URL: {}", url);
                let a = document
                    .create_element("a")
                    .unwrap()
                    .unchecked_into::<HtmlAnchorElement>();
                a.set_href(&url);
                a.set_download(&download_name);
                a.set_attribute("style", "display: none").unwrap();

                body.append_child(&a).unwrap();
                a.click();
                body.remove_child(&a).unwrap();
                Url::revoke_object_url(&url).unwrap();
                progress_message.set("Download complete".to_string());
            }
            Err(e) => {
                leptos::logging::error!("Export failed: {:?}", e);
                progress_message.set(format!("Export failed: {:?}", e));
            }
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn download_ontology(resource_type: &str, progress_message: RwSignal<String>) {
    let _ = resource_type;
    progress_message.set("Ontology export is only available in the browser build.".to_string());
    leptos::logging::warn!(
        "download_ontology invoked on non-wasm target; skipping client download"
    );
}

#[cfg(target_arch = "wasm32")]
fn download_metadata(resource_type: &str) -> (&'static str, String) {
    let normalized = resource_type.to_ascii_uppercase();
    let extension = match normalized.as_str() {
        "RDF" => "rdf",
        "OWL" => "owl",
        "TTL" => "ttl",
        "NTRIPLES" => "nt",
        "NQUADS" => "nq",
        "OFN" => "ofn",
        "OWX" => "owx",
        _ => "dat",
    };

    let mime = match normalized.as_str() {
        "RDF" => "application/rdf+xml",
        "OWL" => "application/owl+xml",
        "TTL" => "text/turtle",
        "N-TRIPLES" => "application/n-triples",
        "N-QUADS" => "application/n-quads",
        "OFN" => "text/plain",
        "OWX" => "application/owl+xml",
        _ => "application/octet-stream",
    };

    (mime, format!("ontology.{}", extension))
}

#[component]
pub fn ExportMenu() -> impl IntoView {
    let progress_message = RwSignal::new(String::new());

    view! {
        <WorkbenchMenuItems title="Export Ontology">
            <div class="flex flex-wrap justify-center w-full">
                // <ExportButton label="Json" icon=icondata::BiExportRegular />
                // <ExportButton label="SVG" icon=icondata::BiExportRegular />
                // <ExportButton label="TeX" icon=icondata::BiExportRegular />
                // <ExportButton label="URL" icon=icondata::BiExportRegular />
                <ExportButton
                    label="OWL"
                    icon=icondata::BiExportRegular
                    on_click=Callback::new({
                        let progress_message = progress_message;
                        move |_| download_ontology("OWL", progress_message)
                    })
                />
                <ExportButton
                    label="RDF"
                    icon=icondata::BiExportRegular
                    on_click=Callback::new({
                        let progress_message = progress_message;
                        move |_| download_ontology("RDF", progress_message)
                    })
                />
                <ExportButton
                    label="TTL"
                    icon=icondata::BiExportRegular
                    on_click=Callback::new({
                        let progress_message = progress_message;
                        move |_| download_ontology("TTL", progress_message)
                    })
                />
                <ExportButton
                    label="N-Triples"
                    icon=icondata::BiExportRegular
                    on_click=Callback::new({
                        let progress_message = progress_message;
                        move |_| download_ontology("NTriples", progress_message)
                    })
                />
                <ExportButton
                    label="N-Quads"
                    icon=icondata::BiExportRegular
                    on_click=Callback::new({
                        let progress_message = progress_message;
                        move |_| download_ontology("NQuads", progress_message)
                    })
                />
                <ExportButton
                    label="OFN"
                    icon=icondata::BiExportRegular
                    on_click=Callback::new({
                        let progress_message = progress_message;
                        move |_| download_ontology("OFN", progress_message)
                    })
                />
                <ExportButton
                    label="OWX"
                    icon=icondata::BiExportRegular
                    on_click=Callback::new({
                        let progress_message = progress_message;
                        move |_| download_ontology("OWX", progress_message)
                    })
                />

            </div>
            {move || {
                let msg = progress_message.get();
                (!msg.is_empty())
                    .then(|| {
                        view! {
                            <div class="mt-2 w-full text-sm text-center text-gray-600">
                                {msg}
                            </div>
                        }
                    })
            }}
        </WorkbenchMenuItems>
    }
}
