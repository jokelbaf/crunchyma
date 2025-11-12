use std::sync::Arc;
use std::time::Duration;

use dotenvy::dotenv;
use sea_orm::DatabaseConnection;
use teloxide::{
    adaptors::DefaultParseMode, prelude::*, types::ParseMode, utils::command::BotCommands,
};
use tokio::time::interval;

use crunchyma::cmd::handle_admin_toggle;
use crunchyma::db::{get_or_create_user, init_db};
use crunchyma::releases::{check_releases, create_client};

#[tokio::main]
async fn main() {
    dotenv().ok();

    pretty_env_logger::init();
    log::info!("Starting crunchyma...");

    let db_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in .env file or environment");

    let db = Arc::new(
        init_db(&db_url)
            .await
            .expect("Failed to connect to the database"),
    );

    let bot = Bot::from_env().parse_mode(ParseMode::Html);

    let background_bot = bot.clone();
    let background_db = db.clone();
    tokio::spawn(async move {
        background_task(background_bot, background_db).await;
    });

    Command::repl(bot, move |bot, msg, cmd| {
        let db = db.clone();
        async move { answer(bot, msg, cmd, db).await }
    })
    .await;
}

async fn background_task(bot: DefaultParseMode<Bot>, db: Arc<DatabaseConnection>) {
    let mut interval = interval(Duration::from_secs(60));

    loop {
        interval.tick().await;
        log::info!("Checking for new releases...");

        let crunchyroll = match create_client().await {
            Ok(client) => client,
            Err(err) => {
                log::error!("Failed to create Crunchyroll client: {}", err);
                continue;
            }
        };

        if let Err(err) = check_releases(bot.clone(), db.clone(), crunchyroll.clone()).await {
            log::error!("Failed to check releases: {}", err);
        } else {
            log::info!("Release check completed successfully.");
        }
    }
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "The following commands are supported:"
)]
enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "make user an admin.")]
    MakeAdmin(i64),
    #[command(description = "remove user from admin.")]
    RemoveAdmin(i64),
}

async fn answer(
    bot: DefaultParseMode<Bot>,
    msg: Message,
    cmd: Command,
    db: Arc<DatabaseConnection>,
) -> ResponseResult<()> {
    let user = match msg.from.clone() {
        Some(user) => match get_or_create_user(&db, user).await {
            Ok(user) => user,
            Err(err) => {
                log::error!("Failed to get or create user: {}", err);
                bot.send_message(
                    msg.chat.id,
                    "<b>Error:</b> Internal error occurred. Please try again later.",
                )
                .await?;
                return Ok(());
            }
        },
        None => return Ok(()),
    };

    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?;
        }
        Command::MakeAdmin(user_id) => {
            handle_admin_toggle(bot, msg, db, user.id, user_id, true).await?;
        }
        Command::RemoveAdmin(user_id) => {
            handle_admin_toggle(bot, msg, db, user.id, user_id, false).await?;
        }
    };

    Ok(())
}
