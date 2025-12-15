use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use web_sys::*;

use super::menu::{Context as MenuContext, Menu};
use super::Padding;

#[derive(Serialize, Deserialize, Clone)]
pub struct Signal {
    pub menu: MenuContext,
    pub padding: Padding,
}

#[component]
pub fn Header(
    orientation: ReadSignal<OrientationType>,
    features: ReadSignal<Signal>,
) -> impl IntoView {
    view! {
        <header class="header">
            <nav class=move || {
                format!(
                    "navbar navbar-expand-lg bg-light sticky-top border-bottom {}",
                    features.get().padding.page,
                )
            }>
                <Menu orientation=orientation features=features/>
            </nav>
            <div class="line-gradient rounded-pill"></div>
        </header>
    }
}
