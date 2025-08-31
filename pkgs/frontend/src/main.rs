use sycamore::prelude::*;

#[component]
fn Products() -> View {
    view! {}
}

mod components;

fn main() {
    console_error_panic_hook::set_once();

    sycamore::render(components::Application);
}
