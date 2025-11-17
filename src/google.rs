use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::{Rng, distr::Alphanumeric};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::env;
use url::Url;

#[derive(Debug, Deserialize)]
struct GogolResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub scope: String,
    pub token_type: String,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Playlist {
    pub id: String,
    pub snippet: PlaylistSnippet,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlaylistSnippet {
    pub title: String,
}

impl std::fmt::Display for PlaylistList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for playlist in &self.items {
            write!(f, "Playlist Title: {}\n", playlist.snippet.title)?;
        }

        Ok(())
    }
}

const PLAYLIST_URL: &str = "https://www.googleapis.com/youtube/v3/playlists";

pub fn perform_oauth(client: &reqwest::blocking::Client) -> String {
    let client_id = env::var("CLIENT_ID").unwrap();
    let client_secret = env::var("CLIENT_SECRET").unwrap();
    let scopes = [
        "https://www.googleapis.com/auth/youtubepartner",
        "https://www.googleapis.com/auth/youtube",
        "https://www.googleapis.com/auth/youtube.force-ssl",
    ];
    let server_uri = "127.0.0.1:8080";
    let redirect_uri = format!("http://{}", server_uri);
    let auth_url = "https://accounts.google.com/o/oauth2/v2/auth";
    let token_url = "https://oauth2.googleapis.com/token";

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
        .append_pair("client_id", &client_id)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", &scopes.join(" "))
        .append_pair("code_challenge", &code_challenge)
        .append_pair("code_challenge_method", "S256")
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
    form.insert("client_id", &client_id);
    form.insert("client_secret", &client_secret);
    form.insert("redirect_uri", &redirect_uri);
    form.insert("grant_type", "authorization_code");
    form.insert("code_verifier", &code_verifier);

    let body: GogolResponse = client
        .post(token_url)
        .form(&form)
        .send()
        .unwrap()
        .json()
        .unwrap();

    body.access_token
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GetPlaylistParams<'a> {
    pub part: &'a str,
    pub mine: &'a str,
    pub page_token: Option<&'a str>,
}

fn get_playlist(
    client: &reqwest::blocking::Client,
    access_token: &str,
    page_token: Option<&str>,
) -> PlaylistList {
    let params = GetPlaylistParams {
        part: "snippet",
        mine: "true",
        page_token,
    };

    client
        .get(PLAYLIST_URL)
        .header("Authorization", format!("Bearer {access_token}"))
        .query(&params)
        .send()
        .unwrap()
        .json()
        .unwrap()
}

pub fn retreive_playlists(client: &reqwest::blocking::Client, access_token: &str) {
    let mut body: PlaylistList = get_playlist(&client, &access_token, Option::None);

    println!("{body}");

    while let Some(page_token) = &body.next_page_token {
        body = get_playlist(&client, &access_token, Some(page_token));
        println!("{body}");
    }
}
