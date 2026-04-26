use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, ErrorKind, Write};
use std::path::PathBuf;

use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};

use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::{IntoParams, OpenApi, ToSchema};

use axum::Router;
use axum::body::Body;
use axum::extract::{DefaultBodyLimit, Json as JsonRequest, Multipart, Path, Query, State};
use axum::http::{self, header};
use axum::response::{IntoResponse, Json as JsonResponse, Response};
use axum::routing::{get, post};

use aws_sdk_s3::primitives::ByteStream;
use chrono::Utc;
use http::StatusCode;
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tokio_util::io::ReaderStream;

use integration::components::appended_log::AppendedLog;
use models::cache::Cache;
use models::entities::admin::{Api, Article, Site, Table};

use crate::api::AppState;
use crate::api::admin::AdminHeaders;

#[derive(ToSchema)]
pub struct AdminFileUpload {
    #[schema(format = Binary)]
    #[allow(dead_code)]
    pub file: Vec<u8>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, IntoParams, ToSchema)]
pub struct QueryPagingInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, ToSchema)]
pub struct ListApiSchema {
    data: Vec<Api>,
    next_after: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, ToSchema)]
pub struct ListTableSchema {
    data: Vec<Table>,
    next_after: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, ToSchema)]
pub struct AdminResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    apis: Option<ListApiSchema>,

    #[serde(skip_serializing_if = "Option::is_none")]
    api: Option<Api>,

    #[serde(skip_serializing_if = "Option::is_none")]
    tables: Option<ListTableSchema>,

    #[serde(skip_serializing_if = "Option::is_none")]
    query: Option<Vec<JsonValue>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(OpenApi)]
#[openapi(
    // 1. List all the handler functions from your SEO/Admin module
    paths(
        get_tenant_id,
        publish_news,
        publish_site,
        fetch_file,
        purge_file,
        list_paginated_api_schemas,
        create_api_schemas,
        get_api_schema,
        query_data_from_api,
        list_paginated_table_schemas,
        create_table_schemas,
        get_token,
        put_token,
        get_appended_log,
        rotate_new_partition,
        build_sitemap_xml,
        build_news_xml
    ),
    // 2. Register all the data structures used in requests/responses
    components(
        schemas(
            QueryPagingInput,
            AdminResponse,
            ListApiSchema,
            PutTokenPayload,
            // Ensure these external models also derive ToSchema
            models::entities::admin::Api,
            models::entities::admin::Site,
            models::entities::admin::Article
        )
    ),
    // 3. Organize them under a clear Tag
    tags(
        (name = "Admin V1", description = "SEO Engine, Sitemap Management, and System Logs")
    ),
    modifiers(&SecurityAddon) // Add the SecurityAddon for x-tenant-id
)]
pub struct AdminV1Api;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "admin_auth",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("x-tenant-id"))),
            )
        }
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/seo/tenant/{host}/id", get(get_tenant_id))
        .route("/seo/news", get(build_news_xml).post(publish_news))
        .route("/seo/sitemap", get(build_sitemap_xml).post(publish_site))
        .route("/seo/files/{*path}", get(fetch_file).head(purge_file))
        .route(
            "/seo/schemas",
            get(list_paginated_api_schemas).post(create_api_schemas),
        )
        .route("/seo/schemas/{id}/metadata", get(get_api_schema))
        .route("/seo/schemas/{id}/query/{*path}", post(query_data_from_api))
        .route(
            "/seo/tables",
            get(list_paginated_table_schemas).post(create_table_schemas),
        )
        .route("/seo/tokens/{name}", get(get_token).post(put_token))
        .route(
            "/seo/logs/{name}",
            get(get_appended_log).patch(rotate_new_partition),
        )
        .layer(DefaultBodyLimit::max(512 * 1024 * 1024))
    //.route("/seo/features",
    //    get(get_features)
    //        .put(configure_features)
    //)
}

