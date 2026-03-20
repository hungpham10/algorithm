use std::io::{Error, ErrorKind};
use std::sync::Arc;

use infisical::secrets::GetSecretRequest;
use infisical::{AuthMethod, Client};

#[derive(Clone)]
pub struct Secret {
    client: Arc<Client>,
}

impl Secret {
    pub async fn new() -> Result<Self, Error> {
        let mut client = Client::builder().build().await.map_err(|error| {
            Error::new(
                ErrorKind::InvalidInput,
                format!("Fail to build infisical client: {:?}", error),
            )
        })?;

        client
            .login(AuthMethod::new_universal_auth(
                std::env::var("INFISICAL_CLIENT_ID").map_err(|_| {
                    Error::new(ErrorKind::InvalidInput, "Invalid INFISICAL_CLIENT_ID")
                })?,
                std::env::var("INFISICAL_CLIENT_SECRET").map_err(|_| {
                    Error::new(ErrorKind::InvalidInput, "Invalid INFISICAL_CLIENT_SECRET")
                })?,
            ))
            .await
            .map_err(|error| {
                Error::new(
                    ErrorKind::InvalidInput,
                    format!("Fail to login to infisical: {:?}", error),
                )
            })?;

        Ok(Self {
            client: Arc::new(client),
        })
    }

    pub async fn get(&self, key: &str, path: &str) -> Result<String, Error> {
        if let Ok(value) = std::env::var(key) {
            Ok(value)
        } else {
            self.force(key, path).await
        }
    }

    pub async fn force(&self, key: &str, path: &str) -> Result<String, Error> {
        let request = GetSecretRequest::builder(
            key,
            std::env::var("INFISICAL_PROJECT_ID")
                .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid INFISICAL_PROJECT_ID"))?,
            std::env::var("ENVIRONMENT").unwrap_or_else(|_| "dev".to_string()),
        )
        .path(path)
        .build();

        let secret = self.client.secrets().get(request).await.map_err(|error| {
            Error::new(
                ErrorKind::InvalidInput,
                format!("Fail fetching secret: {:?}", error),
            )
        })?;

        unsafe {
            std::env::set_var(key, secret.secret_value.clone());
        }
        Ok(secret.secret_value)
    }
}
