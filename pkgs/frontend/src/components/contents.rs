use leptos::prelude::*;

use super::Features;

#[component]
pub fn Contents(features: ReadSignal<Features>) -> impl IntoView {
    view! {
        {
            features.get().contents.into_iter()
                .map(|content| view! {
                    <div class="header-shadow-div border-1 rounded-5 w-45">
                        <div class="btn px-3 pb-2 pt-2">{content}</div>
                    </div>
                })
                .collect::<Vec<_>>()
        }
    }
}
