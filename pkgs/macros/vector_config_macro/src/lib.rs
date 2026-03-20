mod configurable_component;

use configurable_component::*;
use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn sink(attr: TokenStream, item: TokenStream) -> TokenStream {
    configurable_component_impl(attr, item, ComponentType::Sink, vec!["id"])
}

#[proc_macro_attribute]
pub fn source(attr: TokenStream, item: TokenStream) -> TokenStream {
    configurable_component_impl(attr, item, ComponentType::Source, vec!["id"])
}

#[proc_macro_attribute]
pub fn transform(attr: TokenStream, item: TokenStream) -> TokenStream {
    configurable_component_impl(attr, item, ComponentType::Transform, vec!["id", "inputs"])
}