#[utoipa::path(
    post,
    path = "/seo/sitemap",
    request_body(
        content = AdminFileUpload,
        content_type = "multipart/form-data"
    ),
    responses(
        (status = 200, description = "Sitemap uploaded successfully"),
        (status = 400, description = "Invalid multipart data", body = AdminResponse),
        (status = 500, description = "S3 or DB error", body = AdminResponse)
    ),
    security(
        ("admin_auth" = [])
    ),
    tag = "SEO Engine"
)]
async fn publish_site(
    State(app_state): State<AppState>,
    AdminHeaders { tenant_id, .. }: AdminHeaders,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, (StatusCode, JsonResponse<AdminResponse>)> {
    let tenant_id_i64: i64 = tenant_id.into();
    let mut sites_to_save = Vec::new();

    let bucket = app_state.secret.get("S3_BUCKET", "/").await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(AdminResponse {
                error: Some("S3_BUCKET not set".into()),
                ..Default::default()
            }),
        )
    })?;

    while let Ok(Some(field)) = multipart.next_field().await {
        let file_name = field.file_name().unwrap_or("untitled.xml").to_string();
        let content_type = field
            .content_type()
            .unwrap_or("application/xml")
            .to_string();
        let data = field.bytes().await.map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                JsonResponse(AdminResponse {
                    error: Some(format!("Field read error: {}", e)),
                    ..Default::default()
                }),
            )
        })?;

        let s3_key = format!("{}/sitemaps/{}", tenant_id_i64, file_name);

        app_state
            .s3
            .put_object()
            .bucket(&bucket)
            .key(&s3_key)
            .body(ByteStream::from(data))
            .content_type(content_type)
            .send()
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    JsonResponse(AdminResponse {
                        error: Some(format!("S3 error: {}", e)),
                        ..Default::default()
                    }),
                )
            })?;

        sites_to_save.push(Site {
            loc: s3_key,
            last_mod: Utc::now(),
            freq: "daily".to_string(),
            ..Default::default()
        });
    }

    if !sites_to_save.is_empty() {
        app_state
            .admin_entity
            .insert_or_update_sites(tenant_id_i64, sites_to_save)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    JsonResponse(AdminResponse {
                        error: Some(format!("DB error: {}", e)),
                        ..Default::default()
                    }),
                )
            })?;
    }

    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/seo/news",
    request_body(
        content = AdminFileUpload,
        content_type = "multipart/form-data"
    ),
    responses(
        (status = 200, description = "News published successfully"),
        (status = 400, description = "Invalid multipart request", body = AdminResponse),
        (status = 500, description = "Internal server error (S3 or DB)", body = AdminResponse)
    ),
    security(
        ("admin_auth" = [])
    ),
    tag = "SEO Engine"
)]
async fn publish_news(
    State(app_state): State<AppState>,
    AdminHeaders { tenant_id, .. }: AdminHeaders,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, (StatusCode, JsonResponse<AdminResponse>)> {
    let tenant_id_i64: i64 = tenant_id.into();
    let mut articles_to_save = Vec::new();

    let bucket = app_state.secret.get("S3_BUCKET", "/").await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(AdminResponse {
                error: Some("S3_BUCKET not set".into()),
                ..Default::default()
            }),
        )
    })?;

    while let Ok(Some(field)) = multipart.next_field().await {
        let file_name = field.file_name().unwrap_or("news.xml").to_string();
        let data = field.bytes().await.map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                JsonResponse(AdminResponse {
                    error: Some(format!("Field read error: {}", e)),
                    ..Default::default()
                }),
            )
        })?;

        let s3_key = format!("{}/news/{}", tenant_id_i64, file_name);

        app_state
            .s3
            .put_object()
            .bucket(&bucket)
            .key(&s3_key)
            .body(ByteStream::from(data))
            .content_type("application/xml")
            .send()
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    JsonResponse(AdminResponse {
                        error: Some(format!("S3 error: {}", e)),
                        ..Default::default()
                    }),
                )
            })?;

        articles_to_save.push(Article {
            loc: s3_key,
            published_at: Utc::now(),
            title: file_name,
            ..Default::default()
        });
    }

    if !articles_to_save.is_empty() {
        app_state
            .admin_entity
            .insert_or_update_acticles(tenant_id_i64, articles_to_save)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    JsonResponse(AdminResponse {
                        error: Some(format!("DB error: {}", e)),
                        ..Default::default()
                    }),
                )
            })?;
    }

    Ok(StatusCode::OK)
}

