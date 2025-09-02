use leptos::prelude::*;

use super::Features;

#[component]
pub fn Main(features: ReadSignal<Features>) -> impl IntoView {
    view! {
        <main class=move ||{ format!("main pt-1 {}", features.get().padding.page) }>

        </main>
    }
}
