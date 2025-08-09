#[cfg(feature = "landing-page")]
mod tests {
    use sycamore::prelude::*;

    #[component]
    fn TestComponent() -> View {
        let count = create_signal(0);
        view! {
            div {
                p { "Đếm: " (count.get()) }
                button(on:click=move |_| count.set(count.get() + 1)) { "Tăng" }
            }
        }
    }

    #[test]
    fn test_ssr_render() {
        let html = sycamore::render_to_string(|| view! { TestComponent() });

        println!("{}", html);
        assert_eq!(html.contains("Đếm: "), true);
        assert_eq!(html.contains("Tăng"), true);
    }
}
