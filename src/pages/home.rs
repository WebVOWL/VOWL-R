use crate::blocks::{right_side_bar::RightSidebar, workbench::NewWorkbench};
use leptos::prelude::*;
use leptos_meta::Title;

#[component]
pub fn Home() -> impl IntoView {
    view! {
        <Title text="VOWL-R" />
        <main class="-z-99">
            <canvas class="fixed -z-98 size-full" id="canvas" />
            <NewWorkbench />
            <RightSidebar />
        </main>
    }
}
