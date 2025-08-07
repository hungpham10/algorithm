use leptos::*;
use leptos_router::{Route, Router, Routes};
use wasm_bindgen::prelude::wasm_bindgen;

#[component]
fn App() -> impl IntoView {
    view! {
        <Router>
            <h1>"My Leptos App"</h1>
            <main>
                <Routes>
                    <Route path="/" view=|| view! { <h2>"Welcome Home!"</h2> }/>
                    <Route path="/about" view=|| view! { <h2>"About Us"</h2> }/>
                </Routes>
            </main>
        </Router>
    }
}

pub fn main() {
    console_error_panic_hook::set_once();

    mount_to_body(App);
}
