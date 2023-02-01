use std::error::Error;
use std::time::SystemTime;

use discord_rich_presence::activity::{Activity, Assets, Button, Timestamps};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};

/// The discord application's client ID.
static CLIENT_ID: &str = "1070156213892947978";

pub fn default_activity<'a>() -> Activity<'a> {
    Activity::new()
        .assets(Assets::new().large_image("logo"))
        .buttons(vec![Button::new(
            "Find us on Fediverse",
            "https://gamedev.lgbt/@automancy",
        )])
        .details("(WIP) An automation game based on Hexagons.")
}

/// Called at the start of the game and stays.
pub fn start_time() -> Timestamps {
    Timestamps::new().start(
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
    )
}

/// Sets up the discord rich presence.
pub fn setup_rich_presence() -> Result<DiscordIpcClient, Box<dyn Error>> {
    let mut client = DiscordIpcClient::new(CLIENT_ID)?;

    client.connect()?;

    Ok(client)
}

/// Sets the current status of the player here.
pub fn set_status(
    client: &mut DiscordIpcClient,
    start_time: Timestamps,
    status: DiscordStatuses,
) -> Result<(), Box<dyn Error>> {
    client.set_activity(
        default_activity()
            .state(match status {
                DiscordStatuses::MainMenu => "In the main menu",
                DiscordStatuses::InGame => "In game",
            })
            .timestamps(start_time),
    )
}

pub enum DiscordStatuses {
    MainMenu,
    InGame,
}
