use ::lib::cmds::{chat::chat, ggcolab::ggcolab, collect::collect, farm::farm};

fn main() {
    dotenvy::dotenv().ok();

    let _guard = sentry::init((
        std::env::var("SENTRY_DSN").unwrap(),
        sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        },
    ));

    match std::env::args().nth(1).unwrap().as_str() {
        "collect" => collect(),
        "chat" => chat(),
        "ggcolab" => ggcolab(),
        "farm" => farm(),
        unknown => todo!("not yet implement {}", unknown),
    }.unwrap();
}
