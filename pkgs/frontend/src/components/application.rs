use anyhow::{anyhow, Result};
use sycamore::prelude::*;
use wasm_bindgen::prelude::*;
use web_sys::{window, Event, OrientationType};

use super::footer::Footer;
use super::header::Header;
use super::main::Main;

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
pub fn Application() -> View {
    let orientation = create_signal(None::<OrientationType>);

    create_effect(move || {
        if let Some(window) = window() {
            let update = Closure::wrap(Box::new(move |_: Event| {
                if let Ok(orient) = get_orientation() {
                    orientation.set(Some(orient));
                }
            }) as Box<dyn FnMut(_)>);

            if let Ok(value) = get_orientation() {
                orientation.set(Some(value));
            }

            window
                .add_event_listener_with_callback(
                    "orientationchange",
                    update.as_ref().unchecked_ref(),
                )
                .expect("failed to add orientationchange listener");

            update.forget();
        }
    });

    view! {
        Header(orientation=orientation)
        Main(orientation=orientation)
        Footer()
    }
}
