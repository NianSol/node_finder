use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
use crate::chains::get_default_chains;
use crate::bot::state::LOCATIONS;
use crate::config::{Protocol, UserConfig};

pub fn main_menu() -> InlineKeyboardMarkup {
    let buttons = vec![
        vec![InlineKeyboardButton::callback("🔄 Full Node", "node:full")],
        vec![InlineKeyboardButton::callback("📚 Archive Node", "node:archive")],
        vec![InlineKeyboardButton::callback("📦 Bulk Nodes", "node:bulk")],
        vec![InlineKeyboardButton::callback("⚙️ Config", "config:menu")],
    ];
    InlineKeyboardMarkup::new(buttons)
}

pub fn chain_selection() -> InlineKeyboardMarkup {
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = get_default_chains()
        .into_iter()
        .map(|c| {
            vec![InlineKeyboardButton::callback(
                format!("{} {}", c.symbol, c.name),
                format!("chain:{}", c.id),
            )]
        })
        .collect();

    buttons.push(vec![InlineKeyboardButton::callback("🔧 Custom Chain", "chain:custom")]);
    buttons.push(vec![InlineKeyboardButton::callback("« Back", "back:main")]);

    InlineKeyboardMarkup::new(buttons)
}

pub fn location_selection() -> InlineKeyboardMarkup {
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = LOCATIONS
        .iter()
        .map(|loc| {
            vec![InlineKeyboardButton::callback(
                format!("{} {}", loc.flag, loc.name),
                format!("location:{}", loc.code),
            )]
        })
        .collect();

    buttons.push(vec![InlineKeyboardButton::callback("🌍 All Locations", "location:all")]);
    buttons.push(vec![InlineKeyboardButton::callback("« Back", "back:chain")]);

    InlineKeyboardMarkup::new(buttons)
}

pub fn config_menu(config: &UserConfig) -> InlineKeyboardMarkup {
    let protocol_text = match config.protocol {
        Protocol::Http => "HTTP",
        Protocol::Ws => "WS",
    };

    let buttons = vec![
        vec![InlineKeyboardButton::callback(
            format!("📊 Default count: {}", config.default_count),
            "config:count",
        )],
        vec![InlineKeyboardButton::callback(
            format!("🔌 Protocol: {}", protocol_text),
            "config:protocol",
        )],
        vec![InlineKeyboardButton::callback(
            format!("🔄 Sync tolerance: {} blocks", config.sync_tolerance),
            "config:sync",
        )],
        vec![InlineKeyboardButton::callback("📡 Reference RPCs", "config:rpcs")],
        vec![InlineKeyboardButton::callback("« Back", "back:main")],
    ];

    InlineKeyboardMarkup::new(buttons)
}

pub fn count_selection() -> InlineKeyboardMarkup {
    let counts = [5, 10, 20, 50, 100];
    let buttons: Vec<Vec<InlineKeyboardButton>> = counts
        .chunks(3)
        .map(|chunk| {
            chunk
                .iter()
                .map(|&n| InlineKeyboardButton::callback(n.to_string(), format!("setcount:{}", n)))
                .collect()
        })
        .collect();

    let mut buttons = buttons;
    buttons.push(vec![InlineKeyboardButton::callback("« Back", "config:menu")]);

    InlineKeyboardMarkup::new(buttons)
}

pub fn sync_tolerance_selection() -> InlineKeyboardMarkup {
    let tolerances = [5, 20, 50, 100, 500];
    let buttons: Vec<Vec<InlineKeyboardButton>> = tolerances
        .chunks(3)
        .map(|chunk| {
            chunk
                .iter()
                .map(|&n| {
                    InlineKeyboardButton::callback(
                        format!("{} blocks", n),
                        format!("setsync:{}", n),
                    )
                })
                .collect()
        })
        .collect();

    let mut buttons = buttons;
    buttons.push(vec![InlineKeyboardButton::callback("« Back", "config:menu")]);

    InlineKeyboardMarkup::new(buttons)
}

pub fn rpc_selection() -> InlineKeyboardMarkup {
    let chains = get_default_chains();
    let buttons: Vec<Vec<InlineKeyboardButton>> = chains
        .into_iter()
        .map(|c| {
            vec![InlineKeyboardButton::callback(
                format!("{} {}", c.symbol, c.name),
                format!("editrpc:{}", c.id),
            )]
        })
        .collect();

    let mut buttons = buttons;
    buttons.push(vec![InlineKeyboardButton::callback("« Back", "config:menu")]);

    InlineKeyboardMarkup::new(buttons)
}

pub fn back_to_config() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("« Back", "config:menu")],
    ])
}
