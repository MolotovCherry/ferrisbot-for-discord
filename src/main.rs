#[macro_use]
extern crate diesel;

mod api;
mod commands;
mod db;
mod dispatcher;
mod schema;
mod state_machine;
mod tags;

use commands::{Args, Commands};
use dispatcher::{EventDispatcher, MessageDispatcher};
use serenity::{model::prelude::*, prelude::*, utils::parse_username, Client};
use std::collections::HashMap;
use std::str::FromStr;

type Result = crate::commands::Result<()>;

fn app() -> Result {
    let token = std::env::var("DISCORD_TOKEN")
        .map_err(|_| "missing environment variable: DISCORD_TOKEN")?;

    let _ = db::run_migrations()?;

    let mut cmds = Commands::new();

    // Tags
    cmds.add("?tag {key}", tags::get);
    cmds.add("?tag delete {key}", tags::delete);
    cmds.add("?tag create {key} [value]", tags::post);
    cmds.add("?tags", tags::get_all);

    // Slow mode.
    // 0 seconds disables slowmode
    cmds.add("?slowmode {channel} {seconds}", slow_mode);

    // Kick
    cmds.add("?kick {user}", kick);

    // Ban
    cmds.add("?ban {user}", ban);

    // Post the welcome message to the welcome channel.
    cmds.add("?CoC {channel}", welcome_message);

    let messages = MessageDispatcher::new(cmds);

    let mut client =
        Client::new_with_handlers(&token, Some(messages), Some(EventDispatcher)).unwrap();

    dispatcher::MessageStore::init(&mut client);

    client.start()?;

    Ok(())
}

fn main() {
    if let Err(err) = app() {
        eprintln!("error: {}", err);
        std::process::exit(1);
    }
}

/// Set slow mode for a channel.  
///
/// A `seconds` value of 0 will disable slowmode
fn slow_mode(args: Args) -> Result {
    if api::is_mod(&args)? {
        let seconds = &args
            .params
            .get("seconds")
            .ok_or("unable to retrieve seconds param")?
            .parse::<u64>()?;

        let channel_name = &args
            .params
            .get("channel")
            .ok_or("unable to retrieve channel param")?;

        ChannelId::from_str(channel_name)?.edit(&args.cx, |c| c.slow_mode_rate(*seconds))?;
    }
    Ok(())
}

/// Kick a user from the guild.  
///
/// Requires the kick members permission
fn kick(args: Args) -> Result {
    if api::is_mod(&args)? {
        let user_id = parse_username(
            &args
                .params
                .get("user")
                .ok_or("unable to retrieve user param")?,
        )
        .ok_or("unable to retrieve user id")?;

        if let Some(guild) = args.msg.guild(&args.cx) {
            guild.read().kick(&args.cx, UserId::from(user_id))?
        }
    }
    Ok(())
}

/// Ban an user from the guild.  
///
/// Requires the ban members permission
fn ban(args: Args) -> Result {
    if api::is_mod(&args)? {
        let user_id = parse_username(
            &args
                .params
                .get("user")
                .ok_or("unable to retrieve user param")?,
        )
        .ok_or("unable to retrieve user id")?;

        if let Some(guild) = args.msg.guild(&args.cx) {
            guild.read().ban(&args.cx, UserId::from(user_id), &"all")?
        }
    }
    Ok(())
}

/// Write the welcome message to the welcome channel.  
fn welcome_message(args: Args) -> Result {
    const WELCOME_BILLBOARD: &'static str = "By joining this community, you agree to adhere to the CoC.  Click the :white_check_mark: to indicate you agree, otherwise you can leave this Discord.  ";

    if api::is_mod(&args)? {
        let channel_name = &args
            .params
            .get("channel")
            .ok_or("unable to retrieve channel param")?;

        let channel_id = ChannelId::from_str(channel_name)?;
        let message = channel_id.say(&args.cx, WELCOME_BILLBOARD)?;
        dispatcher::MessageStore::save(&args.cx, "welcome".into(), (message, channel_id));
    }
    Ok(())
}
