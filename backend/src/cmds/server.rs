#[actix_rt::main]
pub async fn bff() -> std::io::Result<()> {
    super::Application::new()
        .await
        .handon_bff_server(false, 3000)
        .await
}

#[actix_rt::main]
pub async fn auth() -> std::io::Result<()> {
    super::Application::new()
        .await
        .handon_auth_server(3000)
        .await
}

#[actix_rt::main]
pub async fn render() -> std::io::Result<()> {
    super::Application::new()
        .await
        .handon_render_server(3000)
        .await
}

#[actix_rt::main]
pub async fn event() -> std::io::Result<()> {
    super::Application::new()
        .await
        .handon_event_server(3000)
        .await
}

#[actix_rt::main]
pub async fn datasource() -> std::io::Result<()> {
    super::Application::new()
        .await
        .handon_datasource_server(3000)
        .await
}

#[actix_rt::main]
pub async fn server() -> std::io::Result<()> {
    super::Application::new()
        .await
        .handon_bff_server(true, 3000)
        .await
}
