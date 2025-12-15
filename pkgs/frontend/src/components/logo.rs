use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use super::header::Signal as HeaderFeatures;

#[derive(Serialize, Deserialize, Clone)]
pub struct Context {
    pub link: String,
}

#[component]
pub fn Logo(features: ReadSignal<HeaderFeatures>) -> impl IntoView {
    view! {
        <div>
            <img
                class="navbar-brand"
                src=move|| { features.get().menu.logo.link }
            />
        </div>
    }
}