#[utoipa::path(
    get,
    path = "/seo/tokens/{name}",
    params(
        ("name" = String, Path, description = "The unique name/identifier of the token")
    ),
    responses(
        (status = 200, description = "Returns the unencrypted token value", body = String),
        (status = 500, description = "Internal Server Error - usually if the token is missing or DB fails", body = String)
    ),
    security(
        ("admin_auth" = [])
    ),
    tag = "Tokens"
)]
async fn get_token(
    State(app_state): State<AppState>,
    Path(name): Path<String>,
    AdminHeaders { tenant_id, .. }: AdminHeaders,
) -> impl IntoResponse {
    match app_state
        .admin_entity
        .get_unencrypted_token(tenant_id.into(), &name)
        .await
    {
        Ok(token) => (StatusCode::OK, token),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Fail getting token: {error}"),
        ),
    }
}

#[derive(Deserialize, ToSchema, IntoParams)]
struct PutTokenPayload {
    name: String,
    token: String,
}

#[utoipa::path(
    post,
    path = "/seo/tokens/{name}",
    params(
        ("name" = String, Path, description = "Token identifier")
    ),
    request_body = PutTokenPayload,
    responses(
        (status = 200, description = "Token updated successfully"),
        (status = 500, description = "Internal Server Error")
    ),
    security(
        ("admin_auth" = [])
    ),
    tag = "Tokens"
)]
async fn put_token(
    State(app_state): State<AppState>,
    AdminHeaders { tenant_id, .. }: AdminHeaders,
    JsonRequest(PutTokenPayload { name, token }): JsonRequest<PutTokenPayload>,
) -> impl IntoResponse {
    match app_state
        .admin_entity
        .put_unencrypted_token(tenant_id.into(), &name, &token)
        .await
    {
        Ok(_) => Ok(StatusCode::OK),
        Err(error) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Fail put new token {name}: {error}"),
        )),
    }
}

#[utoipa::path(
    get,
    path = "/seo/tenant/{host}/id",
    params(
        ("host" = String, Path, description = "The hostname to look up")
    ),
    responses(
        (status = 200, description = "Returns the Tenant ID", body = String),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "Tenant"
)]
async fn get_tenant_id(
    State(app_state): State<AppState>,
    Path(host): Path<String>,
) -> impl IntoResponse {
    match app_state.admin_entity.get_tenant_id(&host).await {
        Ok(response) => (StatusCode::OK, format!("{}", response)),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Fail to get tenant of {}: {:?}", host, error),
        ),
    }
}

//async fn get_features(
//    State(app_state): State<AppState>,
//    AdminHeaders { tenant_id, .., }: AdminHeaders,
//) -> impl IntoResponse {
//}

fn write_tag<W: Write>(
    writer: &mut Writer<W>,
    tag: &str,
    text: &str,
) -> Result<(), (StatusCode, String)> {
    writer
        .write_event(Event::Start(BytesStart::new(tag)))
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Fail to write: {}", error),
            )
        })?;
    writer
        .write_event(Event::Text(BytesText::new(text)))
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Fail to write: {}", error),
            )
        })?;
    writer
        .write_event(Event::End(BytesEnd::new(tag)))
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Fail to write: {}", error),
            )
        })?;
    Ok(())
}

