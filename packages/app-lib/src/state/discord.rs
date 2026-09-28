use std::sync::mpsc;

use discord_rich_presence::{
    DiscordIpc, DiscordIpcClient,
    activity::{Activity, Assets},
};

use crate::State;

enum Presence {
    Activity {
        text: String,
        reconnect_if_fail: bool,
    },
    Clear,
}

/// Talks to Discord from a thread of its own. Every call into
/// `discord-rich-presence` is a blocking read on the IPC pipe with no timeout,
/// and a Discord that accepts the pipe but never answers the handshake holds
/// that read forever — so nothing here is ever awaited: callers queue what the
/// presence should be and move on, and a stuck pipe only stalls the presence.
pub struct DiscordGuard {
    sender: mpsc::Sender<Presence>,
}

impl DiscordGuard {
    /// Starts the Discord thread. It connects on the first activity it is
    /// given, and never just to clear one.
    pub fn init() -> crate::Result<DiscordGuard> {
        // Noctrinth's own Discord application, registered at
        // https://discord.com/developers/applications — not Modrinth's, which
        // is where the name Discord shows on the presence comes from.
        //
        // The application is also where the picture comes from: `large_image`
        // below is a key into that application's Rich Presence → Art Assets,
        // not a file in this repository, and a key with nothing uploaded under
        // it draws no picture at all rather than failing in any visible way.
        let client = DiscordIpcClient::new("1505548256585846974");

        let (sender, receiver) = mpsc::channel();
        std::thread::Builder::new()
            .name("discord-rpc".into())
            .spawn(move || run(client, receiver))?;

        Ok(DiscordGuard { sender })
    }

    /// Set the activity to the given message, or clear it if Discord RPC is
    /// disabled in the settings
    pub async fn set_activity(
        &self,
        msg: &str,
        reconnect_if_fail: bool,
    ) -> crate::Result<()> {
        let state = State::get().await?;
        let settings = crate::state::Settings::get(&state.pool).await?;
        let _ = self.sender.send(if settings.discord_rpc {
            Presence::Activity {
                text: msg.to_string(),
                reconnect_if_fail,
            }
        } else {
            Presence::Clear
        });
        Ok(())
    }

    /// Clear the activity, but if there is a running profile, set the activity to that instead
    pub async fn clear_to_default(
        &self,
        reconnect_if_fail: bool,
    ) -> crate::Result<()> {
        let state = State::get().await?;
        let running_instances = state.process_manager.get_all();
        if let Some(existing_child) = running_instances.first() {
            self.set_activity(
                &format!("Playing {}", existing_child.instance_name),
                reconnect_if_fail,
            )
            .await
        } else {
            self.set_activity("Idling...", reconnect_if_fail).await
        }
    }
}

fn run(mut client: DiscordIpcClient, receiver: mpsc::Receiver<Presence>) {
    let mut connected = false;
    while let Ok(mut presence) = receiver.recv() {
        // Only the newest request matters once the thread has fallen behind.
        while let Ok(newer) = receiver.try_recv() {
            presence = newer;
        }

        match presence {
            Presence::Activity {
                text,
                reconnect_if_fail,
            } => {
                // The client can panic when used without ever having connected.
                if !connected {
                    connected = client.connect().is_ok();
                    if !connected {
                        continue;
                    }
                }

                let activity = Activity::new().state(&text).assets(
                    Assets::new()
                        .large_image("noctrinth_simple")
                        .large_text("Noctrinth"),
                );
                if client.set_activity(activity.clone()).is_err()
                    && reconnect_if_fail
                {
                    connected = client.reconnect().is_ok()
                        && client.set_activity(activity).is_ok();
                }
            }
            Presence::Clear => {
                if connected && client.clear_activity().is_err() {
                    connected = client.reconnect().is_ok()
                        && client.clear_activity().is_ok();
                }
            }
        }
    }
}
