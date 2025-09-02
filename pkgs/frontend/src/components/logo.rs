use leptos::prelude::*;

use super::Features;

#[component]
pub fn Logo(features: ReadSignal<Features>) -> impl IntoView {
    view! {
        <div>
            <img
                class="navbar-brand"
                src=move|| { features.get().logo }
            />
        </div>
    }
}
