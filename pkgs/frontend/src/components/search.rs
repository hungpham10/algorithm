use sycamore::prelude::*;
use web_sys::OrientationType;

#[component(inline_props)]
pub fn Search(orientation: Signal<Option<OrientationType>>) -> View {
    // @TODO: chỗ này nếu đổi ra dạng màn hình portable thì sẽ đổi
    //        sang icon search thôi, như vậy tối ưu diện tích

    view! {
        (match orientation.get() {
            Some(OrientationType::LandscapePrimary) => view! {
                div(class="border border-primary border-1 p-1 rounded-1 w-50 w-md-75 w-sm-100") {
                    form(class="input-group") {
                        input(class="form-control form-control-sm border-0 shadow-none", placeholder="Search for anything", aria-label="Search") { }
                        div(class="input-group-append") {
                            button(class="btn btn-sm btn-outline-secondary") { "Search" }
                        }
                    }
                }
            },
            _ => view! {},
        })
    }
}
