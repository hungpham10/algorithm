use std::io::Cursor;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;

use actix_web::error::ErrorInternalServerError;
use actix_web::web::{Data, Path};
use actix_web::{HttpResponse, Result};

use aws_sdk_s3::primitives::ByteStream;

use super::SeoHeaders;
use crate::api::AppState;

struct Stream(ByteStream);

impl actix::prelude::Stream for Stream {
    type Item = Result<actix_web::web::Bytes, actix_web::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match futures_util::ready!(Pin::new(&mut self.0).poll_next(cx)) {
            Some(Ok(bytes)) => Poll::Ready(Some(Ok(actix_web::web::Bytes::from(bytes)))),
            Some(Err(e)) => Poll::Ready(Some(Err(actix_web::error::ErrorInternalServerError(
                format!("ByteStream error: {}", e),
            )))),
            None => Poll::Ready(None),
        }
    }
}

pub async fn tenant_id(appstate: Data<Arc<AppState>>, path: Path<String>) -> Result<HttpResponse> {
    let host = path.into_inner();

    if let Some(entity) = appstate.seo_entity() {
        match entity.get_tenant_id(&host).await {
            Ok(id) => Ok(HttpResponse::Ok().body(format!("{}", id))),
            Err(error) => Ok(HttpResponse::InternalServerError()
                .body(format!("Failed resolve tenant id: {}", error))),
        }
    } else {
        Ok(HttpResponse::InternalServerError().body(format!("Not implemented")))
    }
}

pub async fn features(appstate: Data<Arc<AppState>>, headers: SeoHeaders) -> Result<HttpResponse> {
    if let Some(entity) = appstate.seo_entity() {
        Ok(HttpResponse::InternalServerError().body(format!("Not implemented")))
    } else {
        Ok(HttpResponse::InternalServerError().body(format!("Not implemented")))
    }
}

pub async fn sitemap(appstate: Data<Arc<AppState>>, headers: SeoHeaders) -> Result<HttpResponse> {
    if let Some(entity) = appstate.seo_entity() {
        match entity.list_sites(headers.tenant_id).await {
            Ok(sites) => {
                let mut buffer = Cursor::new(Vec::new());
                let mut writer = Writer::new(&mut buffer);

                if sites.len() > 0 {
                    // XML header
                    writer
                        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
                        .map_err(|e| ErrorInternalServerError(e))?;

                    // <urlset>
                    let mut urlset = BytesStart::new("urlset");
                    urlset.push_attribute(("xmlns", "http://www.sitemaps.org/schemas/sitemap/0.9"));
                    writer
                        .write_event(Event::Start(urlset))
                        .map_err(|e| ErrorInternalServerError(e))?;

                    for site in sites {
                        writer
                            .write_event(Event::Start(BytesStart::new("url")))
                            .map_err(|e| ErrorInternalServerError(e))?;

                        // <loc>
                        writer
                            .write_event(Event::Start(BytesStart::new("loc")))
                            .map_err(|e| ErrorInternalServerError(e))?;
                        writer
                            .write_event(Event::Text(BytesText::new(site.loc.as_str())))
                            .map_err(|e| ErrorInternalServerError(e))?;
                        writer
                            .write_event(Event::End(BytesEnd::new("loc")))
                            .map_err(|e| ErrorInternalServerError(e))?;

                        // <lastmod>
                        writer
                            .write_event(Event::Start(BytesStart::new("lastmod")))
                            .map_err(|e| ErrorInternalServerError(e))?;
                        writer
                            .write_event(Event::Text(BytesText::new(
                                site.lastmod.format("%Y-%m-%d").to_string().as_str(),
                            )))
                            .map_err(|e| ErrorInternalServerError(e))?;
                        writer
                            .write_event(Event::End(BytesEnd::new("lastmod")))
                            .map_err(|e| ErrorInternalServerError(e))?;

                        // <changefreq>
                        writer
                            .write_event(Event::Start(BytesStart::new("changefreq")))
                            .map_err(|e| ErrorInternalServerError(e))?;
                        writer
                            .write_event(Event::Text(BytesText::new(site.freq.as_str())))
                            .map_err(|e| ErrorInternalServerError(e))?;
                        writer
                            .write_event(Event::End(BytesEnd::new("changefreq")))
                            .map_err(|e| ErrorInternalServerError(e))?;

                        // <priority>
                        writer
                            .write_event(Event::Start(BytesStart::new("priority")))
                            .map_err(|e| ErrorInternalServerError(e))?;
                        writer
                            .write_event(Event::Text(BytesText::new(
                                site.priority.to_string().as_str(),
                            )))
                            .map_err(|e| ErrorInternalServerError(e))?;
                        writer
                            .write_event(Event::End(BytesEnd::new("priority")))
                            .map_err(|e| ErrorInternalServerError(e))?;

                        writer
                            .write_event(Event::End(BytesEnd::new("url")))
                            .map_err(|e| ErrorInternalServerError(e))?;
                    }

                    // </urlset>
                    writer
                        .write_event(Event::End(BytesEnd::new("urlset")))
                        .map_err(|e| ErrorInternalServerError(e))?;

                    Ok(HttpResponse::Ok().content_type("application/xml").body(
                        String::from_utf8(buffer.into_inner())
                            .map_err(|e| ErrorInternalServerError(e))?,
                    ))
                } else {
                    Ok(HttpResponse::InternalServerError().body(format!(
                        "Sitemap is empty for tenant_id {}, host {}",
                        headers.tenant_id, headers.host
                    )))
                }
            }
            Err(error) => Ok(HttpResponse::InternalServerError()
                .body(format!("Failed to get sitemap.xml: {}", error))),
        }
    } else {
        Ok(HttpResponse::InternalServerError().body(format!("Not implemented")))
    }
}

pub async fn file(
    appstate: Data<Arc<AppState>>,
    path: Path<String>,
    headers: SeoHeaders,
) -> Result<HttpResponse> {
    // Extract kind and id from the path
    let path_in_str = format!("{}/{}", headers.host, path.into_inner());

    // @TODO: to support A/B testing, must design muxing here to do that

    // Get the S3 bucket name from environment variable
    let bucket = match std::env::var("S3_BUCKET") {
        Ok(bucket) => bucket,
        Err(_) => {
            return Ok(
                HttpResponse::InternalServerError().body("S3_BUCKET environment variable not set")
            )
        }
    };

    // Send the S3 get_object request
    let response = match appstate
        .s3
        .get_object()
        .bucket(&bucket)
        .key(&path_in_str)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            return Ok(
                HttpResponse::InternalServerError().body(format!("Failed to fetch from S3: {}", e))
            )
        }
    };

    // Build the streaming response
    Ok(HttpResponse::Ok().streaming(Stream(response.body)))
}
