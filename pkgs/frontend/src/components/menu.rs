use leptos::prelude::*;

use super::Features;

#[component]
pub fn Menu(features: ReadSignal<Features>) -> impl IntoView {
    view! {
        {
            if features.get().menu.len() > 0 {
                view! {
                    <div class="dropdown">
                        <div class="dropdown-button">"|||"</div>
                        <div class="dropdown-content" aria-labelledby="categoriesDropdown">
                        {
                            features.get().menu.into_iter()
                                .map(|item| view! {
                                    <div class="header-shadow-div border-1 rounded-5 w-45">
                                        <div class="px-3 pb-2 pt-2">{item}</div>
                                    </div>
                                })
                                .collect::<Vec<_>>()
                        }
                        </div>
                    </div>
                }
                .into_any()
            } else {
                view! { }.into_any()
            }
        }
    }
}
