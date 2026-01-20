use teloxide::prelude::*;
use super::keyboards;
use super::state::{BotState, NodeType};
use crate::chains::{get_chain_by_id, get_default_chains, Chain};
use crate::config::Protocol;
use crate::shodan::ShodanResult;
use crate::validator::ValidatedNode;
use futures::future::join_all;

pub async fn handle_callback(
    bot: Bot,
    q: CallbackQuery,
    state: BotState,
) -> ResponseResult<()> {
    let data = q.data.as_deref().unwrap_or("");
    let user_id = q.from.id.0 as i64;
    let chat_id = q.message.as_ref().map(|m| m.chat.id).unwrap_or(ChatId(0));
    let message_id = q.message.as_ref().map(|m| m.id);

    // Acknowledge callback
    bot.answer_callback_query(&q.id).await?;

    let parts: Vec<&str> = data.split(':').collect();

    match parts.as_slice() {
        // Node type selection
        ["node", node_type] => {
            let nt = match *node_type {
                "full" => NodeType::Full,
                "archive" => NodeType::Archive,
                "bulk" => NodeType::Bulk,
                _ => return Ok(()),
            };

            state.update_session(user_id, |s| {
                s.node_type = Some(nt);
            }).await;

            if let Some(msg_id) = message_id {
                bot.edit_message_text(chat_id, msg_id, "Select a chain:")
                    .reply_markup(keyboards::chain_selection())
                    .await?;
            }
        }

        // Chain selection
        ["chain", chain_id] => {
            if *chain_id == "custom" {
                state.update_session(user_id, |s| {
                    s.awaiting_chain_id = true;
                }).await;

                if let Some(msg_id) = message_id {
                    bot.edit_message_text(
                        chat_id,
                        msg_id,
                        "Enter the Chain ID (decimal number):\n\nExamples:\n• 137 for Polygon\n• 42161 for Arbitrum\n• 10 for Optimism",
                    )
                    .await?;
                }
            } else if let Ok(id) = chain_id.parse::<u64>() {
                if let Some(chain) = get_chain_by_id(id) {
                    state.update_session(user_id, |s| {
                        s.chain = Some(chain);
                    }).await;

                    if let Some(msg_id) = message_id {
                        bot.edit_message_text(chat_id, msg_id, "Select a location:")
                            .reply_markup(keyboards::location_selection())
                            .await?;
                    }
                }
            }
        }

        // Location selection - trigger search
        ["location", location] => {
            let session = state.get_session(user_id).await;
            let country_code = if *location == "all" { None } else { Some(*location) };

            if let (Some(node_type), Some(chain)) = (session.node_type, session.chain) {
                // Clear session
                state.clear_session(user_id).await;

                let chain_name = chain.name.clone();

                // Send searching message
                if let Some(msg_id) = message_id {
                    bot.edit_message_text(chat_id, msg_id, "🔍 Searching...")
                        .await?;
                }

                // Perform search and validation
                let result = perform_search(
                    state.clone(),
                    user_id,
                    node_type,
                    chain.clone(),
                    country_code,
                )
                .await;

                match result {
                    Ok(nodes) if !nodes.is_empty() => {
                        send_results(&bot, chat_id, &nodes, node_type, &chain_name).await?;
                    }
                    Ok(_) => {
                        // No nodes found, try all locations if we had a specific location
                        if country_code.is_some() {
                            if let Some(msg_id) = message_id {
                                bot.edit_message_text(
                                    chat_id,
                                    msg_id,
                                    "No nodes found in selected location. Expanding to all locations...",
                                )
                                .await?;
                            }

                            let expanded_result = perform_search(
                                state.clone(),
                                user_id,
                                node_type,
                                chain,
                                None,
                            )
                            .await;

                            match expanded_result {
                                Ok(nodes) if !nodes.is_empty() => {
                                    send_results(&bot, chat_id, &nodes, node_type, &chain_name).await?;
                                }
                                _ => {
                                    bot.send_message(
                                        chat_id,
                                        "❌ No working nodes found. The network may be experiencing issues.",
                                    )
                                    .await?;
                                }
                            }
                        } else {
                            bot.send_message(
                                chat_id,
                                "❌ No working nodes found. The network may be experiencing issues.",
                            )
                            .await?;
                        }
                    }
                    Err(e) => {
                        bot.send_message(chat_id, format!("❌ Error: {}", e)).await?;
                    }
                }
            }
        }

        // Back navigation
        ["back", target] => {
            match *target {
                "main" => {
                    state.clear_session(user_id).await;
                    if let Some(msg_id) = message_id {
                        bot.edit_message_text(
                            chat_id,
                            msg_id,
                            "🔍 <b>Node Finder</b>\n\nSelect an option:",
                        )
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .reply_markup(keyboards::main_menu())
                        .await?;
                    }
                }
                "chain" => {
                    if let Some(msg_id) = message_id {
                        bot.edit_message_text(chat_id, msg_id, "Select a chain:")
                            .reply_markup(keyboards::chain_selection())
                            .await?;
                    }
                }
                _ => {}
            }
        }

        // Config menu
        ["config", action] => {
            let config = state.config_manager.get_user_config(user_id).await;

            match *action {
                "menu" => {
                    if let Some(msg_id) = message_id {
                        bot.edit_message_text(chat_id, msg_id, "⚙️ Configuration")
                            .reply_markup(keyboards::config_menu(&config))
                            .await?;
                    }
                }
                "count" => {
                    if let Some(msg_id) = message_id {
                        bot.edit_message_text(
                            chat_id,
                            msg_id,
                            format!("Current: {} nodes\n\nSelect new default count:", config.default_count),
                        )
                        .reply_markup(keyboards::count_selection())
                        .await?;
                    }
                }
                "protocol" => {
                    let new_protocol = match config.protocol {
                        Protocol::Http => Protocol::Ws,
                        Protocol::Ws => Protocol::Http,
                    };
                    state.config_manager.update_user_config(user_id, |c| {
                        c.protocol = new_protocol;
                    }).await;

                    let updated_config = state.config_manager.get_user_config(user_id).await;
                    if let Some(msg_id) = message_id {
                        bot.edit_message_text(chat_id, msg_id, "⚙️ Configuration")
                            .reply_markup(keyboards::config_menu(&updated_config))
                            .await?;
                    }
                }
                "sync" => {
                    if let Some(msg_id) = message_id {
                        bot.edit_message_text(
                            chat_id,
                            msg_id,
                            format!(
                                "Current: {} blocks tolerance\n\nSelect new sync tolerance:",
                                config.sync_tolerance
                            ),
                        )
                        .reply_markup(keyboards::sync_tolerance_selection())
                        .await?;
                    }
                }
                "rpcs" => {
                    let mut rpc_text = String::from("📡 Reference RPCs:\n\n");
                    for chain in get_default_chains() {
                        let rpc = config
                            .reference_rpcs
                            .get(&chain.id)
                            .map(|s| s.as_str())
                            .unwrap_or(&chain.default_rpc);
                        rpc_text.push_str(&format!("{} {}: {}\n", chain.symbol, chain.name, rpc));
                    }
                    rpc_text.push_str("\nSelect a chain to edit:");

                    if let Some(msg_id) = message_id {
                        bot.edit_message_text(chat_id, msg_id, rpc_text)
                            .reply_markup(keyboards::rpc_selection())
                            .await?;
                    }
                }
                _ => {}
            }
        }

        // Set count
        ["setcount", count] => {
            if let Ok(n) = count.parse::<u32>() {
                state.config_manager.update_user_config(user_id, |c| {
                    c.default_count = n;
                }).await;

                let config = state.config_manager.get_user_config(user_id).await;
                if let Some(msg_id) = message_id {
                    bot.edit_message_text(chat_id, msg_id, "⚙️ Configuration")
                        .reply_markup(keyboards::config_menu(&config))
                        .await?;
                }
            }
        }

        // Set sync tolerance
        ["setsync", tolerance] => {
            if let Ok(n) = tolerance.parse::<u64>() {
                state.config_manager.update_user_config(user_id, |c| {
                    c.sync_tolerance = n;
                }).await;

                let config = state.config_manager.get_user_config(user_id).await;
                if let Some(msg_id) = message_id {
                    bot.edit_message_text(chat_id, msg_id, "⚙️ Configuration")
                        .reply_markup(keyboards::config_menu(&config))
                        .await?;
                }
            }
        }

        // Edit RPC - start wizard
        ["editrpc", chain_id] => {
            if let Ok(id) = chain_id.parse::<u64>() {
                state.update_session(user_id, |s| {
                    s.custom_chain_id = Some(id);
                    s.awaiting_rpc_url = true;
                }).await;

                let chain_name = get_chain_by_id(id)
                    .map(|c| c.name)
                    .unwrap_or_else(|| format!("Chain {}", id));

                if let Some(msg_id) = message_id {
                    bot.edit_message_text(
                        chat_id,
                        msg_id,
                        format!("Enter new reference RPC URL for {}:", chain_name),
                    )
                    .await?;
                }
            }
        }

        _ => {}
    }

    Ok(())
}

