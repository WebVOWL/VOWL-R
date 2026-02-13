use super::WorkbenchMenuItems;
use leptos::prelude::*;

#[derive(Clone)]
pub struct ErrorLogContext {
    pub errors: RwSignal<Vec<String>>,
}

pub fn ErrorLog() -> impl IntoView {
    fn unescape_log(s: &str) -> String {
        s.replace("\\n", "\n").replace("\\t", "\t")
    }

    let error_log = expect_context::<ErrorLogContext>();

    view! {
        {move || {
            let errors = error_log.errors.get();
            view! {
                <div class="overflow-y-auto p-2 mt-2 bg-red-50 rounded border border-red-200 max-h-130">
                    {if errors.is_empty() {
                        view! { <p class="text-xs text-gray-600">"No errors"</p> }
                            .into_any()
                    } else {
                        view! {
                            <ul class="space-y-1 text-xs text-red-700">
                                {errors
                                    .into_iter()
                                    .map(|err| {
                                        let err = unescape_log(&err);
                                        view! {
                                            <li class="font-mono whitespace-pre-wrap">"• " {err}</li>
                                        }
                                    })
                                    .collect_view()}

                            </ul>
                        }
                            .into_any()
                    }}
                </div>
            }
                .into_any()
        }}
    }
}

#[component]
pub fn ErrorMenu() -> impl IntoView {
    view! {
        <WorkbenchMenuItems title="Error Log">
            <ErrorLog />
        </WorkbenchMenuItems>
    }
}
