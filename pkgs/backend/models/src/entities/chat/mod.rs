mod threads;
use threads::Entity as Threads;

use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QuerySelect, Set};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::resolver::Resolver;

pub struct Chat {
    resolver: Arc<Resolver>,
}

#[derive(Deserialize, Serialize)]
pub struct Thread {
    pub thread_id: String,
    pub source_id: String,
    pub source_type: i32,
}

impl Chat {
    pub fn new(resolver: &Arc<Resolver>) -> Self {
        Self {
            resolver: resolver.clone(),
        }
    }

    fn dbt(&self, tenant_id: i64) -> &DatabaseConnection {
        self.resolver.database(tenant_id)
    }

    pub async fn get_thread_by_sender_id(
        &self,
        tenant_id: i64,
        sender_id: &String,
    ) -> Result<Option<String>, DbErr> {
        let thread_id = Threads::find()
            .select_only()
            .column(threads::Column::ThreadId)
            .filter(threads::Column::TenantId.eq(tenant_id))
            .filter(threads::Column::SourceId.eq(sender_id))
            .into_tuple::<String>()
            .one(self.dbt(tenant_id))
            .await;
        match thread_id {
            Ok(thread_id) => {
                if let Some(thread_id) = thread_id {
                    Ok(Some(thread_id))
                } else {
                    Ok(None)
                }
            }
            Err(error) => Err(error),
        }
    }

    pub async fn get_sender_id_by_thread(
        &self,
        tenant_id: i64,
        thread_id: &String,
    ) -> Result<Option<String>, DbErr> {
        let sender_id = Threads::find()
            .select_only()
            .column(threads::Column::SourceId)
            .filter(threads::Column::TenantId.eq(tenant_id))
            .filter(threads::Column::ThreadId.eq(thread_id))
            .into_tuple::<String>()
            .one(self.dbt(tenant_id))
            .await?;
        if let Some(sender_id) = sender_id {
            Ok(Some(sender_id))
        } else {
            Ok(None)
        }
    }

    pub async fn start_new_thread(&self, tenant_id: i64, thread: Thread) -> Result<(), DbErr> {
        let thread_model = threads::ActiveModel {
            tenant_id: Set(tenant_id),
            thread_id: Set(thread.thread_id),
            source_id: Set(thread.source_id),
            source_type: Set(thread.source_type),
            ..Default::default()
        };

        Threads::insert(thread_model)
            .exec(self.dbt(tenant_id))
            .await?;
        Ok(())
    }
}
