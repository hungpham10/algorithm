use sycamore::prelude::*;
use web_sys::OrientationType;

#[component(inline_props)]
pub fn Main(orientation: Signal<Option<OrientationType>>) -> View {
    view! {}
}