#[utoipa::path(
    get,
    path = "/seo/sitemap",
    responses(
        (
            status = 200,
            description = "Returns a standard XML sitemap (Sitemaps.org 0.9)",
            content_type = "application/xml"
        ),
        (status = 500, description = "Database error or XML generation error", body = String)
    ),
    security(
        ("admin_auth" = [])
    ),
    tag = "SEO Engine"
)]
async fn build_sitemap_xml(
    State(app_state): State<AppState>,
    AdminHeaders { tenant_id, .. }: AdminHeaders,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let tenant_id = tenant_id.into();
    let sites = app_state
        .admin_entity
        .list_sites(tenant_id)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Fail to list sites: {}", error),
            )
        })?;

    if sites.is_empty() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Sitemap is empty for tenant_id {}", tenant_id),
        ));
    }

    let mut buffer = Cursor::new(Vec::new());
    let mut writer = Writer::new(&mut buffer);

    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Fail to write sitemap: {}", error),
            )
        })?;

    let mut urlset = BytesStart::new("urlset");
    urlset.push_attribute(("xmlns", "http://www.sitemaps.org/schemas/sitemap/0.9"));
    writer.write_event(Event::Start(urlset)).map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Fail to write sitemap: {}", error),
        )
    })?;

    for site in sites {
        writer
            .write_event(Event::Start(BytesStart::new("url")))
            .map_err(|error| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Fail to write sitemap: {}", error),
                )
            })?;

        // Viết loc, lastmod, freq, priority (tương tự code cũ)
        write_tag(&mut writer, "loc", &site.loc)?;
        write_tag(
            &mut writer,
            "lastmod",
            &site.last_mod.format("%Y-%m-%d").to_string(),
        )?;
        write_tag(&mut writer, "changefreq", &site.freq)?;
        write_tag(&mut writer, "priority", &site.priority.to_string())?;

        writer
            .write_event(Event::End(BytesEnd::new("url")))
            .map_err(|error| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Fail to write sitemap: {}", error),
                )
            })?;
    }

    writer
        .write_event(Event::End(BytesEnd::new("urlset")))
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Fail to write sitemap: {}", error),
            )
        })?;

    let xml = String::from_utf8(buffer.into_inner()).map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Fail to write sitemap: {}", error),
        )
    })?;

    Response::builder()
        .header(header::CONTENT_TYPE, "application/xml; charset=utf-8")
        .body(Body::from(xml))
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Fail to write sitemap: {}", error),
            )
        })
}

#[utoipa::path(
    get,
    path = "/seo/news",
    responses(
        (
            status = 200,
            description = "Returns a Google News compatible XML sitemap",
            content_type = "application/xml"
        ),
        (status = 500, description = "Database or XML generation error", body = String)
    ),
    security(
        ("admin_auth" = [])
    ),
    tag = "SEO Engine"
)]
async fn build_news_xml(
    State(app_state): State<AppState>,
    AdminHeaders { tenant_id, .. }: AdminHeaders,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let articles = app_state
        .admin_entity
        .list_articles(tenant_id.into())
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Fail to list articles: {}", error),
            )
        })?;

    let mut buffer = Cursor::new(Vec::new());
    let mut writer = Writer::new(&mut buffer);

    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Fail to write news: {}", error),
            )
        })?;

    let mut urlset = BytesStart::new("urlset");
    urlset.push_attribute(("xmlns", "http://www.sitemaps.org/schemas/sitemap/0.9"));
    urlset.push_attribute((
        "xmlns:news",
        "http://www.google.com/schemas/sitemap-news/0.9",
    ));
    writer.write_event(Event::Start(urlset)).map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Fail to write news: {}", error),
        )
    })?;

    for article in articles {
        writer
            .write_event(Event::Start(BytesStart::new("url")))
            .map_err(|error| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Fail to write news: {}", error),
                )
            })?;
        write_tag(&mut writer, "loc", &article.loc)?;

        writer
            .write_event(Event::Start(BytesStart::new("news")))
            .map_err(|error| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Fail to write news: {}", error),
                )
            })?;
        writer
            .write_event(Event::Start(BytesStart::new("publication")))
            .map_err(|error| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Fail to write news: {}", error),
                )
            })?;
        write_tag(&mut writer, "name", &article.name)?;
        write_tag(&mut writer, "language", &article.language)?;
        writer
            .write_event(Event::End(BytesEnd::new("publication")))
            .map_err(|error| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Fail to write news: {}", error),
                )
            })?;

        write_tag(
            &mut writer,
            "publication_date",
            &article.published_at.format("%Y-%m-%d").to_string(),
        )?;
        write_tag(&mut writer, "title", &article.title)?;

        if let Some(keywords) = article.keywords {
            write_tag(&mut writer, "keywords", &keywords)?;
        }

        writer
            .write_event(Event::End(BytesEnd::new("news")))
            .map_err(|error| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Fail to write news: {}", error),
                )
            })?;
        writer
            .write_event(Event::End(BytesEnd::new("url")))
            .map_err(|error| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Fail to write news: {}", error),
                )
            })?;
    }

    writer
        .write_event(Event::End(BytesEnd::new("urlset")))
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Fail to write news: {}", error),
            )
        })?;

    let xml = String::from_utf8(buffer.into_inner()).map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Fail to write news: {}", error),
        )
    })?;

    Response::builder()
        .header(header::CONTENT_TYPE, "application/xml; charset=utf-8")
        .body(Body::from(xml))
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Fail to write news: {}", error),
            )
        })
}

