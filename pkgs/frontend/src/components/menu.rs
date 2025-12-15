use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use web_sys::*;

use super::dropdown::{Context as DropdownContext, Dropdown};
use super::header::Signal as HeaderFeatures;
use super::logo::{Context as LogoContext, Logo};
use super::search::{Context as SearchContext, Search};
use super::vertical::{Context as VerticalContext, Vertical};

#[derive(Serialize, Deserialize, Clone)]
pub struct Context {
    pub logo: LogoContext,
    pub search: Option<SearchContext>,
    pub dropdown: Option<DropdownContext>,
    pub vertical: Option<VerticalContext>,
}

#[component]
pub fn Menu(
    orientation: ReadSignal<OrientationType>,
    features: ReadSignal<HeaderFeatures>,
) -> impl IntoView {
    view! {
        <div class="container-fluid d-flex align-items-center gap-5">
            <Logo features=features/>
            {move ||
                if features.get().menu.vertical.is_some() {
                    if orientation.get() == OrientationType::LandscapePrimary {
                        view! {
                            <Search features=features/>
                            <Vertical features=features/>
                        }
                        .into_any()
                    } else {
                        view! {
                            <Vertical features=features/>
                        }
                        .into_any()
                    }
                } else {
                    view! { }.into_any()
                }
            }
            {move ||
                if features.get().menu.dropdown.is_some() {
                    view! {
                        <Dropdown features=features/>
                    }
                    .into_any()
                } else {
                    view! { }.into_any()
                }
            }
        </div>
    }
}
