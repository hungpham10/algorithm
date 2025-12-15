use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use super::Padding;

#[derive(Serialize, Deserialize, Clone)]
pub struct Signal {
    pub padding: Padding,
}

#[component]
pub fn Main(features: ReadSignal<Signal>) -> impl IntoView {
    view! {
        <main class=move ||{ format!("main pt-1 {}", features.get().padding.page) }>
        </main>
    }
}
