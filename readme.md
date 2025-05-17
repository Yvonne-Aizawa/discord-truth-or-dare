# Discord Truth or Dare Bot

This project is a Discord bot that provides a fun and interactive way to play Truth or Dare in Discord servers. The bot allows users to get random truth questions or dares, add their own, and even suggest new ones for moderators to approve.

## Features

- **Truth or Dare Commands**: Get random truth questions or dares, including NSFW options if used in an NSFW channel.
- **Add Questions/Dares**: Users with the appropriate role can add new truth questions or dares to the database.
- **Suggestion System**: Users can suggest new truth questions or dares, which moderators can approve or deny.
- **Channel Restrictions**: Commands can be restricted to specific channels for better moderation.
- **Role-Based Access**: Certain commands are restricted to users with specific roles.

## Commands

### Public Commands
- `/truth`: Get a random truth question. 
- `/dare`: Get a random dare.

        both of these look at the channel they are in. if its age gated it chooses an nsfw question/dare
### Moderator Commands
- `/add_question`: Add a new truth question.
- `/add_dare`: Add a new dare.

        only users with the tod_admin role can use those

### Suggestion Command
- `/suggest`: Suggest a new truth or dare for moderators to review.

        anyone can use this command. and ANYONE can accept them so make sure that the channel where the suggestions are sent is private for admins only

## Usage Examples
- `/truth`: Returns a random truth question.
- `/dare`: Returns a random dare.
- `/add_question "What is your biggest fear?"`: Adds a new truth question.
- `/suggest "Do a handstand for 10 seconds"`: Suggests a new dare.

## Setup

### Prerequisites
- Rust (latest stable version)
- SQLite
- A Discord bot token
- `.env` file with the following variables:
  - `DISCORD_TOKEN`: Your bot's token from the Discord Developer Portal.
  - `ALLOWED_CHANNEL_IDS`: Comma-separated list of channel IDs where commands are allowed.
  - `SUGGESTION_CHANNEL_ID`: The channel ID where suggestions will be sent.

### Installation
1. Clone the repository:
   ```bash
   git clone https://github.com/Yvonne-Aizawa/discord-truth-or-dare.git
   cd discord-truth-or-dare
2. create .env file
    ```env
    DISCORD_TOKEN=your_discord_token
    ALLOWED_CHANNEL_IDS=123456789012345678,987654321098765432
    SUGGESTION_CHANNEL_ID=123456789012345678
    ```
3. build the program
    ```bash 
    cargo build --release
    ```
4. run the program
    ```bash
    cargo run --release
    ```
### Todo
- [ ] make a command to allow/disallow a channel
- [ ] allow sfw in nsfw channels when configured
- [ ] see if there is a way to hide admin only commands from non admins
- [x] make sure only admins can accept suggestions
### Database
The bot uses an SQLite database (truth_or_dare.db) to store truth questions and dares. The database is automatically initialized with the following tables:

* questions: Stores truth questions.
* dares: Stores dares.
### Contributing
Contributions are welcome! Feel free to open issues or submit pull requests.
### Support
If you encounter any issues or have questions, feel free to open an issue on the [GitHub repository](https://github.com/Yvonne-Aizawa/discord-truth-or-dare/issues).
### License
This project is licensed under the GNU GPLv3 License. See the LICENSE file for details.

### Acknowledgments
* Serenity for Discord API integration.
* Poise for command framework.
* dotenvy for environment variable management.
* rusqlite for SQLite integration.
* and of couse github copilot for doing 99% of this