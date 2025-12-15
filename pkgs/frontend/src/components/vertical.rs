use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use super::header::Signal as HeaderFeatures;

#[derive(Serialize, Deserialize, Clone)]
pub struct Item {
    pub name: String,
    pub link: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Classes {
    pub bounder: Vec<String>,
    pub button: Vec<String>,
    pub highlight: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Context {
    pub items: Vec<Item>,
    pub classes: Classes,
}

#[component]
pub fn Vertical(features: ReadSignal<HeaderFeatures>) -> impl IntoView {
    let current_path = Memo::new(move |_| {
        web_sys::window()
            .and_then(|w| w.location().pathname().ok())
            .unwrap_or_default()
    });

    let vertical = move || features.get().menu.vertical.clone();
    let items = move || vertical().map(|v| v.items).unwrap_or_default();

    let button_classes = move || {
        vertical()
            .as_ref()
            .and_then(|v| Some(v.classes.button.join(" ")))
            .unwrap_or_default()
    };
    let bounder_classes = move || {
        vertical()
            .as_ref()
            .and_then(|v| Some(v.classes.bounder.join(" ")))
            .unwrap_or_default()
    };
    let highlight_classes = move || {
        vertical()
            .as_ref()
            .and_then(|v| {
                Some(format!(
                    "{} {}",
                    v.classes.button.join(" "),
                    v.classes.highlight.join(" "),
                ))
            })
            .unwrap_or_default()
    };

    view! {
        <For
            each=items
            key=|item| (item.link.clone(), item.name.clone())
            children=move |item| {
                let href = item.link.clone();
                let name = item.name.clone();

                let bounder_class = bounder_classes();
                let button_class = if current_path.get() == href || (href == "/" && current_path.get() == "/") {
                    highlight_classes()
                } else {
                    button_classes()
                };

                view! {
                    <div class=bounder_class>
                        <a href=href.clone() class=button_class>
                            {name}
                        </a>
                    </div>
                }
            }
        />
    }
}
