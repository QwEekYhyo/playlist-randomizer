mod google;

use std::io::Write;

use color_eyre::eyre::{Context, bail};
use colored::Colorize;
use keyring::{Entry, Error::NoEntry};

fn clear_stored_tokens() -> color_eyre::Result<()> {
    let access_token_entry = Entry::new("yt-randomizer", "access")?;
    let refresh_token_entry = Entry::new("yt-randomizer", "refresh")?;

    // TODO: revoke token

    for entry in [access_token_entry, refresh_token_entry] {
        if entry.delete_credential().is_ok() {
            println!("Successfully cleared token");
        } else {
            println!("Tried to clear unexisting token");
        }
    }

    Ok(())
}

fn main() -> color_eyre::Result<()> {
    // This shouldn't error
    color_eyre::install()?;

    dotenvy::dotenv().ok();

    let mut args = std::env::args().skip(1);
    if let Some(cmd) = args.next()
        && cmd == "clear"
    {
        clear_stored_tokens()?;
        return Ok(());
    }

    // This shouldn't error
    let keyring_entry = Entry::new("yt-randomizer", "access")?;

    let client = google::GogolClient::new().wrap_err("Cannot create Google Client")?;

    let mut access_token = match keyring_entry.get_password() {
        Ok(p) => p,
        Err(e) => {
            // TODO: test if we can actually access keyring BEFORE that
            // and at least warn the user, maybe ask if they want to try again?
            let (token, refresh_token) = client.perform_oauth();

            if matches!(e, NoEntry) {
                keyring_entry.set_password(&token).unwrap();
                let keyring_entry_refresh =
                    Entry::new("yt-randomizer", "refresh").wrap_err("Could not access keyrings")?;
                keyring_entry_refresh.set_password(&refresh_token).unwrap();
            } else {
                println!(
                    "Warning: could not access keyring, tokens will not be stored between sessions"
                );
            }

            token
        }
    };

    println!("access_token {access_token}");

    let playlists = match client.retreive_playlists(&access_token) {
        Ok(playlists) => playlists,
        // TODO: Only do this when error really is UNAUTHORIZED
        Err(_) => {
            let refresh_token = Entry::new("yt-randomizer", "refresh")
                ?
                .get_password()
                .wrap_err("Error accessing keyring, if you are using the app without it please restart to get a new access token")?;

            if let Ok(new_access_token) = client.refresh_access_token(&refresh_token) {
                keyring_entry.set_password(&new_access_token)?;

                access_token = new_access_token;
                client.retreive_playlists(&access_token)?
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

    println!(
        "{}",
        "[IMPORTANT] Make sure the playlist you choose is manually sorted"
            .red()
            .bold()
    );
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
