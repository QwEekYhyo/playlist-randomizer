use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use color_eyre::{
    Result,
    eyre::{Context, eyre},
};
use rand::{Rng, distr::Alphanumeric, seq::SliceRandom};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::env;
use url::Url;

pub struct GogolClient {
    http_client: reqwest::blocking::Client,
    client_id: String,
    client_secret: String,
}

impl GogolClient {
    pub fn new() -> Result<Self> {
        let client_id = env::var("CLIENT_ID").wrap_err("CLIENT_ID not set")?;
        let client_secret = env::var("CLIENT_SECRET").wrap_err("CLIENT_SECRET not set")?;

        Ok(Self {
            http_client: reqwest::blocking::Client::new(),
            client_id,
            client_secret,
        })
    }
}

#[derive(Debug, Deserialize)]
struct GogolResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub scope: String,
    pub token_type: String,
    pub refresh_token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: String,
    pub snippet: PlaylistSnippet,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistSnippet {
    pub title: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlaylistList {
    pub next_page_token: Option<String>,
    pub page_info: PlaylistPageInfo,
    pub items: Vec<Playlist>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlaylistPageInfo {
    pub total_results: usize,
    pub results_per_page: usize,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlaylistItem {
    pub id: String,
    pub snippet: PlaylistItemSnippet,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlaylistItemSnippet {
    #[serde(skip_serializing)]
    pub title: String,
    pub position: usize,
    pub playlist_id: String,
    pub resource_id: PlaylistItemResourceId,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlaylistItemResourceId {
    pub kind: String,
    pub video_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlaylistItemList {
    pub next_page_token: Option<String>,
    pub page_info: PlaylistPageInfo,
    pub items: Vec<PlaylistItem>,
}

impl std::fmt::Display for PlaylistItemList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Here are the items in the playlist\n")?;

        for (index, playlist_item) in self.items.iter().enumerate() {
            write!(
                f,
                "{}. {}",
                playlist_item.snippet.position, playlist_item.snippet.title
            )?;
            if index != self.page_info.total_results - 1 {
                write!(f, "\n")?;
            }
        }

        Ok(())
    }
}

// I used this for convenience, it's not used anymore because
// I need to also display the index in the TOTAL list of playlist
// not just in the subset of the Google "page"
impl std::fmt::Display for PlaylistList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for playlist in &self.items {
            write!(f, "Playlist Title: {}\n", playlist.snippet.title)?;
        }

        Ok(())
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GetPlaylistParams<'a> {
    pub part: &'a str,
    pub mine: &'a str,
    pub page_token: Option<&'a str>,
}

fn print_playlist_subset(playlist_list: &PlaylistList, index: &mut usize) {
    for playlist in &playlist_list.items {
        println!("{}. {}", index, playlist.snippet.title);
        *index += 1;
    }
}

const PLAYLIST_URL: &str = "https://www.googleapis.com/youtube/v3/playlists";
const PLAYLIST_ITEMS_URL: &str = "https://www.googleapis.com/youtube/v3/playlistItems";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

impl GogolClient {
    pub fn refresh_access_token(&self, refresh_token: &str) -> Result<String> {
        let mut form = std::collections::HashMap::new();
        form.insert("client_id", self.client_id.as_str());
        form.insert("client_secret", self.client_secret.as_str());
        form.insert("grant_type", "refresh_token");
        form.insert("refresh_token", refresh_token);

        let body: GogolResponse = self
            .http_client
            .post(TOKEN_URL)
            .form(&form)
            .send()?
            .json()?;

        Ok(body.access_token)
    }

    pub fn perform_oauth(&self) -> (String, String) {
        let scopes = [
            "https://www.googleapis.com/auth/youtubepartner",
            "https://www.googleapis.com/auth/youtube",
            "https://www.googleapis.com/auth/youtube.force-ssl",
        ];
        let server_uri = if env::var("IN_DOCKER").is_ok() {
            println!("IN DOCKEERRRRR");
            "0.0.0.0:8080"
        } else {
            "127.0.0.1:8080"
        };
        let redirect_uri = "http://127.0.0.1:8080";
        let auth_url = "https://accounts.google.com/o/oauth2/v2/auth";

        let code_verifier: String = rand::rng()
            .sample_iter(&Alphanumeric)
            .take(128)
            .map(char::from)
            .collect();
        let code_challenge = {
            let hash = Sha256::digest(code_verifier.as_bytes());
            URL_SAFE_NO_PAD.encode(hash)
        };

        println!("code_verifier: {}", code_verifier);
        println!("code_challenge: {:?}", code_challenge);

        let state: String = rand::rng()
            .sample_iter(&Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();

        let mut url = Url::parse(auth_url).unwrap();
        url.query_pairs_mut()
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", &redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", &scopes.join(" "))
            .append_pair("code_challenge", &code_challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("access_type", "offline")
            .append_pair("prompt", "consent")
            .append_pair("state", &state);

        println!("Open this URL in your browser: {}", url);

        let server = tiny_http::Server::http(server_uri).unwrap();

        let (returned_state, code) = loop {
            let request = server.recv().unwrap();

            let url = Url::parse(&format!("http://localhost{}", request.url())).unwrap();
            let code = url
                .query_pairs()
                .find(|(k, _)| k == "code")
                .map(|(_, v)| v.to_string());
            let returned_state = url
                .query_pairs()
                .find(|(k, _)| k == "state")
                .map(|(_, v)| v.to_string());

            if let Some(truc) = code {
                let response = tiny_http::Response::from_string("You can close this tab now.");
                let _ = request.respond(response);
                break (returned_state, truc);
            }
        };

        if returned_state.unwrap_or_default() != state {
            panic!("We got fooled by CSRF!");
        }

        println!("Authorization code: {:?}", code);

        let mut form = std::collections::HashMap::new();
        form.insert("code", code.as_str());
        form.insert("client_id", &self.client_id);
        form.insert("client_secret", &self.client_secret);
        form.insert("redirect_uri", &redirect_uri);
        form.insert("grant_type", "authorization_code");
        form.insert("code_verifier", &code_verifier);

        let body: GogolResponse = self
            .http_client
            .post(TOKEN_URL)
            .form(&form)
            .send()
            .unwrap()
            .json()
            .unwrap();

        if body.refresh_token.is_none() {
            panic!("Google are assholes and did not provide a refresh token");
        }

        (body.access_token, body.refresh_token.unwrap())
    }

    fn get_playlist(&self, access_token: &str, page_token: Option<&str>) -> Result<PlaylistList> {
        let params = GetPlaylistParams {
            part: "snippet",
            mine: "true",
            page_token,
        };

        let response = self
            .http_client
            .get(PLAYLIST_URL)
            .header("Authorization", format!("Bearer {access_token}"))
            .query(&params)
            .send()?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            Err(eyre!("Need to refresh the access token"))
        } else {
            Ok(response.json()?)
        }
    }

    // This also prints them
    // Also there is a "weak binding" between indices in the resulting Vec and the ones shown by the
    // print that relies on the fact that Vec::append preserves the order of elements
    pub fn retreive_playlists(&self, access_token: &str) -> Result<Vec<Playlist>> {
        let mut body = self.get_playlist(&access_token, None)?;
        let mut index = 1;

        print_playlist_subset(&body, &mut index);

        let mut playlists = Vec::new();
        playlists.append(&mut body.items);

        while let Some(page_token) = &body.next_page_token {
            // get_playlist below shouldn't error
            // unless of course we lost internet connection between requests
            body = self.get_playlist(&access_token, Some(page_token))?;
            print_playlist_subset(&body, &mut index);
            playlists.append(&mut body.items);
        }

        Ok(playlists)
    }

    pub fn shuffle_playlist(&self, access_token: &str, playlist: &Playlist) -> Result<()> {
        let mut body: PlaylistItemList = self
            .http_client
            .get(PLAYLIST_ITEMS_URL)
            .header("Authorization", format!("Bearer {access_token}"))
            .query(&[
                ("part", "snippet"),
                ("playlistId", &playlist.id),
                ("maxResults", "50"),
            ])
            .send()?
            .json()?;

        println!("{body}");

        // I don't know if this is the most efficient way to do it as I am iterating all elements
        // afterwards to set their position and perform the update request
        body.items.shuffle(&mut rand::rng());

        println!("{body}");

        // Apply new positions
        for (pos, playlist_item) in body.items.iter_mut().enumerate() {
            playlist_item.snippet.position = pos;
            let response = self
                .http_client
                .put(PLAYLIST_ITEMS_URL)
                .header("Authorization", format!("Bearer {access_token}"))
                .query(&[("part", "snippet")])
                .json(playlist_item)
                .send();

            match response {
                Ok(_) => println!("Item updated"),
                Err(_) => println!("Error while updating"),
            }
        }

        Ok(())
    }
}
