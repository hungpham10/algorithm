use sycamore::prelude::*;
use web_sys::OrientationType;

use super::logo::Logo;
use super::menu::Menu;
use super::search::Search;

#[component(inline_props)]
pub fn Header(orientation: Signal<Option<OrientationType>>) -> View {
    view! {
        header(class="header") {
            nav(class="navbar navbar-expand-lg bg-light sticky-top border-bottom") {
                div(class="container-fluid d-flex align-items-center gap-5 px-5 px-md-5") {
                    Logo()
                    Search(orientation=orientation)
                    Menu(orientation=orientation)
                }
            }
        }
    }
}
