use dotenv::dotenv;
use serenity::{
    async_trait,
    builder::CreateEmbed,
    client::{Context, EventHandler},
    futures::StreamExt,
    model::{
        channel::Message,
        interactions::{
            message_component::{ActionRowComponent, ButtonStyle, InputTextStyle},
            InteractionResponseType,
        },
        prelude::Ready,
    },
    Client,
};
use std::{env, time::Duration};
use tracing_subscriber;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content != "!embed" {
            return;
        }

        let mut msg = msg
            .channel_id
            .send_message(&ctx, |m| {
                m.content("Create and embed with modals");
                m.components(|c| {
                    c.create_action_row(|ar| {
                        ar.create_button(|button| {
                            button
                                .style(ButtonStyle::Primary)
                                .label("Set the title")
                                .custom_id("button_title")
                        })
                    })
                })
            })
            .await
            .unwrap();

        let react = msg.await_component_interaction(&ctx).await.unwrap();
        // remove the component
        msg.edit(&ctx, |m| m.components(|c| c)).await.unwrap();

        react
            .create_interaction_response(&ctx, |r| {
                r.kind(InteractionResponseType::Modal);
                r.interaction_response_data(|d| {
                    d.custom_id("modal_1");
                    d.title("Set the title for the embed");
                    d.components(|c| {
                        c.create_action_row(|ar| {
                            ar.create_input_text(|it| {
                                it.style(InputTextStyle::Short)
                                    .required(true)
                                    .custom_id("title")
                                    .label("Embed title")
                            })
                        })
                    })
                })
            })
            .await
            .unwrap();

        // Await the modal with the title
        let react = match msg
            .await_modal_interaction(&ctx)
            .timeout(Duration::from_secs(30))
            .await
        {
            Some(r) => r,
            None => {
                msg.reply(&ctx, "Timed out").await.unwrap();
                return;
            }
        };

        let title = match react
            .data
            .components
            .get(0)
            .unwrap()
            .components
            .get(0)
            .unwrap()
        {
            ActionRowComponent::InputText(it) => &it.value,
            _ => unreachable!(),
        };
        react.defer(&ctx).await.unwrap();

        let mut emb = CreateEmbed::default();
        emb.title(title);

        msg.edit(&ctx, |m| {
            m.content("Add fields to the")
                .set_embed(emb.clone())
                .components(|c| {
                    c.create_action_row(|ar| {
                        ar.create_button(|b| {
                            b.style(ButtonStyle::Primary)
                                .custom_id("add_field")
                                .label("Add Field")
                        })
                        .create_button(|b| {
                            b.style(ButtonStyle::Success)
                                .custom_id("done")
                                .label("Done")
                        })
                    })
                })
        })
        .await
        .unwrap();

        let mut comp_reactions = msg.await_component_interactions(&ctx).await;
        let mut modal_reactions = msg.await_modal_interactions(&ctx).await;

        loop {
            tokio::select! {
                button = comp_reactions.next() => {
                    let button = button.unwrap();
                    match button.data.custom_id.as_str() {
                        "add_field" => {
                            button.create_interaction_response(&ctx, |r| {
                                r.kind(InteractionResponseType::Modal);
                                r.interaction_response_data(|d| {
                                    d.custom_id("modal_2");
                                    d.title("Add a field to the embed");
                                    d.components(|c| {
                                        c.create_action_row(|ar| {
                                            ar.create_input_text(|it| {
                                                it.style(InputTextStyle::Short)
                                                    .required(true)
                                                    .custom_id("field_title")
                                                    .label("Field Title")
                                            })
                                        });
                                        c.create_action_row(|ar| {
                                            ar.create_input_text(|it| {
                                                it.style(InputTextStyle::Paragraph)
                                                    .required(true)
                                                    .custom_id("field_content")
                                                    .label("Field Content")
                                            })
                                        })
                                    })
                                })
                            }).await.unwrap();
                        },
                        "done" => {
                            button.create_interaction_response(&ctx, |r| {
                                r.kind(InteractionResponseType::UpdateMessage);
                                r.interaction_response_data(|d| {
                                    d
                                        .content("")
                                        .create_embed(|e| { e.0 = emb.0; e } )
                                        .components(|c| c)
                                })
                            }).await.unwrap();
                            break;
                        },
                        _ => unreachable!(),
                    }
                },
                modal = modal_reactions.next() => {
                    let modal = modal.unwrap();
                    let field_title = match modal.data.components.get(0).unwrap().components.get(0).unwrap() {
                        ActionRowComponent::InputText(it) => &it.value,
                        _ => unreachable!(),
                    };
                    let field_content = match modal.data.components.get(1).unwrap().components.get(0).unwrap() {
                        ActionRowComponent::InputText(it) => &it.value,
                        _ => unreachable!(),
                    };
                    emb.field(field_title, field_content, false);
                    modal.create_interaction_response(&ctx, |r| {
                        r.kind(InteractionResponseType::UpdateMessage);
                        r.interaction_response_data(|d| {
                            d.create_embed(|e| { e.0 = emb.clone().0; e } )
                        })

                    }).await.unwrap();
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt::init();
    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    // The Application Id is usually the Bot User Id. It is needed for components
    let application_id: u64 = env::var("APPLICATION_ID")
        .expect("Expected an application id in the environment")
        .parse()
        .expect("application id is not a valid id");

    // Build our client.
    let mut client = Client::builder(token)
        .event_handler(Handler)
        .application_id(application_id)
        .await
        .expect("Error creating client");

    // Finally, start a single shard, and start listening to events.
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
