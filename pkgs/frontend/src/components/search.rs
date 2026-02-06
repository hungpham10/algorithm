use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use super::header::Signal as HeaderFeatures;

#[derive(Serialize, Deserialize, Clone)]
pub struct Context {}

#[component]
pub fn Search(features: ReadSignal<HeaderFeatures>) -> impl IntoView {
    view! {
        {
            move || if features.get().menu.search.is_some() {
                view! {
                    <div class="border border-primary border-1 p-1 rounded-1 w-50 w-md-75 w-sm-100">
                        <form class="input-group">
                            <input class="form-control form-control-sm border-0 shadow-none" placeholder="Search for anything" aria-label="Search"/>
                            <div class="input-group-append">
                                <button class="btn btn-sm btn-outline-secondary">"Search"</button>
                            </div>
                        </form>
                    </div>
                }.into_any()
            } else {
                view! { }.into_any()
            }
        }
    }
}
