use std::sync::Arc;

#[actix_rt::main]
pub async fn graphql_server() -> std::io::Result<()> {
    let app = super::Application::new().await;

    app.start_cron(Vec::new())
        .await;
    app.handon_bff_server(3000)
        .await
}

#[actix_rt::main]
pub async fn sql_server() -> std::io::Result<()> {
    let app = super::Application::new().await;
    let capacity = std::env::var("SQL_CAPACITY")
            .unwrap_or_else(|_| "0".to_string())
            .parse()
            .unwrap_or(0);

    app.start_cron(Vec::new()).await;

    app.handon_sql_server(3001, capacity)
        .await
}

#[actix_rt::main]
pub async fn monolith_server() -> std::io::Result<()> {
    // @NOTE: configure application
    let app = Arc::new(super::Application::new().await);
    let capacity = std::env::var("SQL_CAPACITY")
            .unwrap_or_else(|_| "0".to_string())
            .parse()
            .unwrap_or(0);

    // @NOTE: start cron first
    app.start_cron(Vec::new()).await;

    let sql_server = app.clone();
    let http_server = app.clone();

    actix_rt::spawn(async move {
        sql_server
            .handon_sql_server(5432, capacity)
            .await
    });

    http_server
        .handon_bff_server(3000)
        .await
}
