use sycamore::prelude::*;

#[component]
pub fn Logo() -> View {
    view! {
        div {
            a(class="navbar-brand", href="#") {
                "Branch"
            }
        }
    }
}
