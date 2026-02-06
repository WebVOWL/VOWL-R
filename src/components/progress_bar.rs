use leptos::prelude::*;

#[component]
pub fn ProgressBar(
    #[prop(into)] progress: Signal<u64>,
    #[prop(into)] total: Signal<u64>,
) -> impl IntoView {
    let pro = move || {
        let t = *total.read();

        if t != 0 {
            (*progress.read() / t).clamp(0, 1) * 100
        } else {
            0
        }
    };

    view! {
        <div class="w-full bg-gray-200 rounded-full dark:bg-gray-700">
            <div
                class="p-0.5 text-xs font-medium leading-none text-center text-blue-100 bg-blue-600 rounded-full"
                style=move || { format!("width: {}%", pro()) }
            >
                {move || pro()}
                %
            </div>
        </div>
    }
}
