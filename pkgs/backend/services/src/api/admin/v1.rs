use std::fs;
use std::io::{Cursor, ErrorKind, Write};
use std::path::PathBuf;

use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};

use axum::Router;
use axum::body::Body;
use axum::extract::{Json as JsonRequest, Path, Query, State};
use axum::http::{self, header};
use axum::response::{IntoResponse, Json as JsonResponse, Response};
use axum::routing::get;

use http::StatusCode;
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use tokio_util::io::ReaderStream;

use integration::components::appended_log::AppendedLog;
use models::cache::Cache;
use models::entities::admin::Api;

use crate::api::AppState;
use crate::api::admin::{AdminHeaders, FetchFileFromS3Headers, PurgeFileFromS3Headers};

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct QueryPagingInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ListApiSchema {
    data: Vec<Api>,
    next_after: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AdminResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    apis: Option<ListApiSchema>,

    #[serde(skip_serializing_if = "Option::is_none")]
    api: Option<Api>,

    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/seo/tenant/{host}/id", get(get_tenant_id))
        .route("/seo/sitemap", get(build_sitemap_xml))
        .route("/seo/news", get(build_news_xml))
        .route("/seo/files/{*path}", get(fetch_file).head(purge_file))
        .route(
            "/seo/schemas",
            get(list_paginated_api_schemas).post(create_api_schemas),
        )
        .route("/seo/schemas/{id}", get(get_api_schema))
        .route("/seo/tokens/{name}", get(get_token).post(put_token))
        .route(
            "/seo/logs/{name}",
            get(get_appended_log).patch(rotate_new_partition),
        )
    //.route("/seo/features",
    //    get(get_features)
    //        .put(configure_features)
    //)
}

async fn get_token(
    State(app_state): State<AppState>,
    Path(name): Path<String>,
    AdminHeaders { tenant_id }: AdminHeaders,
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

#[derive(Deserialize)]
struct PutTokenPayload {
    name: String,
    token: String,
}

async fn put_token(
    State(app_state): State<AppState>,
    AdminHeaders { tenant_id }: AdminHeaders,
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
//    AdminHeaders { tenant_id, }: AdminHeaders,
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

async fn build_sitemap_xml(
    State(app_state): State<AppState>,
    AdminHeaders { tenant_id }: AdminHeaders,
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

async fn build_news_xml(
    State(app_state): State<AppState>,
    AdminHeaders { tenant_id }: AdminHeaders,
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

pub async fn purge_file(
    Path(path): Path<String>,
    PurgeFileFromS3Headers { host }: PurgeFileFromS3Headers,
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

pub async fn fetch_file(
    State(app_state): State<AppState>,
    Path(path): Path<String>,
    FetchFileFromS3Headers { tenant_id, host }: FetchFileFromS3Headers,
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

    let bucket = std::env::var("S3_BUCKET").map_err(|_| {
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

pub async fn list_paginated_api_schemas(
    State(app_state): State<AppState>,
    AdminHeaders { tenant_id }: AdminHeaders,
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

pub async fn create_api_schemas(
    State(app_state): State<AppState>,
    AdminHeaders { tenant_id }: AdminHeaders,
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

pub async fn get_api_schema(
    State(app_state): State<AppState>,
    Path(id): Path<i64>,
    AdminHeaders { tenant_id }: AdminHeaders,
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

pub async fn get_appended_log(
    State(app_state): State<AppState>,
    Path(name): Path<String>,
    AdminHeaders { tenant_id }: AdminHeaders,
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

pub async fn rotate_new_partition(
    State(app_state): State<AppState>,
    Path(name): Path<String>,
    AdminHeaders { tenant_id }: AdminHeaders,
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
