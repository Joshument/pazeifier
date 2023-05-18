use poise::serenity_prelude as serenity;
use std::{collections::HashMap, env::var, sync::Mutex, time::Duration};
use thiserror;

mod commands;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, (), Error>;

async fn on_error(error: poise::FrameworkError<'_, (), Error>) {
    let err_message = match &error {
        poise::FrameworkError::Setup { error, .. } => format!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx } => {
            format!("Error in command `{}`: {:?}", ctx.command().name, error,)
        }
        _ => String::from("Something bad happened and I am too lazy to re-route it to the bot sorry"),
    };

    match error.ctx() {
        Some(ctx) => ctx
            .send(|m| {
                m.embed(|e| {
                    e.color(0x000000)
                        .title("Error occured whilst running command!")
                        .description(err_message)
                })
            })
            .await
            .expect("how the hell did I somehow error whilst sending the error"),
        None => panic!("{err_message}"),
    };
}

#[tokio::main]
async fn main() {
    // env_logger::init();

    let options = poise::FrameworkOptions {
        commands: vec![commands::pazeify()],
        on_error: |error| Box::pin(on_error(error)),
        event_handler: |_ctx, event, _framework, _data| {
            Box::pin(async move {
                println!("Got an event in event handler: {:?}", event.name());
                Ok(())
            })
        },

        ..Default::default()
    };

    poise::Framework::builder()
        .token(var("DISCORD_TOKEN").expect("please provide a DISCORD_TOKEN environment variable!"))
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                println!("Logged in as: {}", _ready.user.name);
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(())
            })
        })
        .options(options)
        .intents(serenity::GatewayIntents::non_privileged())
        .run()
        .await
        .unwrap();
}
