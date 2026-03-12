use crate::components::icon::MaybeShowIcon;
use leptos::{html::Div, prelude::*};

#[derive(Clone, Copy)]
struct ActiveListElement {
    active: RwSignal<Option<RwSignal<bool>>>,
}

#[component]
pub fn ListElementGroup(children: Children) -> impl IntoView {
    provide_context(ActiveListElement {
        active: RwSignal::new(None),
    });
    children()
}

/// A generic list element.
///
/// The `children` use an "absolute" position. Use the "relative" position on
/// any parent element higher in the DOM tree to position `children` relative to it.
#[component]
pub fn ListElement(
    #[prop(into)] title: Signal<String>,
    #[prop(optional, into)] icon: MaybeProp<icondata::Icon>,
    children: Children,
) -> impl IntoView {
    let show_element = RwSignal::new(false);
    let target = NodeRef::<Div>::new();
    let active_child = use_context::<ActiveListElement>();

    let open = move |_| {
        if let Some(child) = active_child {
            if let Some(prev) = child.active.get_untracked() {
                prev.set(false);
            }
            child.active.set(Some(show_element));
        }
        show_element.set(true);
    };

    let close = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        show_element.set(false);
        if let Some(child) = active_child {
            child.active.set(None);
        }
    };

    view! {
        <li on:click=open>
            <a
                href="#"
                class="flex gap-2 items-center py-2 px-4 text-gray-500 rounded-lg hover:text-gray-700 hover:bg-gray-100"
            >
                <MaybeShowIcon icon=icon></MaybeShowIcon>
                <span class="text-sm font-medium">{move || title.get()}</span>
            </a>
            <div
                node_ref=target
                class="overflow-y-scroll absolute top-0 left-full m-4 bg-white border-gray-100 w-fit max-h-[80vh] min-h-[80vh]"
                style=move || {
                    if show_element.get() { "" } else { "display: none" }
                }
            >
                <button
                    class="inline-flex absolute top-0 right-0 justify-center items-center p-0.5 text-gray-400 bg-white rounded-md hover:text-gray-500 hover:bg-gray-100 focus:outline-none cursor-pointer mt-2 mr-2"
                    on:click=close
                >
                    <svg
                        class="w-6 h-6"
                        xmlns="http://www.w3.org/2000/svg"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                        aria-hidden="true"
                    >
                        <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M6 18L18 6M6 6l12 12"
                        />
                    </svg>
                </button>
                {children()}
            </div>
        </li>
    }
}

/// A list with a dropdown button containing children.
#[component]
pub fn ListDetails(
    #[prop(into)] title: String,
    #[prop(optional, into)] icon: MaybeProp<icondata::Icon>,
    children: Children,
) -> impl IntoView {
    view! {
        <li>
            <details class="group [&amp;_summary::-webkit-details-marker]:hidden">
                <summary class="flex justify-between items-center py-2 px-4 text-gray-500 rounded-lg cursor-pointer hover:text-gray-700 hover:bg-gray-100">
                    <div class="flex gap-2 items-center">
                        <MaybeShowIcon icon=icon></MaybeShowIcon>
                        <span class="text-sm font-medium">{title}</span>
                    </div>

                    <span class="transition duration-300 shrink-0 group-open:-rotate-180">
                        <MaybeShowIcon icon=icondata::BiChevronDownRegular></MaybeShowIcon>
                    </span>
                </summary>

                <ul class="px-4 mt-2 space-y-1">{children()}</ul>
            </details>
        </li>
    }
}
