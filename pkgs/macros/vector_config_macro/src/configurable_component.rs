use proc_macro::TokenStream;
use quote::quote;
use std::collections::HashSet;
use syn::{
    Data, DeriveInput, Meta, Token, parse_macro_input, punctuated::Punctuated, spanned::Spanned,
};

pub enum ComponentType {
    Source,
    Sink,
    Transform,
}

fn to_snake_case(s: &str) -> String {
    let mut snake = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i != 0 {
                snake.push('_');
            }
            snake.push(ch.to_ascii_lowercase());
        } else {
            snake.push(ch);
        }
    }
    snake
}

pub fn configurable_component_impl(
    attr: TokenStream,
    item: TokenStream,
    kind: ComponentType,
    requirements: Vec<&str>,
) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let name = &input.ident;

    // 1. Parse Attribute Arguments (e.g., derives(Clone, Debug))
    let mut extra_derives = Vec::new();
    if !attr.is_empty() {
        let attr_args =
            parse_macro_input!(attr with Punctuated::<Meta, Token![,]>::parse_terminated);
        for meta in attr_args {
            if meta.path().is_ident("derive") && let Meta::List(list) = meta {
                // Parse nội dung bên trong dấu ngoặc của derives(...)
                let nested = list
                    .parse_args_with(Punctuated::<syn::Path, Token![,]>::parse_terminated)
                    .expect("Failed to parse derives content");
                for path in nested {
                    extra_derives.push(path);
                }
            }
        }
    }

    // 2. Validate Required Fields
    let fields = if let Data::Struct(ref data) = input.data {
        &data.fields
    } else {
        return syn::Error::new(input.span(), "Struct is required to use with this macro")
            .to_compile_error()
            .into();
    };

    let existing_fields = fields
        .iter()
        .filter_map(|f| f.ident.as_ref().map(|id| id.to_string()))
        .collect::<HashSet<_>>();

    let missing_fields = requirements
        .iter()
        .filter(|&&req| !existing_fields.contains(req))
        .map(|&req| req.to_string())
        .collect::<Vec<_>>();

    if !missing_fields.is_empty() {
        return syn::Error::new(
            name.span(),
            format!(
                "Struct '{}' missing required fields: [{}]",
                name,
                missing_fields.join(", ")
            ),
        )
        .to_compile_error()
        .into();
    }

    // 3. Generate Output
    let component_name_str = to_snake_case(&(name.to_string()));
    let macro_name = quote::format_ident!("impl_{}", component_name_str);

    // Chuẩn bị token cho các derive bổ sung
    let extra_derive_tokens = if extra_derives.is_empty() {
        quote! {}
    } else {
        quote! { , #(#extra_derives),* }
    };

    // Định nghĩa logic cho từng loại Component
    let (ident_impl, type_enum) = match kind {
        ComponentType::Source => (
            quote! {
                fn get_inputs(&self) -> Option<&Vec<String>> { None }
            },
            quote! { vector_runtime::ComponentType::Source },
        ),
        ComponentType::Sink => (
            quote! {
                fn get_inputs(&self) -> Option<&Vec<String>> { Some(&self.inputs) }
            },
            quote! { vector_runtime::ComponentType::Sink },
        ),
        ComponentType::Transform => (
            quote! {
                fn get_inputs(&self) -> Option<&Vec<String>> { Some(&self.inputs) }
            },
            quote! { vector_runtime::ComponentType::Transform },
        ),
    };

    let output = quote! {
        #[derive(serde::Serialize, serde::Deserialize #extra_derive_tokens)]
        #[serde(deny_unknown_fields)]
        #input

        impl Identify for #name {
            fn id(&self) -> String {
                self.id.clone()
            }

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            #ident_impl
            fn component_type(&self) -> vector_runtime::ComponentType {
                #type_enum
            }

            fn compare(&self, other: &dyn Component) -> bool {
                if let Some(other_concrete) = other.as_any().downcast_ref::<#name>() {
                    // Lưu ý: User phải tự impl PartialEq hoặc truyền vào derives(...)
                    self == other_concrete
                } else {
                    false
                }
            }
        }

        #[macro_export]
        macro_rules! #macro_name {
            ($($tt:tt)*) => {
                #[typetag::serde(name = #component_name_str)]
                #[async_trait::async_trait]
                impl Component for #name {
                    $($tt)*
                }
            };
        }
    };

    TokenStream::from(output)
}
