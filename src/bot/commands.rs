use teloxide::{prelude::*, utils::command::BotCommands};
use super::keyboards;
use super::state::BotState;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
pub enum Command {
    #[command(description = "Start the bot and show main menu")]
    Start,
    #[command(description = "Show help information")]
    Help,
}

pub async fn handle_command(
    bot: Bot,
    msg: Message,
    cmd: Command,
    state: BotState,
) -> ResponseResult<()> {
    match cmd {
        Command::Start => handle_start(bot, msg, state).await,
        Command::Help => handle_help(bot, msg).await,
    }
}

async fn handle_start(bot: Bot, msg: Message, state: BotState) -> ResponseResult<()> {
    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);

    // Clear any existing session
    state.clear_session(user_id).await;

    let text = "🔍 *Node Finder*\n\n\
                Find public RPC nodes from Shodan.\n\n\
                Select an option below:";

    bot.send_message(msg.chat.id, text)
        .parse_mode(teloxide::types::ParseMode::MarkdownV2)
        .reply_markup(keyboards::main_menu())
        .await?;

    Ok(())
}

async fn handle_help(bot: Bot, msg: Message) -> ResponseResult<()> {
    let text = "📖 *Node Finder Help*\n\n\
                *Commands:*\n\
                /start \\- Show main menu\n\
                /help \\- Show this help\n\n\
                *Node Types:*\n\
                • Full Node \\- Synced nodes\n\
                • Archive Node \\- Nodes with historical data\n\
                • Bulk Nodes \\- JSON export of many nodes\n\n\
                *Config:*\n\
                • Set default node count\n\
                • Choose HTTP or WS protocol\n\
                • Adjust sync tolerance\n\
                • Set custom reference RPCs";

    bot.send_message(msg.chat.id, text)
        .parse_mode(teloxide::types::ParseMode::MarkdownV2)
        .await?;

    Ok(())
}

/// Handle text messages (for custom chain wizard)
pub async fn handle_message(bot: Bot, msg: Message, state: BotState) -> ResponseResult<()> {
    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);
    let session = state.get_session(user_id).await;
    let text = msg.text().unwrap_or("");

    if session.awaiting_chain_id {
        // Parse chain ID
        match text.trim().parse::<u64>() {
            Ok(chain_id) => {
                state.update_session(user_id, |s| {
                    s.custom_chain_id = Some(chain_id);
                    s.awaiting_chain_id = false;
                    s.awaiting_rpc_url = true;
                }).await;

                bot.send_message(
                    msg.chat.id,
                    "Enter a reference RPC URL for this chain:\n\n\
                     Example: https://polygon-rpc.com",
                )
                .await?;
            }
            Err(_) => {
                bot.send_message(
                    msg.chat.id,
                    "❌ Invalid chain ID. Please enter a valid number (e.g., 137 for Polygon):",
                )
                .await?;
            }
        }
    } else if session.awaiting_rpc_url {
        // Parse and validate URL
        if text.starts_with("http://") || text.starts_with("https://") {
            let chain_id = session.custom_chain_id.unwrap_or(1);

            // Store custom RPC in user config
            state.config_manager.update_user_config(user_id, |config| {
                config.reference_rpcs.insert(chain_id, text.to_string());
            }).await;

            state.update_session(user_id, |s| {
                s.awaiting_rpc_url = false;
                s.chain = Some(crate::chains::Chain {
                    id: chain_id,
                    name: format!("Chain {}", chain_id),
                    symbol: "🔧".to_string(),
                    default_rpc: text.to_string(),
                    genesis_hash: String::new(), // Custom chains skip genesis check
                });
            }).await;

            bot.send_message(msg.chat.id, "✅ Custom chain configured!\n\nSelect a location:")
                .reply_markup(keyboards::location_selection())
                .await?;
        } else {
            bot.send_message(
                msg.chat.id,
                "❌ Invalid URL. Please enter a valid RPC URL starting with http:// or https://:",
            )
            .await?;
        }
    }

    Ok(())
}