#[utoipa::path(
    // We use 'head' because your router.route().head(purge_file) uses the HEAD method
    head,
    path = "/seo/files/{path}",
    params(
        ("path" = String, Path, description = "The full S3 object path to purge from local cache"),
        ("x-host" = String, Header, description = "The hostname associated with the file")
    ),
    responses(
        (status = 200, description = "File successfully purged from Nginx cache or was not found"),
        (status = 500, description = "Internal server error while accessing the filesystem")
    ),
    tag = "SEO Engine"
)]
pub async fn purge_file(
    Path(path): Path<String>,
    AdminHeaders { host, .. }: AdminHeaders,
) -> impl IntoResponse {
    let mut hasher = Md5::new();

    hasher.update(
        format!(
            "{}{}",
            host,
            if path.starts_with('/') {
                path
            } else {
                format!("/{}", path)
            },
        )
        .as_bytes(),
    );

    let hash = format!("{:x}", hasher.finalize());

    let mut file_path = PathBuf::from("/var/cache/nginx");
    file_path.push(&hash[hash.len() - 1..]);
    file_path.push(&hash[hash.len() - 3..hash.len() - 1]);
    file_path.push(&hash);

    match fs::remove_file(&file_path) {
        Ok(_) => StatusCode::OK,
        Err(error) => {
            if error.kind() == ErrorKind::NotFound {
                StatusCode::OK
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

#[utoipa::path(
    get,
    path = "/seo/files/{path}",
    params(
        ("path" = String, Path, description = "Full path of the file in S3"),
        ("x-tenant-id" = i64, Header, description = "The unique tenant identifier"),
        ("x-host" = String, Header, description = "The hostname for cache key generation")
    ),
    responses(
        (status = 200, description = "Returns the file stream from S3", content_type = "application/octet-stream"),
        (status = 500, description = "S3 or Configuration error", body = AdminResponse)
    ),
    tag = "SEO Engine"
)]
pub async fn fetch_file(
    State(app_state): State<AppState>,
    Path(path): Path<String>,
    AdminHeaders { tenant_id, host }: AdminHeaders,
) -> Result<Response, (StatusCode, impl IntoResponse)> {
    let tenant_id = tenant_id.into();
    let cache = Cache::new(app_state.connector.clone(), tenant_id);
    let host = host.hostname();
    let key = format!("seo_file:{}:{}", host, path);

    let path_in_str = match cache.get(&key).await {
        Ok(value) => value,
        Err(_) => {
            if let Ok(path) = app_state.admin_entity.get_full_path(tenant_id, &path).await {
                format!("{}/{}", host, path)
            } else {
                format!("{}/{}", host, path)
            }
        }
    };

    let bucket = app_state.secret.get("S3_BUCKET", "/").await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(AdminResponse {
                error: Some("S3_BUCKET not set".into()),
                ..Default::default()
            }),
        )
    })?;

    let response = app_state
        .s3
        .get_object()
        .bucket(&bucket)
        .key(&path_in_str)
        .send()
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(AdminResponse {
                    error: Some(format!("S3 error: {}", error)),
                    ..Default::default()
                }),
            )
        })?;

    let content_type = response
        .content_type()
        .unwrap_or("application/octet-stream")
        .to_string();

    let content_length = response.content_length().unwrap_or(0);
    let body = response.body.into_async_read();

    if let Err(error) = cache.set(&key, &path_in_str, 86400).await {
        log::warn!("Failed to cache response for key {}: {}", key, error);
    }

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, content_length)
        .body(Body::from_stream(ReaderStream::new(body)))
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(AdminResponse {
                    error: Some(format!("Stream error: {}", error)),
                    ..Default::default()
                }),
            )
        })
}

