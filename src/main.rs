mod google;

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

    match google::retreive_playlists(&client, &access_token) {
        Ok(_) => (),
        Err(_) => {
            let refresh_token = Entry::new("yt-randomizer", "refresh").unwrap().get_password().unwrap();
            if let Ok(access_token) = google::refresh_access_token(&client, &refresh_token) {
                keyring_entry.set_password(&access_token).unwrap();

                let playlists = google::retreive_playlists(&client, &access_token).unwrap();

                println!("Found {} playlists", playlists.len());
            }
        }
    }
}
