mod bot;
mod chains;
mod config;
mod shodan;
mod validator;

use bot::{callbacks, commands, BotState};
use dotenvy::dotenv;
use std::env;
use teloxide::prelude::*;

#[tokio::main]
async fn main() {
    // Load environment variables
    dotenv().ok();
    env_logger::init();

    log::info!("Starting Node Finder bot...");

    let telegram_token = env::var("TELEGRAM_TOKEN")
        .expect("TELEGRAM_TOKEN must be set in .env file");
    let shodan_token = env::var("SHODAN_TOKEN")
        .expect("SHODAN_TOKEN must be set in .env file");

    let bot = Bot::new(telegram_token);
    let state = BotState::new(shodan_token);

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .filter_command::<commands::Command>()
                .endpoint(handle_command_with_state),
        )
        .branch(
            Update::filter_message()
                .filter(|msg: Message| msg.text().is_some())
                .endpoint(handle_message_with_state),
        )
        .branch(
            Update::filter_callback_query()
                .endpoint(handle_callback_with_state),
        );

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![state])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

async fn handle_command_with_state(
    bot: Bot,
    msg: Message,
    cmd: commands::Command,
    state: BotState,
) -> ResponseResult<()> {
    commands::handle_command(bot, msg, cmd, state).await
}

async fn handle_message_with_state(
    bot: Bot,
    msg: Message,
    state: BotState,
) -> ResponseResult<()> {
    commands::handle_message(bot, msg, state).await
}

async fn handle_callback_with_state(
    bot: Bot,
    q: CallbackQuery,
    state: BotState,
) -> ResponseResult<()> {
    callbacks::handle_callback(bot, q, state).await
}
