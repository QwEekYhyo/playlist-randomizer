mod google;

use keyring::{Entry, Error::NoEntry};

fn main() {
    let keyring_entry = Entry::new("yt-randomizer", "token").unwrap();
    let client = reqwest::blocking::Client::new();

    dotenvy::dotenv().unwrap();

    let access_token = match keyring_entry.get_password() {
        Ok(p) => p,
        Err(NoEntry) => {
            let token = google::perform_oauth(&client);
            keyring_entry.set_password(&token).unwrap();
            token
        },
        Err(e) => Err(e).unwrap()
    };

    println!("access_token {access_token}");

    google::retreive_playlists(&client, &access_token);
}
