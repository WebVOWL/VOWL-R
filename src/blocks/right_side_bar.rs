use crate::components::buttons::graph_interaction_buttons::GraphInteractionButtons;
use leptos::prelude::*;

#[component]
pub fn Accordion(#[prop(into)] title: &'static str, children: Children) -> impl IntoView {
    let (is_open, set_is_open) = signal(false);

    view! {
        <div class="border-b border-gray-200">
            <button
                class="flex justify-between items-center py-3 px-4 w-full font-medium text-left text-gray-700 transition-colors hover:bg-gray-50"
                on:click=move |_| set_is_open.update(|v| *v = !*v)
            >
                <span>{title}</span>
                <span
                    class="text-gray-500 transition-transform"
                    class=("rotate-180", move || is_open.get())
                >
                    "▼"
                </span>
            </button>
            <div
                class="overflow-hidden transition-all duration-300"
                style=move || {
                    if is_open.get() {
                        "max-height: 1000px; opacity: 1;"
                    } else {
                        "max-height: 0px; opacity: 0;"
                    }
                }
            >
                <div class="py-3 px-4 text-gray-700 bg-white">{children()}</div>
            </div>
        </div>
    }
}

#[component]
pub fn OntologyIri() -> impl IntoView {
    let ontologyiri = RwSignal::new("http://xmlns.com/foaf/0.1/".to_string());
    view! {
        <p class="flex gap-2 justify-center items-center py-2 my-2 text-sm text-gray-500">
            <a
                href=move || ontologyiri.get()
                target="_blank"
                class="text-blue-600 hover:underline"
            >
                {move || ontologyiri.get()}
            </a>
        </p>
    }
}

#[component]
pub fn Version() -> impl IntoView {
    let ontologyversion = RwSignal::new("0.99".to_string());
    view! {
        <p class="flex gap-2 justify-center items-center py-2 my-2 text-sm text-gray-500">
            "Version: "{move || ontologyversion.get()}
        </p>
    }
}

#[component]
pub fn Author() -> impl IntoView {
    let ontologyauthors = RwSignal::new("Alice, Bob, Charlie".to_string());
    view! {
        <p class="flex gap-2 justify-center items-center py-2 my-2 text-sm text-gray-500">
            Author(s): {move || ontologyauthors.get()}
        </p>
    }
}

#[component]
pub fn Language() -> impl IntoView {
    let ontologylanguages = RwSignal::new(vec![
        "english".to_string(),
        "german".to_string(),
        "french".to_string(),
    ]);
    view! {
        <p class="flex gap-2 justify-center items-center py-2 my-2 text-sm text-gray-500">
            "Language(s):"
            <select class="py-1 px-2 text-sm text-gray-500 rounded-md border border-gray-300 focus:ring-2 focus:ring-blue-500 focus:outline-none w-[100px] h-[30px]">
                {move || {
                    ontologylanguages
                        .get()
                        .into_iter()
                        .map(|lang| view! { <option>{lang}</option> })
                        .collect_view()
                }}
            </select>
        </p>
    }
}

#[component]
pub fn Description() -> impl IntoView {
    let ontologydescription = RwSignal::new("The Friend of a Friend (FOAF) RDF vocabulary, described using W3C RDF Schema and the Web Ontology Language.".to_string());
    view! {
        <Accordion title="Description">
            <p>{move || ontologydescription.get()}</p>
        </Accordion>
    }
}

#[component]
pub fn MetaData() -> impl IntoView {
    let metadata = RwSignal::new("The Friend of a Friend (FOAF) RDF vocabulary, described using W3C RDF Schema and the Web Ontology Language.The Friend of a Friend (FOAF) RDF vocabulary, described using W3C RDF Schema and the Web Ontology Language.The Friend of a Friend (FOAF) RDF vocabulary, described using W3C RDF Schema and the Web Ontology Language.The Friend of a Friend (FOAF) RDF vocabulary, described using W3C RDF Schema and the Web Ontology Language.The Friend of a Friend (FOAF) RDF vocabulary, described using W3C RDF Schema and the Web Ontology Language.The Friend of a Friend (FOAF) RDF vocabulary, described using W3C RDF Schema and the Web Ontology Language.The Friend of a Friend (FOAF) RDF vocabulary, described using W3C RDF Schema and the Web Ontology Language.The Friend of a Friend (FOAF) RDF vocabulary, described using W3C RDF Schema and the Web Ontology Language.The Friend of a Friend (FOAF) RDF vocabulary, described using W3C RDF Schema and the Web Ontology Language.".to_string());
    view! {
        <Accordion title="Metadata">
            <p>{move || metadata.get()}</p>
        </Accordion>
    }
}

#[component]
pub fn SelectionDetails() -> impl IntoView {
    let selection_details = RwSignal::new("Select an element in the visualization.".to_string());
    view! {
        <Accordion title="Selection Details">
            <p>{move || selection_details.get()}</p>
        </Accordion>
    }
}

#[component]
pub fn RightSidebar() -> impl IntoView {
    let is_open = RwSignal::new(false);
    view! {
        <div data-sidebar-open=move || is_open.get().to_string()>
            <button
                class="flex fixed justify-center items-center w-6 h-6 bg-white border border-black duration-500 cursor-pointer top-[5%] z-[3] transition-[right] hover:bg-[#dd9900]"
                class=("right-[22%]", move || is_open.get())
                class=("right-0", move || !is_open.get())
                on:click=move |_| {
                    is_open.update(|value| *value = !*value);
                }
            >
                {move || if is_open.get() { ">" } else { "<" }}
            </button>
            <div
                class="overflow-y-auto overflow-x-hidden fixed top-0 right-0 h-screen text-gray-500 bg-white duration-500 transition-[width]"
                class=("w-[22%]", move || is_open.get())
                class=("w-0", move || !is_open.get())
            >
                <div
                    class="transition-opacity duration-500"
                    class=("opacity-100", move || is_open.get())
                    class=(
                        "opacity-0 pointer-events-none",
                        move || !is_open.get(),
                    )
                >
                    <p class="py-4 font-thin text-center text-gray-500 text-[1.5em]">
                        "Friend of a Friend (FOAF) vocabulary"
                    </p>
                    <OntologyIri />
                    <Version />
                    <Author />
                    <Language />
                    <Description />
                    <MetaData />
                    <SelectionDetails />
                </div>
            </div>
            <GraphInteractionButtons is_sidebar_open=is_open />
        </div>
    }
}
