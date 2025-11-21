mod google;

use std::io::Write;

use color_eyre::eyre::{Context, bail};
use keyring::{Entry, Error::NoEntry};

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    dotenvy::dotenv().ok();

    let keyring_entry = Entry::new("yt-randomizer", "access").unwrap();
    let client = google::GogolClient::new().wrap_err("Cannot create Google Client")?;

    let mut access_token = match keyring_entry.get_password() {
        Ok(p) => p,
        Err(NoEntry) => {
            let (token, refresh_token) = client.perform_oauth();
            keyring_entry.set_password(&token).unwrap();
            let keyring_entry_refresh = Entry::new("yt-randomizer", "refresh").unwrap();
            keyring_entry_refresh.set_password(&refresh_token).unwrap();
            token
        }
        Err(e) => Err(e).unwrap(),
    };

    println!("access_token {access_token}");

    let playlists = match client.retreive_playlists(&access_token) {
        Ok(playlists) => playlists,
        Err(_) => {
            let refresh_token = Entry::new("yt-randomizer", "refresh")
                .unwrap()
                .get_password()
                .unwrap();

            if let Ok(new_access_token) = client.refresh_access_token(&refresh_token) {
                keyring_entry.set_password(&new_access_token).unwrap();

                access_token = new_access_token;
                client.retreive_playlists(&access_token).unwrap()
            } else {
                bail!(
                    "Error trying to refresh access token which needs to be handled and will probably be in the near future"
                );
            }
        }
    };

    // TODO: Handle 0 playlists
    println!("Found {} playlists", playlists.len());

    let mut input = String::new();

    print!("Please enter a playlist number [1-{}]: ", playlists.len());
    std::io::stdout().flush();

    std::io::stdin()
        .read_line(&mut input)
        .expect("Error while reading line");

    // TODO: watchout for 0
    let index: usize = input.trim().parse().expect("Input is not a valid number");

    let chosen_playlist = &playlists[index - 1];
    println!(
        "Chose {}, with id: {}",
        chosen_playlist.snippet.title, chosen_playlist.id
    );

    client
        .shuffle_playlist(&access_token, chosen_playlist)
        .unwrap();

    Ok(())
}
