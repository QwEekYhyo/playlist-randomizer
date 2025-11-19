mod google;

use std::io::Write;

use keyring::{Entry, Error::NoEntry};

fn main() {
    let keyring_entry = Entry::new("yt-randomizer", "access").unwrap();
    let client = reqwest::blocking::Client::new();

    dotenvy::dotenv().unwrap();

    let access_token = match keyring_entry.get_password() {
        Ok(p) => p,
        Err(NoEntry) => {
            let (token, refresh_token) = google::perform_oauth(&client);
            keyring_entry.set_password(&token).unwrap();
            let keyring_entry_refresh = Entry::new("yt-randomizer", "refresh").unwrap();
            keyring_entry_refresh.set_password(&refresh_token).unwrap();
            token
        }
        Err(e) => Err(e).unwrap(),
    };

    println!("access_token {access_token}");

    let playlists = match google::retreive_playlists(&client, &access_token) {
        Ok(playlists) => playlists,
        Err(_) => {
            let refresh_token = Entry::new("yt-randomizer", "refresh").unwrap().get_password().unwrap();
            if let Ok(access_token) = google::refresh_access_token(&client, &refresh_token) {
                keyring_entry.set_password(&access_token).unwrap();

                google::retreive_playlists(&client, &access_token).unwrap()
            } else {
                panic!("Error trying to refresh access token which needs to be handled and will probably be in the near future");
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

    let chosen_playlist = &playlists[index-1];
    println!("Chose {}, with id: {}", chosen_playlist.snippet.title, chosen_playlist.id);
}
