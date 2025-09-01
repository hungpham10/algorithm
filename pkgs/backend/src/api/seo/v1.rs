use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

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

pub async fn robots(appstate: Data<Arc<AppState>>, headers: SeoHeaders) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().body("ok"))
}

pub async fn sitemap(appstate: Data<Arc<AppState>>, headers: SeoHeaders) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().body("ok"))
}

pub async fn file(
    appstate: Data<Arc<AppState>>,
    path: Path<String>,
    headers: SeoHeaders,
) -> Result<HttpResponse> {
    // Extract kind and id from the path
    let path_in_str = path.into_inner();

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