#[utoipa::path(
    get,
    path = "/seo/schemas",
    params(
        QueryPagingInput // Utoipa automatically extracts fields from the struct
    ),
    responses(
        (status = 200, description = "Paginated list of API schemas", body = AdminResponse),
        (status = 500, description = "Database error", body = AdminResponse)
    ),
    security(
        ("admin_auth" = [])
    ),
    tag = "Schemas"
)]
pub async fn list_paginated_api_schemas(
    State(app_state): State<AppState>,
    AdminHeaders { tenant_id, .. }: AdminHeaders,
    Query(QueryPagingInput { after, limit }): Query<QueryPagingInput>,
) -> Result<impl IntoResponse, (StatusCode, impl IntoResponse)> {
    let after = after.unwrap_or(0);
    let limit = limit.unwrap_or(10);

    if limit > 100 {
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(AdminResponse {
                error: Some(format!(
                    "Maximum item per page does not exceed 100, currently is {}",
                    limit
                )),
                ..Default::default()
            }),
        ))
    } else {
        match app_state
            .admin_entity
            .list_paginated_api_schema(tenant_id.into(), after, limit)
            .await
        {
            Ok(data) => {
                let next_after = if data.len() == limit as usize {
                    data.last().unwrap().id
                } else {
                    None
                };

                Ok(JsonResponse(AdminResponse {
                    apis: Some(ListApiSchema { data, next_after }),
                    ..Default::default()
                }))
            }
            Err(error) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(AdminResponse {
                    error: Some(format!("Database error: {}", error)),
                    ..Default::default()
                }),
            )),
        }
    }
}

#[utoipa::path(
    post,
    path = "/seo/schemas",
    request_body = [Api],
    responses(
        (status = 200, description = "Successfully created schemas", body = AdminResponse),
        (status = 500, description = "Database error", body = AdminResponse)
    ),
    security(
        ("admin_auth" = [])
    ),
    tag = "Schemas"
)]
pub async fn create_api_schemas(
    State(app_state): State<AppState>,
    AdminHeaders { tenant_id, .. }: AdminHeaders,
    JsonRequest(schemas): JsonRequest<Vec<Api>>,
) -> Result<impl IntoResponse, (StatusCode, impl IntoResponse)> {
    match app_state
        .admin_entity
        .create_api_schemas(tenant_id.into(), schemas)
        .await
    {
        Ok(data) => Ok(JsonResponse(AdminResponse {
            apis: Some(ListApiSchema {
                data,
                next_after: None,
            }),
            ..Default::default()
        })),
        Err(error) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(AdminResponse {
                error: Some(format!("Database error: {}", error)),
                ..Default::default()
            }),
        )),
    }
}

#[utoipa::path(
    get,
    path = "/seo/schemas/{id}/metadata",
    params(
        ("id" = i64, Path, description = "The internal database ID of the API schema")
    ),
    responses(
        (status = 200, description = "Returns the requested API schema", body = AdminResponse),
        (status = 500, description = "Database error or schema not found", body = AdminResponse)
    ),
    security(
        ("admin_auth" = [])
    ),
    tag = "Schemas"
)]
pub async fn get_api_schema(
    State(app_state): State<AppState>,
    Path(id): Path<i64>,
    AdminHeaders { tenant_id, .. }: AdminHeaders,
) -> Result<impl IntoResponse, (StatusCode, impl IntoResponse)> {
    match app_state
        .admin_entity
        .get_api_schema_by_id(tenant_id.into(), id)
        .await
    {
        Ok(data) => Ok(JsonResponse(AdminResponse {
            api: Some(data),
            ..Default::default()
        })),
        Err(error) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(AdminResponse {
                error: Some(format!("Database error: {}", error)),
                ..Default::default()
            }),
        )),
    }
}

type BoxedAdminError = Box<(StatusCode, JsonResponse<AdminResponse>)>;

