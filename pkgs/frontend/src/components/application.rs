use anyhow::{anyhow, Result};

use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use web_sys::{window, OrientationType};

use super::footer::Footer;
use super::header::{Header, Signal as HeaderFeatures};
use super::logo::Context as LogoContext;
use super::main::{Main, Signal as MainFeatures};
use super::menu::Context as MenuContext;
use super::vertical::{
    Classes as VerticalClasses, Context as VerticalContext, Item as VerticalItem,
};
use super::Padding;

fn get_orientation() -> Result<OrientationType> {
    Ok(window()
        .ok_or(anyhow!("Can't get window object from DOM"))?
        .screen()
        .map_err(|error| anyhow!(format!("Can't get screen object: {:?}", error)))?
        .orientation()
        .type_()
        .map_err(|error| anyhow!(format!("Can't get orientation type: {:?}", error)))?)
}

#[component]
pub fn Application() -> impl IntoView {
    let (orientation, set_orientation) =
        signal(get_orientation().unwrap_or(OrientationType::LandscapePrimary));

    let (header, _) = signal(HeaderFeatures {
        menu: MenuContext {
            logo: LogoContext {
                link: "https://via.placeholder.com/150".to_string(),
            },
            search: None,
            dropdown: None,
            vertical: Some(VerticalContext {
                items: vec![
                    VerticalItem {
                        name: "Bán chạy".to_string(),
                        link: "/best-sellers".to_string(),
                    },
                    VerticalItem {
                        name: "Tất cả sản phẩm".to_string(),
                        link: "/products".to_string(),
                    },
                    VerticalItem {
                        name: "Về chúng mình".to_string(),
                        link: "/about-us".to_string(),
                    },
                ],
                classes: VerticalClasses {
                    button: vec!["btn", "px-3", "pb-2", "pt-2"]
                        .iter()
                        .map(|it| it.to_string())
                        .collect::<Vec<_>>(),
                    highlight: vec!["text-decoration-underline", "fw-bold"]
                        .iter()
                        .map(|it| it.to_string())
                        .collect::<Vec<_>>(),
                    bounder: vec!["rounded-lg", "overflow-hidden"]
                        .iter()
                        .map(|it| it.to_string())
                        .collect::<Vec<_>>(),
                },
            }),
        },
        padding: Padding {
            page: "px-page".to_string(),
        },
    });

    let (main, _) = signal(MainFeatures {
        padding: Padding {
            page: "px-page".to_string(),
        },
    });

    Effect::new(move |_| {
        if let Some(window) = web_sys::window() {
            let closure = Closure::wrap(Box::new(move || {
                set_orientation.set(get_orientation().unwrap_or(OrientationType::LandscapePrimary));
            }) as Box<dyn FnMut()>);

            window
                .add_event_listener_with_callback(
                    "orientationchange",
                    closure.as_ref().unchecked_ref(),
                )
                .expect("Failed to add orientationchange listener");
            closure.forget();
        }
    });

    view! {
        <Header orientation=orientation features=header/>
        <Main features=main/>
        <Footer />
    }
}
