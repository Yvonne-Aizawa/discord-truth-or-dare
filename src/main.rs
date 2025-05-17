use dotenvy::dotenv;
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
    print!("Initializing database... ");
    let conn = Connection::open("truth_or_dare.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS questions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            text TEXT NOT NULL,
            nsfw BOOL DEFAULT 0
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS dares (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            text TEXT NOT NULL,
            nsfw BOOL DEFAULT 0
        )",
        [],
    )?;
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
    conn.execute(
        &format!("INSERT INTO {} (text, nsfw) VALUES (?1, ?2)", table),
        params![text, nsfw],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Middleware check to restrict commands to owners or users with a specific role
async fn has_role(ctx: Context<'_>) -> Result<bool, Error> {
    let author_id = ctx.author().id.to_string();

    // Check if the user has a specific role
    if let Some(guild_id) = ctx.guild_id() {
        let member = ctx
            .serenity_context()
            .http
            .get_member(guild_id, ctx.author().id)
            .await?;
        let required_role_name = "tod_admin"; // Replace with your desired role name
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

    // If the user is not allowed, send an ephemeral message
    ctx.send(
        poise::CreateReply::default()
            .content("You do not have permission to use this command.")
            .ephemeral(true),
    )
    .await?;
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
    Ok(false)
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
            ctx.say(question).await?;
        }
        Err(err) => {
            ctx.say(format!("Error: {}", err)).await?;
        }
    }
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
            ctx.say(dare).await?;
        }
        Err(err) => {
            ctx.say(format!("Error: {}", err)).await?;
        }
    }
    Ok(())
}

/// Add a new truth question
#[poise::command(slash_command, prefix_command, check = "has_role")]
async fn add_question(
    ctx: Context<'_>,
    #[description = "The question you want to add"] question: String,
    #[description = "weather the question is nfsw (true or false)"] nsfw: bool,
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
#[poise::command(slash_command, prefix_command, check = "has_role")]
async fn add_dare(
    ctx: Context<'_>,
    #[description = "The dare you want to add."] dare: String,
    #[description = "weather the question is nfsw (true or false)"] nsfw: bool,
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
    // Load environment variables from the .env file
    dotenv().ok();

    // Get the bot token and owners from the environment
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let intents = serenity::GatewayIntents::non_privileged();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![truth(), dare(), add_question(), add_dare(), suggest()],

            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                // Register commands globally
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        })
        // .setup(move |_ctx, _ready, _framework| Box::pin(async move { Ok(Data {}) }))
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}

#[poise::command(slash_command, prefix_command, check = "is_in_allowed_channel")]
pub async fn suggest(
    ctx: Context<'_>,
    #[description = "what do you wanna suggest?"] kind: SuggestionType,
    #[description = "the suggestion."] suggestion: String,
    #[description = "Is it nsfw?"] nsfw: bool,
) -> Result<(), Error> {
    let embed = serenity::CreateEmbed::new()
        .title(format!(
            "{:?} suggestion {}",
            kind,
            if nsfw { "(NSFW)" } else { "" }
        ))
        .description(&suggestion)
        .footer(serenity::CreateEmbedFooter::new(
            "requested by: ".to_owned() + &ctx.author().name,
        ))
        .color(serenity::Color::from_rgb(255, 0, 0));

    let components = vec![serenity::CreateActionRow::Buttons(vec![
        serenity::CreateButton::new("accept")
            .label("Accept")
            .style(serenity::ButtonStyle::Success),
        serenity::CreateButton::new("deny")
            .label("Deny")
            .style(serenity::ButtonStyle::Danger),
    ])];
    let channel_id = env::var("SUGGESTION_CHANNEL_ID")
        .expect("Expected SUGGESTION_CHANNEL_ID in the environment")
        .parse::<u64>()
        .expect("SUGGESTION_CHANNEL_ID must be a valid u64");

    let builder = serenity::CreateMessage::default()
        .embed(embed)
        .components(components);
    //create a new message builder
    let res = serenity::ChannelId::new(channel_id)
        .send_message(&ctx.serenity_context().http, builder)
        .await
        .map_err(|e| format!("Error sending suggestion: {}", e));
    match res {
        Ok(_) => {
            ctx.say("Suggestion sent!").await?;
        }
        Err(err) => {
            ctx.say(format!("Error: {}", err)).await?;
        }
    }

    // Wait for a moderator to click a button
    while let Some(interaction) = serenity::ComponentInteractionCollector::new(ctx)
        .timeout(std::time::Duration::from_secs(600))
        .await
    {
        match interaction.data.custom_id.as_str() {
            "accept" => {
                let conn = initialize_database().map_err(|e| format!("Database error: {}", e))?;

                // Add the suggestion to the database
                match add_entry(
                    &conn,
                    if kind == SuggestionType::Truth {
                        "questions"
                    } else {
                        "dares"
                    },
                    &suggestion,
                    nsfw,
                ) {
                    Ok(_) => {
                        interaction
                            .create_response(
                                ctx,
                                serenity::CreateInteractionResponse::Message(
                                    serenity::CreateInteractionResponseMessage::new()
                                        .content("Accepted"),
                                ),
                            )
                            .await?;
                        let mut msg = interaction.message.clone();
                        msg.edit(
                            ctx,
                            serenity::EditMessage::new()
                                .components(Vec::<serenity::CreateActionRow>::new()),
                        )
                        .await?;
                    }
                    Err(err) => {}
                }
                break;
            }
            "deny" => {
                interaction
                    .create_response(
                        ctx,
                        serenity::CreateInteractionResponse::Message(
                            serenity::CreateInteractionResponseMessage::new().content("Denied"),
                        ),
                    )
                    .await?;
                let mut msg = interaction.message.clone();
                msg.edit(
                    ctx,
                    serenity::EditMessage::new()
                        .components(Vec::<serenity::CreateActionRow>::new()),
                )
                .await?;
            }
            _ => {}
        }
    }

    Ok(())
}