async fn perform_search(
    state: BotState,
    user_id: i64,
    node_type: NodeType,
    chain: Chain,
    country_code: Option<&str>,
) -> Result<Vec<ValidatedNode>, String> {
    let config = state.config_manager.get_user_config(user_id).await;

    // Get reference block number
    let reference_rpc = config
        .get_reference_rpc(chain.id)
        .cloned()
        .unwrap_or(chain.default_rpc.clone());

    let reference_block = state
        .http_validator
        .get_current_block(&reference_rpc)
        .await
        .map_err(|e| format!("Reference node unavailable: {}. Configure a custom RPC in settings or try again later.", e))?;

    // Query Shodan
    let shodan_results = state
        .shodan
        .search_nodes(chain.id, country_code)
        .await?;

    if shodan_results.is_empty() {
        return Ok(vec![]);
    }

    // Filter by protocol preference
    // For WebSocket, we use port 8545 results and convert to 8546 (Shodan doesn't index 8546)
    let filtered: Vec<ShodanResult> = shodan_results
        .into_iter()
        .filter(|r| r.is_http_port())
        .collect();

    // Determine how many nodes to validate
    let target_count = match node_type {
        NodeType::Full | NodeType::Archive => config.default_count as usize,
        NodeType::Bulk => 50,
    };

    // Validate nodes in parallel
    let validation_futures: Vec<_> = filtered
        .into_iter()
        .take(target_count * 3) // Validate more to account for failures
        .map(|result| {
            let state = state.clone();
            let chain = chain.clone();
            let genesis_hash = chain.genesis_hash.clone();
            async move {
                let url = match config.protocol {
                    Protocol::Http => result.http_url(),
                    Protocol::Ws => result.ws_url(),
                };

                let validation_result = match config.protocol {
                    Protocol::Http => {
                        state
                            .http_validator
                            .validate(
                                &url,
                                chain.id,
                                &genesis_hash,
                                reference_block,
                                config.sync_tolerance,
                            )
                            .await
                    }
                    Protocol::Ws => {
                        state
                            .ws_validator
                            .validate(
                                &url,
                                chain.id,
                                &genesis_hash,
                                reference_block,
                                config.sync_tolerance,
                            )
                            .await
                    }
                };

                validation_result.ok()
            }
        })
        .collect();

    let results: Vec<ValidatedNode> = join_all(validation_futures)
        .await
        .into_iter()
        .flatten()
        .collect();

    // For archive nodes, additionally check archive capability
    let mut final_results = if node_type == NodeType::Archive {
        let archive_futures: Vec<_> = results
            .into_iter()
            .map(|node| {
                let validator = state.archive_validator.clone();
                async move { validator.validate_archive(node).await.ok() }
            })
            .collect();

        join_all(archive_futures)
            .await
            .into_iter()
            .flatten()
            .filter(|n| n.is_archive)
            .collect()
    } else {
        results
    };

    // Sort by latency
    final_results.sort_by_key(|n| n.latency_ms);

    // Limit results
    final_results.truncate(target_count);

    Ok(final_results)
}

