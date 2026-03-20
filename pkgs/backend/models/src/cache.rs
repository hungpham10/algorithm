use redis::{AsyncCommands, RedisResult};
use std::sync::Arc;

use super::resolver::Resolver;

pub struct Cache {
    resolver: Arc<Resolver>,
    tenant_id: i64,
}

impl Cache {
    pub fn new(resolver: Arc<Resolver>, tenant_id: i64) -> Self {
        Self {
            resolver,
            tenant_id,
        }
    }

    pub async fn get(&self, key: &String) -> RedisResult<String> {
        self.resolver.cache(self.tenant_id).get(key).await
    }

    pub async fn set(&self, key: &String, value: &String, ttl: usize) -> RedisResult<()> {
        self.resolver
            .cache(self.tenant_id)
            .set_ex(key, value, ttl as u64)
            .await
    }
}
