use dotenvy::dotenv;
use env_logger::Env;
use log::{error, info};
use poise::{
    ChoiceParameter,
    serenity_prelude::{self as serenity},
};
use rusqlite::{Connection, Result as SqlResult, params};
use std::env;

struct Data {}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// Initializes the SQLite database and creates the necessary tables if they don't exist
fn initialize_database() -> SqlResult<Connection> {
    // Initialize the logger

    info!("Initializing database...");
    let db_path = env::var("DATABASE_PATH").unwrap_or_else(|_| "truth_or_dare.db".to_string());
    let conn = match Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to open database: {}", e);
            return Err(e);
        }
    };

    if let Err(e) = conn.execute(
        "CREATE TABLE IF NOT EXISTS questions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            text TEXT NOT NULL,
            author TEXT NOT NULL,
            nsfw BOOL DEFAULT 0
        )",
        [],
    ) {
        error!("Failed to create 'questions' table: {}", e);
        return Err(e);
    }

    if let Err(e) = conn.execute(
        "CREATE TABLE IF NOT EXISTS dares (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            text TEXT NOT NULL,
            author TEXT NOT NULL,
            nsfw BOOL DEFAULT 0
        )",
        [],
    ) {
        error!("Failed to create 'dares' table: {}", e);
        return Err(e);
    }

    if let Err(e) = conn.execute(
        "CREATE TABLE IF NOT EXISTS suggestions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            kind TEXT NOT NULL,
            suggestion TEXT NOT NULL,
            nsfw BOOL NOT NULL,
            author TEXT NOT NULL,
            status TEXT DEFAULT 'pending'
        )",
        [],
    ) {
        error!("Failed to create 'suggestions' table: {}", e);
        return Err(e);
    }

    info!("Database initialized successfully.");
    Ok(conn)
}

/// Fetches a random entry from the specified table
fn get_random_entry(conn: &Connection, table: &str, nsfw: bool) -> Result<String, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT text FROM {} WHERE nsfw = {} ORDER BY RANDOM() LIMIT 1",
            table, nsfw
        ))
        .map_err(|e| e.to_string())?;
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;

    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        Ok(row.get(0).map_err(|e| e.to_string())?)
    } else {
        Err(format!(
            "no {} found, please add some using /suggest",
            table
        ))
    }
}