async fn send_results(
    bot: &Bot,
    chat_id: ChatId,
    nodes: &[ValidatedNode],
    node_type: NodeType,
    chain_name: &str,
) -> ResponseResult<()> {
    if node_type == NodeType::Bulk {
        // JSON format, split if needed
        let urls: Vec<&str> = nodes.iter().map(|n| n.url.as_str()).collect();
        let json = serde_json::to_string_pretty(&urls).unwrap_or_default();

        // Split into chunks if needed (Telegram limit is 4096)
        let chunks: Vec<String> = json
            .chars()
            .collect::<Vec<char>>()
            .chunks(4000)
            .map(|c| c.iter().collect::<String>())
            .collect();

        for (i, chunk) in chunks.iter().enumerate() {
            let msg = if chunks.len() > 1 {
                format!("<b>{}</b> - Bulk Export\n<pre>{}</pre>\nPart {}/{}", chain_name, chunk, i + 1, chunks.len())
            } else {
                format!("<b>{}</b> - Bulk Export\n<pre>{}</pre>", chain_name, chunk)
            };

            bot.send_message(chat_id, msg)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
        }
    } else {
        // List format
        let type_name = match node_type {
            NodeType::Full => "synced",
            NodeType::Archive => "archive",
            NodeType::Bulk => "bulk",
        };

        let mut msg = format!("✅ Found {} {} <b>{}</b> nodes:\n\n", nodes.len(), type_name, chain_name);
        for (i, node) in nodes.iter().enumerate() {
            msg.push_str(&format!("{}. <code>{}</code>\n", i + 1, node.url));
        }

        bot.send_message(chat_id, msg)
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;
    }

    Ok(())
}
