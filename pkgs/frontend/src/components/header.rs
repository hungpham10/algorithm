use leptos::prelude::*;
use web_sys::*;

use super::contents::Contents;
use super::logo::Logo;
use super::menu::Menu;
use super::search::Search;
use super::Features;

#[component]
pub fn Header(
    orientation: ReadSignal<OrientationType>,
    features: ReadSignal<Features>,
) -> impl IntoView {
    view! {
        <header class="header">
            <nav class=move || {
                format!(
                    "navbar navbar-expand-lg bg-light sticky-top border-bottom {}",
                    features.get().padding.page,
                )
            }>
                <div class="container-fluid d-flex align-items-center gap-5">
                    <Logo features=features/>
                    {move ||
                        if orientation.get() == OrientationType::LandscapePrimary {
                            view! {
                                <Contents features=features/>
                                <Search features=features/>
                            }
                            .into_any()
                        } else {
                            view! { }.into_any()
                        }
                    }
                    <Menu features=features/>
                </div>
            </nav>
            <div class="line-gradient rounded-pill"></div>
        </header>
    }
}