fn admin_error(status: StatusCode, message: String) -> BoxedAdminError {
    Box::new((
        status,
        JsonResponse(AdminResponse {
            error: Some(message),
            ..Default::default()
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/seo/schemas/{id}/query/{path:.*}",
    params(
        ("id" = i64, Path, description = "The internal database ID of the API schema"),
        ("path" = String, Path, description = "The derivative path of the API (can be empty or multiple segments like v1/data)"),
        ("params" = Option<HashMap<String, String>>, Query, description = "Dynamic query arguments for the API")
    ),
    responses(
        (status = 200, description = "Successful query", body = AdminResponse),
        (status = 500, description = "Execution error", body = AdminResponse)
    ),
    security(
        ("admin_auth" = [])
    ),
    tag = "Schemas"
)]
pub async fn query_data_from_api(
    State(app_state): State<AppState>,
    Path((id, path)): Path<(i64, String)>,
    AdminHeaders { tenant_id, .. }: AdminHeaders,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, (StatusCode, impl IntoResponse)> {
    let tenant_id: i64 = tenant_id.into();
    let cache = Cache::new(app_state.connector.clone(), tenant_id);

    let paths = if path.is_empty() {
        vec![]
    } else {
        path.split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    };

    let mut sorted_keys = params.keys().cloned().collect::<Vec<_>>();
    sorted_keys.sort();

    let sorted_values = sorted_keys
        .iter()
        .filter_map(|k| params.get(k).cloned())
        .collect::<Vec<_>>();

    let query_string_part = sorted_keys
        .iter()
        .zip(sorted_values.iter())
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&");

    let cache_key = format!("res:{}:{}:{}:{}", tenant_id, id, path, query_string_part);

    if let Ok(cached) = cache.get(&cache_key).await {
        return Ok(fast_cache_response(cached).into_response());
    }

    let api_result = app_state
        .admin_entity
        .perform_api_by_api_id(
            tenant_id,
            id,
            paths,
            sorted_values.clone(),
            HashMap::new(),
            None,
        )
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(AdminResponse {
                    error: Some(format!("API Error: {}", error)),
                    ..Default::default()
                }),
            )
        })?;

    let response_data = AdminResponse {
        query: Some(api_result),
        ..Default::default()
    };

    if let Ok(serialized) = serde_json::to_string(&response_data) {
        let _ = cache.set(&cache_key, &serialized, 300).await;
    }

    Ok((StatusCode::OK, JsonResponse(response_data)).into_response())
}

fn fast_cache_response(cached_json: String) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(cached_json))
        .unwrap()
}

#[utoipa::path(
    get,
    path = "/seo/logs/{name}",
    params(
        ("name" = String, Path, description = "The unique identifier for the log DSN (e.g., 'primary_db_log')")
    ),
    responses(
        (status = 200, description = "Successfully retrieved log partitions", body = AdminResponse),
        (status = 500, description = "Failed to connect to log server or retrieve DSN", body = AdminResponse)
    ),
    security(
        ("admin_auth" = [])
    ),
    tag = "System Logs"
)]
pub async fn get_appended_log(
    State(app_state): State<AppState>,
    Path(name): Path<String>,
    AdminHeaders { tenant_id, .. }: AdminHeaders,
) -> Result<impl IntoResponse, (StatusCode, impl IntoResponse)> {
    let dsn = app_state
        .admin_entity
        .get_unencrypted_token(tenant_id.into(), &name)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(AdminResponse {
                    error: Some(format!(
                        "Failed to get dsn from our token inventory: {error}"
                    )),
                    ..Default::default()
                }),
            )
        })?;

    let result: Result<Vec<String>, BoxedAdminError> = tokio::task::spawn_blocking(move || {
        let logger = AppendedLog::new(&dsn).map_err(|e| {
            admin_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to connect to log server: {e}"),
            )
        })?;

        let partitions = logger.list_partitions().map_err(|e| {
            admin_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed listing partitions: {e}"),
            )
        })?;

        Ok(partitions)
    })
    .await
    .map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(AdminResponse {
                error: Some(format!("Failed while spawning blocking task: {error}")),
                ..Default::default()
            }),
        )
    })?;

    match result {
        Ok(_) => Ok(JsonResponse(AdminResponse::default())),
        Err(boxed_err) => Err(*boxed_err), // Unbox để trả về Axum
    }
}

