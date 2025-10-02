mod threads;
use threads::Entity as Threads;

use sea_orm::{
    ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QuerySelect, RuntimeErr, Set,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct Chat {
    db: Vec<Arc<DatabaseConnection>>,
}

#[derive(Deserialize, Serialize)]
pub struct Thread {
    pub thread_id: String,
    pub source_id: String,
    pub source_type: i32,
}

impl Chat {
    pub fn new(db: Vec<Arc<DatabaseConnection>>) -> Self {
        Self { db }
    }

    fn dbt(&self, tenant_id: i32) -> &DatabaseConnection {
        self.db[(tenant_id as usize) % self.db.len()].as_ref()
    }

    pub async fn get_thread_by_sender_id(
        &self,
        tenant_id: i32,
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
        tenant_id: i32,
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

    pub async fn start_new_thread(&self, tenant_id: i32, thread: Thread) -> Result<(), DbErr> {
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
