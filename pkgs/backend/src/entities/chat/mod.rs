mod threads;
use threads::Entity as Threads;

use log::info;
use sea_orm::{
    ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QuerySelect, RuntimeErr, Set,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct Chat {
    db: Arc<DatabaseConnection>,
}

#[derive(Deserialize, Serialize)]
pub struct Thread {
    pub thread_id: String,
    pub source_id: String,
    pub source_type: i32,
}

impl Chat {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn get_thread_by_sender_id(
        &self,
        tenant_id: i32,
        sender_id: &String,
    ) -> Result<String, DbErr> {
        let thread_id = Threads::find()
            .filter(threads::Column::TenantId.eq(tenant_id))
            .filter(threads::Column::SourceId.eq(sender_id))
            .column(threads::Column::ThreadId)
            .into_tuple::<String>()
            .one(&*self.db)
            .await;
        match thread_id {
            Ok(thread_id) => {
                if let Some(thread_id) = thread_id {
                    Ok(thread_id)
                } else {
                    Err(DbErr::Query(RuntimeErr::Internal(format!(
                        "Not found {}",
                        sender_id,
                    ))))
                }
            }
            Err(error) => {
                info!("{:?}", error);
                Err(error)
            }
        }
    }

    pub async fn get_sender_id_by_thread(
        &self,
        tenant_id: i32,
        thread_id: &String,
    ) -> Result<String, DbErr> {
        let sender_id = Threads::find()
            .filter(threads::Column::TenantId.eq(tenant_id))
            .filter(threads::Column::ThreadId.eq(thread_id))
            .column(threads::Column::SourceId)
            .into_tuple::<String>()
            .one(&*self.db)
            .await?;
        if let Some(sender_id) = sender_id {
            Ok(sender_id)
        } else {
            Err(DbErr::Query(RuntimeErr::Internal(format!(
                "Not found {}",
                thread_id,
            ))))
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

        Threads::insert(thread_model).exec(&*self.db).await?;
        Ok(())
    }
}