/// Adds a new entry to the specified table
fn add_entry(conn: &Connection, table: &str, text: &str, nsfw: bool) -> Result<(), String> {
    info!("Adding entry to {}: {}", table, text);
    conn.execute(
        &format!("INSERT INTO {} (text, nsfw) VALUES (?1, ?2)", table),
        params![text, nsfw],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Middleware check to restrict commands to owners or users with a specific role
async fn is_admin(ctx: Context<'_>) -> Result<bool, Error> {
    let required_role_name = "tod_admin"; // Replace with your desired role name

    if let Some(guild_id) = ctx.guild_id() {
        let member = ctx
            .serenity_context()
            .http
            .get_member(guild_id, ctx.author().id)
            .await?;
        if let Some(guild) = ctx.guild() {
            if member.roles.iter().any(|role_id| {
                guild
                    .roles
                    .get(role_id)
                    .map(|r| r.name == required_role_name)
                    .unwrap_or(false)
            }) {
                return Ok(true);
            }
        }
    }
    //notify the user that they don't have the required role
    ctx.send(
        poise::CreateReply::default()
            .content(format!(
                "only users with the role {} can use this command",
                required_role_name
            ))
            .ephemeral(true),
    ).await;
    // If the user does not have the required role, return false
    Ok(false)
}

/// Middleware check to restrict commands to a specific channel
async fn is_in_allowed_channel(ctx: Context<'_>) -> Result<bool, Error> {
    // Get the allowed channel IDs from the environment variable they are stored in an array
    let allowed_channel_ids: Vec<u64> = env::var("ALLOWED_CHANNEL_IDS")
        .expect("Expected ALLOWED_CHANNEL_IDS in the environment")
        .split(',')
        .filter_map(|s| s.trim().parse::<u64>().ok())
        .collect();

    if let Some(channel_id) = ctx
        .channel_id()
        .to_channel(&ctx.serenity_context())
        .await
        .ok()
        .map(|c| c.id())
    {
        // Check if the command is used in an allowed channel
        if allowed_channel_ids.contains(&channel_id.get()) {
            return Ok(true);
        }
    }

    // If the command is not in the allowed channel, send an ephemeral message
    ctx.send(
        poise::CreateReply::default()
            .content("This command can only be used in the allowed channel.")
            .ephemeral(true),
    )
    .await?;
    return Ok(false);
}

/// Get a random truth
#[poise::command(slash_command, prefix_command, check = "is_in_allowed_channel")]
async fn truth(ctx: Context<'_>) -> Result<(), Error> {
    let conn = initialize_database().map_err(|e| format!("Database error: {}", e))?;
    let channel_id = ctx.channel_id().to_string();
    let channel = ctx
        .serenity_context()
        .http
        .get_channel(channel_id.parse::<u64>().unwrap().into())
        .await
        .map_err(|e| format!("Error fetching channel: {}", e))?;
    let is_nsfw = channel.guild().map(|c| c.nsfw).unwrap_or(false);
    match get_random_entry(&conn, "questions", is_nsfw) {
        Ok(question) => {
            // ctx.say(question).await?;
            create_embed_message(
                &ctx,
                "Truth",
                &question,
                &("Requested by: ".to_owned() + &ctx.author().name),
            )
            .await?;
        }
        Err(err) => {
            ctx.say(format!("Error: {}", err)).await?;
        }
    }
    Ok(())
}
// create embed message
async fn create_embed_message(
    ctx: &Context<'_>,
    title: &str,
    description: &str,
    footer: &str,
) -> Result<(), Error> {
    let embed = serenity::CreateEmbed::new()
        .title(title)
        .description(description)
        .footer(serenity::CreateEmbedFooter::new(footer))
        .color(serenity::Color::from_rgb(0, 255, 0));

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// Get a random dare
#[poise::command(slash_command, prefix_command, check = "is_in_allowed_channel")]
async fn dare(ctx: Context<'_>) -> Result<(), Error> {
    let conn = initialize_database().map_err(|e| format!("Database error: {}", e))?;
    // if the command is used in a nsfw channel, get a random nsfw dare
    let channel_id = ctx.channel_id().to_string();
    let channel = ctx
        .serenity_context()
        .http
        .get_channel(channel_id.parse::<u64>().unwrap().into())
        .await
        .map_err(|e| format!("Error fetching channel: {}", e))?;
    let is_nsfw = channel.guild().map(|c| c.nsfw).unwrap_or(false);
    match get_random_entry(&conn, "dares", is_nsfw) {
        Ok(dare) => {
            create_embed_message(
                &ctx,
                "Dare",
                &dare,
                &("Requested by: ".to_owned() + &ctx.author().name),
            )
            .await?;
        }
        Err(err) => {
            ctx.say(format!("Error: {}", err)).await?;
        }
    }
    Ok(())
}

/// Add a new truth question
#[poise::command(slash_command, prefix_command, check = "is_admin")]
async fn add_question(
    ctx: Context<'_>,
    #[description = "The question you want to add"] question: String,
    #[description = "Whether the question is NSFW (true or false)"] nsfw: bool,
) -> Result<(), Error> {
    let conn = initialize_database().map_err(|e| format!("Database error: {}", e))?;

    match add_entry(&conn, "questions", &question, nsfw) {
        Ok(_) => {
            ctx.say("Question added!").await?;
        }
        Err(err) => {
            ctx.say(format!("Error: {}", err)).await?;
        }
    }
    Ok(())
}

/// Add a new dare
#[poise::command(slash_command, prefix_command, check = "is_admin")]
async fn add_dare(
    ctx: Context<'_>,
    #[description = "The dare you want to add."] dare: String,
    #[description = "Whether the dare is NSFW (true or false)"] nsfw: bool,
) -> Result<(), Error> {
    let conn = initialize_database().map_err(|e| format!("Database error: {}", e))?;

    match add_entry(&conn, "dares", &dare, nsfw) {
        Ok(_) => {
            ctx.say("Dare added!").await?;
        }
        Err(err) => {
            ctx.say(format!("Error: {}", err)).await?;
        }
    }
    Ok(())
}

/// Enum for suggestion types
#[derive(ChoiceParameter, Debug, PartialEq)] // Automatically implements required traits
pub enum SuggestionType {
    #[name = "Truth"]
    Truth,
    #[name = "Dare"]
    Dare,
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::Builder::from_env(Env::default().default_filter_or("error")).init();

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let intents = serenity::GatewayIntents::non_privileged();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                truth(),
                dare(),
                add_question(),
                add_dare(),
                suggest(),
                approve(),
                reject(),
            ],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        })
        .build();

    let mut client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await
        .expect("Error creating client");

    client.start().await.expect("Error starting client");
}
#[poise::command(slash_command, prefix_command, check = "is_in_allowed_channel")]
pub async fn suggest(
    ctx: Context<'_>,
    #[description = "What do you want to suggest?"] kind: SuggestionType,
    #[description = "The suggestion."] suggestion: String,
    #[description = "Is it NSFW?"] nsfw: bool,
) -> Result<(), Error> {
    info!(
        "Received a suggestion: kind = {:?}, nsfw = {}, author = {}",
        kind,
        nsfw,
        ctx.author().name
    );

    let conn = initialize_database().map_err(|e| {
        error!("Failed to initialize database: {}", e);
        format!("Database error: {}", e)
    })?;

    // Persist the suggestion in the database
    conn.execute(
        "INSERT INTO suggestions (kind, suggestion, nsfw, author) VALUES (?1, ?2, ?3, ?4)",
        params![
            format!("{:?}", kind),
            suggestion,
            nsfw,
            ctx.author().id.to_string()
        ],
    )
    .map_err(|e| format!("Error saving suggestion: {}", e))?;

    // Retrieve the ID of the newly inserted suggestion
    let id: i64 = conn
        .query_row("SELECT last_insert_rowid()", [], |row| row.get(0))
        .map_err(|e| format!("Error retrieving suggestion ID: {}", e))?;

    info!("Suggestion saved to database successfully with ID {}", id);

    // Notify the user that the suggestion has been saved
    ctx.say(format!(
        "Suggestion saved with ID {} and awaiting moderator review!",
        id
    ))
    .await?;

    // Send the suggestion to the suggestion channel
    let embed = serenity::CreateEmbed::new()
        .title(format!(
            "{:?} suggestion {}",
            kind,
            if nsfw { "(NSFW)" } else { "" }
        ))
        .description(&suggestion)
        .field("ID", id.to_string(), false)
        .footer(serenity::CreateEmbedFooter::new(
            "Requested by: ".to_owned() + &ctx.author().name,
        ))
        .color(serenity::Color::from_rgb(255, 0, 0));

    let channel_id = match env::var("SUGGESTION_CHANNEL_ID") {
        Ok(id) => id
            .parse::<u64>()
            .expect("SUGGESTION_CHANNEL_ID must be a valid u64"),
        Err(e) => {
            error!(
                "Failed to retrieve SUGGESTION_CHANNEL_ID from environment: {}",
                e
            );
            return Err("Missing SUGGESTION_CHANNEL_ID environment variable".into());
        }
    };

    let builder = serenity::CreateMessage::default().embed(embed);

    serenity::ChannelId::new(channel_id)
        .send_message(&ctx.serenity_context().http, builder)
        .await
        .map_err(|e| format!("Error sending suggestion: {}", e))?;

    info!("Suggestion sent to suggestion channel successfully.");

    Ok(())
}
/// Approve a suggestion by its ID
#[poise::command(slash_command, prefix_command, check = "is_admin")]
async fn approve(
    ctx: Context<'_>,
    #[description = "The ID of the suggestion to approve"] id: i64,
) -> Result<(), Error> {
    let conn = initialize_database().map_err(|e| format!("Database error: {}", e))?;

    // Update the suggestion's status to "approved"
    let rows_affected = conn
        .execute(
            "UPDATE suggestions SET status = 'approved' WHERE id = ?1",
            params![id],
        )
        .map_err(|e| format!("Database error: {}", e))?;

    if rows_affected == 0 {
        ctx.say(format!("No suggestion found with ID {}", id))
            .await?;
    } else {
        // publish the suggestion to the appropriate table
        let suggestion: String = conn
            .query_row(
                "SELECT suggestion FROM suggestions WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .map_err(|e| format!("Database error: {}", e))?;
        let kind: String = conn
            .query_row(
                "SELECT kind FROM suggestions WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .map_err(|e| format!("Database error: {}", e))?;
        let nsfw: bool = conn
            .query_row(
                "SELECT nsfw FROM suggestions WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .map_err(|e| format!("Database error: {}", e))?;
        let owner: String = conn
            .query_row(
                "SELECT author FROM suggestions WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .map_err(|e| format!("Database error: {}", e))?;
        if kind == "Truth" {
            conn.execute(
                "INSERT INTO questions (text, nsfw, author) VALUES (?1, ?2, ?3)",
                params![suggestion, nsfw, owner],
            )
            .map_err(|e| format!("Database error: {}", e))?;
        } else if kind == "Dare" {
            conn.execute(
                "INSERT INTO dares (text, nsfw, author) VALUES (?1, ?2, ?3)",
                params![suggestion, nsfw, owner],
            )
            .map_err(|e| format!("Database error: {}", e))?;
        }

        ctx.say(format!("Suggestion with ID {} has been approved!", id))
            .await?;
    }

    Ok(())
}

/// Reject a suggestion by its ID
#[poise::command(slash_command, prefix_command, check = "is_admin")]
async fn reject(
    ctx: Context<'_>,
    #[description = "The ID of the suggestion to reject"] id: i64,
) -> Result<(), Error> {
    let conn = initialize_database().map_err(|e| format!("Database error: {}", e))?;

    // Update the suggestion's status to "rejected"
    let rows_affected = conn
        .execute(
            "UPDATE suggestions SET status = 'rejected' WHERE id = ?1",
            params![id],
        )
        .map_err(|e| format!("Database error: {}", e))?;

    if rows_affected == 0 {
        ctx.say(format!("No suggestion found with ID {}", id))
            .await?;
    } else {
        ctx.say(format!("Suggestion with ID {} has been rejected!", id))
            .await?;
    }

    Ok(())
}
