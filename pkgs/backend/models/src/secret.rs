use std::io::{Error, ErrorKind};
use std::sync::Arc;

use infisical::secrets::GetSecretRequest;
use infisical::{AuthMethod, Client};

#[derive(Clone)]
pub struct Secret {
    client: Option<Arc<Client>>,
}

impl Secret {
    pub async fn new() -> Result<Self, Error> {
        let client_id = std::env::var("INFISICAL_CLIENT_ID");
        let client_secret = std::env::var("INFISICAL_CLIENT_SECRET");

        let client = match (client_id, client_secret) {
            (Ok(id), Ok(secret)) => {
                let mut client = Client::builder().build().await.map_err(|error| {
                    Error::new(
                        ErrorKind::InvalidInput,
                        format!("Fail to build infisical client: {:?}", error),
                    )
                })?;

                client
                    .login(AuthMethod::new_universal_auth(id, secret))
                    .await
                    .map_err(|error| {
                        Error::new(
                            ErrorKind::InvalidInput,
                            format!("Fail to login to infisical: {:?}", error),
                        )
                    })?;

                Some(Arc::new(client))
            }
            _ => None,
        };

        Ok(Self { client })
    }

    pub async fn get(&self, key: &str, path: &str) -> Result<String, Error> {
        if let Ok(value) = std::env::var(key) {
            Ok(value)
        } else {
            self.force(key, path).await
        }
    }

    pub async fn force(&self, key: &str, path: &str) -> Result<String, Error> {
        match &self.client {
            Some(client) => {
                let request = GetSecretRequest::builder(
                    key,
                    std::env::var("INFISICAL_PROJECT_ID").map_err(|_| {
                        Error::new(ErrorKind::InvalidInput, "Invalid INFISICAL_PROJECT_ID")
                    })?,
                    std::env::var("ENVIRONMENT").unwrap_or_else(|_| "dev".to_string()),
                )
                .path(path)
                .build();

                let secret = client.secrets().get(request).await.map_err(|error| {
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
            None => {
                // No Infisical client available — check env var directly
                std::env::var(key).map_err(|_| {
                    Error::new(
                        ErrorKind::NotFound,
                        format!(
                            "Secret '{}' not found and no Infisical client available",
                            key
                        ),
                    )
                })
            }
        }
    }
}
