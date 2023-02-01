use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use discord_rich_presence::activity::Assets;
use vulkano::command_buffer::PrimaryCommandBufferAbstract;

/// The discord application's client ID.
const CLIENT_ID: &'static str = "1070156213892947978";
/// Sets up the discord rich presence.
pub fn setup_rich_presence() -> Result<DiscordIpcClient, Box<dyn std::error::Error>>{
    let mut client = DiscordIpcClient::new(CLIENT_ID)?;

    client.connect()?;
    client.set_activity(activity::Activity::new()
        .state("Loading")
        .details("Automancy is not released yet, go to https://gamedev.lgbt/@automancy to see the development progress")
    )?;
    Ok(client)
}
/// Sets the current status of the player here.
pub fn set_status(client: &mut DiscordIpcClient, status: DiscordStatuses) -> Result<(), Box<dyn std::error::Error>> {
    client.set_activity(activity::Activity::new()
        .state(match status {
            DiscordStatuses::MainMenu => {"In the main menu"}
            DiscordStatuses::InGame => {"In game"}
        })
        .assets(Assets::new().large_image("logo"))
        .details("https://gamedev.lgbt/@automancy"))
}
pub enum DiscordStatuses {
    MainMenu,
    InGame
}