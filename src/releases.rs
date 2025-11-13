use std::collections::HashMap;
use std::sync::Arc;

use crunchyroll_rs::{
    Crunchyroll, Locale,
    common::StreamExt,
    crunchyroll::{CrunchyrollBuilder, DeviceIdentifier},
    media::{MediaType, Rating},
    search::{BrowseOptions, BrowseSortType, SearchEpisode, SearchMediaCollection},
};
use regex::Regex;
use reqwest::{
    Url,
    header::{HeaderMap, HeaderValue, USER_AGENT},
};
use sea_orm::{ActiveValue::Set, DatabaseConnection};
use teloxide::{
    RequestError,
    adaptors::DefaultParseMode,
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, InputFile},
};
use tokio::time::{Duration, sleep};
use uuid::Uuid;

use crate::db::{create_episode, get_episode};
use crate::entity::episodes;
use crate::utils::{format_number, is_in_past, is_today, parse_categories};

const PAGE_SIZE: u32 = 100;

const AUDIO_LOCALES_URL: &str =
    "https://static.crunchyroll.com/config/i18n/v3/audio_languages.json";

const CRUNCHYROLL_HEADER: &str = "Crunchyroll/4.90.2 (bundle_identifier:com.crunchyroll.iphone; build_number:4403585.457501952) iOS/26.2.0 Gravity/4.90.2";

const FALLBACK_POSTER_IMAGE: &str = "https://cdn.jokelbaf.dev/crunchyma/404.png";

