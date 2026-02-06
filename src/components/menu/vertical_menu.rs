use leptos::prelude::*;

/// A menu displaying `children` vertically.
///
/// Uses "relative" position to let children position themselves relatively to this element.
#[component]
pub fn VerticalMenu(children: Children) -> impl IntoView {
    view! {
        <div class="flex relative flex-col justify-between h-screen bg-white border-gray-100 w-fit border-e">
            <div class="py-6 px-4">
                <ul class="mt-6 space-y-1">{children()}</ul>
            </div>
        </div>
    }
}
