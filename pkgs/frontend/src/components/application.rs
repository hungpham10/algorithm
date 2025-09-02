use anyhow::{anyhow, Result};
use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use web_sys::{window, OrientationType};

use super::footer::Footer;
use super::header::Header;
use super::main::Main;
use super::{Features, Padding};

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

    let (features, _) = signal(Features {
        searchable: false,
        menu: vec![],
        logo: "https://via.placeholder.com/150".to_string(),
        contents: vec!["Bán chạy", "Tất cả sản phẩm", "Về chúng mình"]
            .iter()
            .map(|it| it.to_string())
            .collect::<Vec<_>>(),
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
        <Header orientation=orientation features=features/>
        <Main features=features/>
        <Footer/>
    }
}