#[utoipa::path(
    patch,
    path = "/seo/logs/{name}",
    params(
        ("name" = String, Path, description = "The unique identifier for the log DSN to rotate")
    ),
    responses(
        (status = 200, description = "New log partition created successfully", body = AdminResponse),
        (status = 500, description = "Rotation failed or DSN not found", body = AdminResponse)
    ),
    security(
        ("admin_auth" = [])
    ),
    tag = "System Logs"
)]
pub async fn rotate_new_partition(
    State(app_state): State<AppState>,
    Path(name): Path<String>,
    AdminHeaders { tenant_id, .. }: AdminHeaders,
) -> Result<impl IntoResponse, (StatusCode, impl IntoResponse)> {
    let dsn = app_state
        .admin_entity
        .get_unencrypted_token(tenant_id.into(), &name)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(AdminResponse {
                    error: Some(format!(
                        "Failed to get dsn from our token inventory: {error}"
                    )),
                    ..Default::default()
                }),
            )
        })?;

    let result: Result<(), BoxedAdminError> = tokio::task::spawn_blocking(move || {
        let logger = AppendedLog::new(&dsn).map_err(|e| {
            admin_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to connect to log server: {e}"),
            )
        })?;

        logger.rotate_new_partition().map_err(|e| {
            admin_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed rotate a new partition: {e}"),
            )
        })?;

        Ok(())
    })
    .await
    .map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(AdminResponse {
                error: Some(format!("Failed while spawning blocking task: {error}")),
                ..Default::default()
            }),
        )
    })?;

    match result {
        Ok(_) => Ok(JsonResponse(AdminResponse::default())),
        Err(boxed_err) => Err(*boxed_err),
    }
}

#[utoipa::path(
    get,
    path = "/seo/tables",
    params(
        QueryPagingInput // Utoipa automatically extracts fields from the struct
    ),
    responses(
        (status = 200, description = "Paginated list of Table schemas", body = AdminResponse),
        (status = 500, description = "Database error", body = AdminResponse)
    ),
    security(
        ("admin_auth" = [])
    ),
    tag = "Schemas"
)]
pub async fn list_paginated_table_schemas(
    State(app_state): State<AppState>,
    AdminHeaders { tenant_id, .. }: AdminHeaders,
    Query(QueryPagingInput { after, limit }): Query<QueryPagingInput>,
) -> Result<impl IntoResponse, (StatusCode, impl IntoResponse)> {
    let after = after.unwrap_or(0);
    let limit = limit.unwrap_or(10);

    if limit > 100 {
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(AdminResponse {
                error: Some(format!(
                    "Maximum item per page does not exceed 100, currently is {}",
                    limit
                )),
                ..Default::default()
            }),
        ))
    } else {
        match app_state
            .admin_entity
            .list_paginated_table_schema(tenant_id.into(), after, limit)
            .await
        {
            Ok(data) => {
                let next_after = if data.len() == limit as usize {
                    data.last().unwrap().id
                } else {
                    None
                };

                Ok(JsonResponse(AdminResponse {
                    tables: Some(ListTableSchema { data, next_after }),
                    ..Default::default()
                }))
            }
            Err(error) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(AdminResponse {
                    error: Some(format!("Database error: {}", error)),
                    ..Default::default()
                }),
            )),
        }
    }
}

#[utoipa::path(
    post,
    path = "/seo/tables",
    request_body = [Api],
    responses(
        (status = 200, description = "Successfully created schemas", body = AdminResponse),
        (status = 500, description = "Database error", body = AdminResponse)
    ),
    security(
        ("admin_auth" = [])
    ),
    tag = "Schemas"
)]
pub async fn create_table_schemas(
    State(app_state): State<AppState>,
    AdminHeaders { tenant_id, .. }: AdminHeaders,
    JsonRequest(schemas): JsonRequest<Vec<Table>>,
) -> Result<impl IntoResponse, (StatusCode, impl IntoResponse)> {
    match app_state
        .admin_entity
        .create_table_schemas(tenant_id.into(), schemas)
        .await
    {
        Ok(data) => Ok(JsonResponse(AdminResponse {
            tables: Some(ListTableSchema {
                data,
                next_after: None,
            }),
            ..Default::default()
        })),
        Err(error) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(AdminResponse {
                error: Some(format!("Database error: {}", error)),
                ..Default::default()
            }),
        )),
    }
}
