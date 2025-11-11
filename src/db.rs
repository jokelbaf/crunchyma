use sea_orm::{
    ActiveModelTrait, ConnectionTrait, Database, DatabaseConnection, DbBackend, DbErr, EntityTrait,
    Schema,
};
use teloxide::types::User;

use crate::entity::episodes;
use crate::entity::users;

pub async fn init_db(db_url: &str) -> Result<DatabaseConnection, DbErr> {
    let db = Database::connect(db_url).await?;

    let schema = Schema::new(DbBackend::Postgres);

    let episodes_schema = schema
        .create_table_from_entity(episodes::Entity)
        .if_not_exists()
        .to_owned();
    db.execute(db.get_database_backend().build(&episodes_schema))
        .await?;

    let users_schema = schema
        .create_table_from_entity(users::Entity)
        .if_not_exists()
        .to_owned();
    db.execute(db.get_database_backend().build(&users_schema))
        .await?;

    Ok(db)
}

pub async fn get_user_by_id(
    db: &DatabaseConnection,
    user_id: i64,
) -> Result<Option<users::Model>, DbErr> {
    users::Entity::find_by_id(user_id).one(db).await
}

pub async fn get_or_create_user(
    db: &DatabaseConnection,
    user: User,
) -> Result<users::Model, DbErr> {
    if let Some(existing_user) = get_user_by_id(db, user.id.0 as i64).await? {
        Ok(existing_user)
    } else {
        let name = format!("{} {}", user.first_name, user.last_name.unwrap_or_default())
            .trim()
            .to_string();

        let new_user = users::ActiveModel {
            id: sea_orm::ActiveValue::Set(user.id.0 as i64),
            name: sea_orm::ActiveValue::Set(name),
            username: sea_orm::ActiveValue::Set(user.username.clone()),
            is_admin: sea_orm::ActiveValue::Set(false),
        };
        let res = new_user.insert(db).await?;
        Ok(res)
    }
}

pub async fn get_episode(
    db: &DatabaseConnection,
    episode_id: &str,
) -> Result<Option<episodes::Model>, DbErr> {
    episodes::Entity::find_by_id(episode_id.to_string())
        .one(db)
        .await
}

pub async fn create_episode(
    db: &DatabaseConnection,
    episode: &episodes::ActiveModel,
) -> Result<episodes::Model, DbErr> {
    episode.clone().insert(db).await
}