#[derive(thiserror::Error, Debug)]
pub enum ReleaseError {
    #[error("Crunchyroll error: {0}")]
    CrunchyrollError(#[from] crunchyroll_rs::Error),
    #[error("Database error: {0}")]
    DatabaseError(#[from] sea_orm::DbErr),
    #[error("Missing environment variable: {0}")]
    MissingEnvVar(String),
    #[error("Invalid environment variable: {0}")]
    InvalidEnvVar(String),
    #[error("Request error: {0}")]
    RequestError(#[from] RequestError),
    #[error("Reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),
}

pub async fn create_client() -> Result<Crunchyroll, ReleaseError> {
    let email = std::env::var("CRUNCHYROLL_EMAIL")
        .map_err(|_| ReleaseError::MissingEnvVar("CRUNCHYROLL_EMAIL".into()))?;

    let password = std::env::var("CRUNCHYROLL_PASSWORD")
        .map_err(|_| ReleaseError::MissingEnvVar("CRUNCHYROLL_PASSWORD".into()))?;

    let uuid = std::env::var("DEVICE_UUID").unwrap_or_else(|_| Uuid::new_v4().to_string());

    let mut default_headers: HeaderMap = CrunchyrollBuilder::DEFAULT_HEADERS
        .iter()
        .cloned()
        .collect();

    default_headers.insert(
        USER_AGENT,
        HeaderValue::from_str(CRUNCHYROLL_HEADER).unwrap(),
    );

    let client = CrunchyrollBuilder::predefined_client_builder()
        .default_headers(default_headers)
        .build()?;

    let device = DeviceIdentifier {
        device_id: uuid,
        device_type: "iPhone 15".to_string(),
        device_name: Some("iPhone 15".to_string()),
    };

    let crunchyroll = Crunchyroll::builder()
        .locale(Locale::en_US)
        .client(client)
        .login_with_credentials(&email, &password, device)
        .await?;

    Ok(crunchyroll)
}

fn make_keyboard(episode: &SearchEpisode) -> InlineKeyboardMarkup {
    let series_url = &format!("https://www.crunchyroll.com/series/{}", episode.series_id);
    let series_button = InlineKeyboardButton::url("View Anime", Url::parse(series_url).unwrap());

    let episode_url = &format!("https://www.crunchyroll.com/watch/{}", episode.id);
    let episode_button =
        InlineKeyboardButton::url("Watch Episode", Url::parse(episode_url).unwrap());

    InlineKeyboardMarkup::new(vec![vec![series_button, episode_button]])
}

fn make_msg_text(
    episode: &SearchEpisode,
    series_rating: &Rating,
    audio_locales: &HashMap<String, String>,
) -> String {
    let re = Regex::new(r"_+").unwrap();
    let tags = format!(
        "#{} #{}",
        re.replace_all(&episode.slug_title.replace('-', "_"), "_"),
        re.replace_all(&episode.series_slug_title.replace('-', "_"), "_"),
    );

    let title = match episode.audio_locale {
        Locale::ja_JP => episode.series_title.clone(),
        _ => {
            let locale = episode.audio_locale.to_string();
            let language = audio_locales.get(&locale).map_or(&locale, |s| s);
            format!("{} ({} dub)", episode.series_title, language)
        }
    };

    format!(
        "<b>{}</b>\n\n<b>Genres:</b> {}\n\n<blockquote><b>About:</b>\n{}\n\nSeason: <b>{}</b>\nEpisode: <b>{}</b>\nDuration: <b>{} mins</b>\nSeries rating: <b>{}★ ({})</b>\nAge restrictions: <b>{}</b></blockquote>\n\n{}",
        title,
        parse_categories(&episode.categories),
        episode.description,
        episode.season_number,
        episode.episode_number.unwrap_or(0),
        episode.duration.num_minutes(),
        series_rating.average,
        format_number(series_rating.total),
        episode.maturity_ratings.join(", "),
        tags,
    )
}

pub async fn check_releases(
    bot: DefaultParseMode<Bot>,
    db: Arc<DatabaseConnection>,
    crunchyroll: Crunchyroll,
) -> Result<(), ReleaseError> {
    let audio_locales = fetch_audio_locales().await?;

    let options = BrowseOptions::default()
        .sort(BrowseSortType::NewlyAdded)
        .media_type(MediaType::Custom("episode".to_string()));

    let mut browse = crunchyroll.browse(options);
    browse.page_size(PAGE_SIZE);

    let mut episodes: Vec<SearchEpisode> = Vec::new();

    for _ in 0..PAGE_SIZE {
        if let Some(item) = browse.next().await {
            let item = item?;
            if let SearchMediaCollection::Episode(episode) = item
                && !episode.is_clip
                && (is_today(&episode.free_available_date)
                    || is_today(&episode.premium_available_date))
            {
                let db_episode = get_episode(&db, &episode.id).await?;
                if db_episode.is_none() {
                    episodes.push(episode);
                }
            }
        } else {
            break;
        }
    }

    log::info!("Found {} new episodes", episodes.len());

    episodes.sort_by_key(|e| {
        is_today(&e.free_available_date)
            .then(|| e.free_available_date)
            .unwrap_or(e.premium_available_date)
    });

    for episode in episodes {
        if !is_in_past(&episode.free_available_date) && !is_in_past(&episode.premium_available_date)
        {
            // Only post episodes that are actually available
            continue;
        }
        sleep(Duration::from_secs(1)).await;

        let series = episode.series().await?;
        let series_rating = series.rating().await?;

        let msg_text = make_msg_text(&episode, &series_rating, &audio_locales);
        let keyboard = make_keyboard(&episode);

        let channel_id = std::env::var("CHANNEL_ID")
            .map_err(|_| ReleaseError::MissingEnvVar("CHANNEL_ID".into()))?
            .parse::<i64>()
            .map_err(|_| ReleaseError::InvalidEnvVar("CHANNEL_ID".into()))?;

        let photo_source = series
            .images
            .poster_tall
            .iter()
            .max_by_key(|img| img.width * img.height)
            .map(|img| img.source.as_str())
            .unwrap_or(FALLBACK_POSTER_IMAGE);

        let photo_url = reqwest::Url::parse(photo_source).unwrap();

        bot.send_photo(ChatId(channel_id), InputFile::url(photo_url))
            .reply_markup(keyboard)
            .caption(msg_text)
            .await?;

        let db_episode = episodes::ActiveModel {
            id: Set(episode.id.clone()),
            title: Set(episode.title.clone()),
            description: Set(Some(episode.description.clone())),
            series_title: Set(episode.series_title.clone()),
            series_id: Set(episode.series_id.clone()),
            season_id: Set(episode.season_id.clone()),
            number: Set(episode.episode_number.unwrap_or(0) as i64),
            season_number: Set(episode.season_number as i64),
            audio_locale: Set(episode.audio_locale.to_string()),
        };

        create_episode(&db, &db_episode).await?;
    }

    Ok(())
}

pub async fn fetch_audio_locales() -> Result<HashMap<String, String>, ReleaseError> {
    let client = reqwest::Client::new();
    let response = client.get(AUDIO_LOCALES_URL).send().await?;

    let locales: HashMap<String, String> = response.json().await?;

    Ok(locales)
}
