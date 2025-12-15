use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use super::header::Signal as HeaderFeatures;

#[derive(Serialize, Deserialize, Clone)]
pub struct Button {
    pub content: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Context {
    pub items: Vec<String>,
    pub button: Button,
}

#[component]
pub fn Dropdown(features: ReadSignal<HeaderFeatures>) -> impl IntoView {
    if let Some(dropdown) = features.get().menu.dropdown {
        view! {
            {
                if dropdown.items.len() > 0 {
                    view! {
                        <div class="dropdown">
                            <div class="dropdown-button">"|||"</div>
                            <div class="dropdown-content" aria-labelledby="categoriesDropdown">
                            {
                                dropdown.items.into_iter()
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
    } else {
        view! {
            {
                view! { }.into_any()
            }
        }
    }
}
