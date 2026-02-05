mod classes;
mod element_legend_injection;
mod filtergroup;
mod filtertype;
mod meta_filter;
mod properties;
mod special_operators;

use super::{GraphDataContext, WorkbenchMenuItems};
use crate::components::user_input::file_upload::handle_internal_sparql;
use grapher::prelude::{EVENT_DISPATCHER, RenderEvent};
use grapher::prelude::{ElementType, GraphDisplayData};
use leptos::prelude::*;
use leptos::task::spawn_local;
use log::error;
use std::collections::HashMap;
use vowlr_sparql_queries::prelude::QueryAssembler;

use classes::{is_owl_class, is_rdf_class};
use filtergroup::FilterGroup;
use meta_filter::filter;
use properties::is_property;
use special_operators::is_set_operator;

fn update_graph(query: String, graph_data: RwSignal<GraphDisplayData>) {
    spawn_local(async move {
        let output_result = handle_internal_sparql(query).await;
        match output_result {
            Ok(new_graph_data) => {
                graph_data.set(new_graph_data.clone());
                let _ = EVENT_DISPATCHER
                    .rend_write_chan
                    .send(RenderEvent::LoadGraph(new_graph_data));
            }
            Err(e) => error!("{e}"),
        }
    });
}

#[component]
pub fn FilterMenu() -> impl IntoView {
    let GraphDataContext {
        graph_data,
        total_graph_data,
    } = expect_context::<GraphDataContext>();
    let element_counts = Memo::new(move |_| {
        let mut counts: HashMap<ElementType, usize> = HashMap::new();
        total_graph_data.with(|data| {
            for element in &data.elements {
                *counts.entry(*element).or_insert(0) += 1;
            }
        });
        counts
    });

    // let char_counts = Memo::new(move |_| {
    //     let mut counts: HashMap<Characteristic, usize> = HashMap::new();
    //     total_graph_data.with(|data| {
    //         for char_str in data.characteristics.values() {
    //             for part in char_str.split('\n') {
    //                 let c = match part.trim() {
    //                     "transitive" => Some(Characteristic::Transitive),
    //                     "functional" => Some(Characteristic::FunctionalProperty),
    //                     "inverse functional" => Some(Characteristic::InverseFunctionalProperty),
    //                     "symmetric" => Some(Characteristic::SymmetricProperty),
    //                     "asymmetric" => Some(Characteristic::AsymmetricProperty),
    //                     "reflexive" => Some(Characteristic::ReflexiveProperty),
    //                     "irreflexive" => Some(Characteristic::IrreflexiveProperty),
    //                     _ => None,
    //                 };
    //                 if let Some(characteristic) = c {
    //                     *counts.entry(characteristic).or_insert(0) += 1;
    //                 }
    //             }
    //         }
    //     });
    //     counts
    // });

    let element_checks = RwSignal::new(HashMap::new());

    element_checks.update_untracked(|map| {
        for elem in element_counts.read().keys() {
            map.insert(*elem, true);
        }
    });

    // let mut initial_char_checks = HashMap::new();
    // let all_chars = vec![
    //     Characteristic::Transitive,
    //     Characteristic::FunctionalProperty,
    //     Characteristic::InverseFunctionalProperty,
    //     Characteristic::ReflexiveProperty,
    //     Characteristic::IrreflexiveProperty,
    //     Characteristic::SymmetricProperty,
    //     Characteristic::AsymmetricProperty,
    // ];
    // for characteristic in &all_chars {
    //     initial_char_checks.insert(characteristic.clone(), true);
    // }
    // let (char_checks, set_char_checks) = signal(initial_char_checks);

    // 5. Characteristics
    // let characteristics = all_chars.clone();

    // Accordion State
    let open_owl = RwSignal::new(false);
    let open_rdf = RwSignal::new(false);
    let open_set_operations = RwSignal::new(false);
    let open_properties = RwSignal::new(false);
    // let (open_chars, set_open_chars) = signal(false);

    Effect::watch(
        move || element_checks.get(),
        move |checks, _, _| {
            let query =
                QueryAssembler::assemble_filtered_query(checks /* , char_checks.get()*/);
            leptos::logging::log!("{}", query);
            update_graph(query, graph_data);
        },
        false,
    );

    view! {
        <WorkbenchMenuItems title="Filter by Type">
            <div class="flex gap-2 items-center pb-3 mb-3 border-b">
                <button
                    class="text-sm text-blue-600 hover:text-blue-800"
                    on:click=move |_| {
                        let c = element_checks.get();
                        element_checks
                            .update(|map| {
                                let target = !c.values().all(|&v| v);
                                for (_, v) in map.iter_mut() {
                                    *v = target;
                                }
                            });
                    }
                >
                    {move || {
                        let all_elem = element_checks.get().values().all(|&v| v);
                        if all_elem { "Disable All" } else { "Enable All" }
                    }}
                </button>
            </div>

            <FilterGroup<
            ElementType,
        >
                name="OWL Classes"
                is_open=open_owl
                items=filter(
                    element_counts
                        .get()
                        .into_keys()
                        .collect::<Vec<_>>(),
                    &[is_owl_class],
                )
                checks=element_checks
                counts=element_counts
            />

            <FilterGroup<
            ElementType,
        >
                name="RDF"
                is_open=open_rdf
                items=filter(
                    element_counts
                        .get()
                        .into_keys()
                        .collect::<Vec<_>>(),
                    &[is_rdf_class],
                )
                checks=element_checks
                counts=element_counts
            />

            <FilterGroup<
            ElementType,
        >
                name="Set Operators"
                is_open=open_set_operations
                items=filter(
                    element_counts
                        .get()
                        .into_keys()
                        .collect::<Vec<_>>(),
                    &[is_set_operator],
                )
                checks=element_checks
                counts=element_counts
            />

            <FilterGroup<
            ElementType,
        >
                name="Properties"
                is_open=open_properties
                items=filter(
                    element_counts
                        .get()
                        .into_keys()
                        .collect::<Vec<_>>(),
                    &[is_property],
                )
                checks=element_checks
                counts=element_counts
            />

        // <FilterGroup
        // name="Characteristics"
        // is_open=open_chars
        // set_open=set_open_chars
        // items=characteristics
        // checks=char_checks.into()
        // set_checks=set_char_checks
        // counts=char_counts.into()
        // on_change=update_query
        // />
        </WorkbenchMenuItems>
    }
}
