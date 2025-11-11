use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection};
use teloxide::{adaptors::DefaultParseMode, prelude::*};

use crate::db::get_user_by_id;
use crate::entity::users;

pub async fn handle_admin_toggle(
    bot: DefaultParseMode<Bot>,
    msg: Message,
    db: Arc<DatabaseConnection>,
    executor_id: i64,
    target_id: i64,
    make_admin: bool,
) -> ResponseResult<()> {
    let owner_id = std::env::var("OWNER_ID")
        .expect("OWNER_ID must be set in .env file or environment")
        .parse::<i64>()
        .expect("OWNER_ID must be a valid i64");

    if executor_id != owner_id {
        bot.send_message(
            msg.chat.id,
            "<b>Error:</b> You are not authorized to use this command.",
        )
        .await?;
        return Ok(());
    }

    match get_user_by_id(&db, target_id).await {
        Ok(Some(user)) => {
            let mut active_user: users::ActiveModel = user.clone().into();
            active_user.is_admin = Set(make_admin);

            match active_user.update(&*db).await {
                Ok(_) => {
                    let action = if make_admin {
                        "made an admin"
                    } else {
                        "removed from admin"
                    };
                    bot.send_message(
                        msg.chat.id,
                        format!("User <b>{}</b> has been {}.", user.name, action),
                    )
                    .await?;
                }
                Err(err) => {
                    log::error!("Failed to update user: {}", err);
                    bot.send_message(
                        msg.chat.id,
                        "<b>Error:</b> Something went wrong. Please try again later.",
                    )
                    .await?;
                }
            }
        }
        Ok(None) => {
            bot.send_message(
                msg.chat.id,
                format!("User {} not found in the database.", target_id),
            )
            .await?;
        }
        Err(err) => {
            log::error!("Failed to retrieve user: {}", err);
            bot.send_message(
                msg.chat.id,
                "<b>Error:</b> Failed to retrieve user from the database.",
            )
            .await?;
        }
    }

    Ok(())
}
