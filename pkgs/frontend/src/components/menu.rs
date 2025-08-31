use sycamore::prelude::*;
use web_sys::OrientationType;

#[component(inline_props)]
pub fn Menu(orientation: Signal<Option<OrientationType>>) -> View {
    let categories = vec!["Action", "Another action"]; // Sample data

    view! {
        div(class="dropdown") {
            div(class="dropdown-button") { "|||" }
            div(class="dropdown-content", aria-labelledby="categoriesDropdown") {
                Indexed(
                    list=categories,
                    view=|category| view! {
                        a {
                            (category)
                        }
                    },
                )
            }
        }
    }
}
