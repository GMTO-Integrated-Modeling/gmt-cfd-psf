use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::process;

#[derive(Deserialize, Debug)]
struct YouTubeItem {
    snippet: YouTubeSnippet,
}

#[derive(Deserialize, Debug)]
struct YouTubeSnippet {
    title: String,
    resource_id: ResourceId,
}

#[derive(Deserialize, Debug)]
struct ResourceId {
    video_id: String,
}

#[derive(Deserialize, Debug)]
struct YouTubeError {
    error: ErrorDetail,
}

#[derive(Deserialize, Debug)]
struct ErrorDetail {
    code: i32,
    message: String,
    errors: Vec<ErrorInfo>,
}

#[derive(Deserialize, Debug)]
struct ErrorInfo {
    message: String,
    domain: String,
    reason: String,
}

fn fetch_playlist_videos(
    api_key: &str,
    playlist_id: &str,
) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let mut next_page_token: Option<String> = None;
    let mut all_videos = Vec::new();
    let mut page_count = 0;

    println!("Fetching videos from playlist: {}", playlist_id);

    loop {
        page_count += 1;
        println!("Fetching page {}...", page_count);

        let mut params = HashMap::new();
        params.insert("part", "snippet");
        params.insert("maxResults", "50");
        params.insert("playlistId", playlist_id);
        params.insert("key", api_key);

        if let Some(token) = &next_page_token {
            params.insert("pageToken", token);
        }

        let response = client
            .get("https://www.googleapis.com/youtube/v3/playlistItems")
            .query(&params)
            .send()?;

        let status = response.status();
        let response_text = response.text()?;

        // First, try to parse as JSON to see what we're dealing with
        let json_value: Value = serde_json::from_str(&response_text)?;

        if !status.is_success() {
            // Try to parse as YouTube error response
            if let Ok(error_response) = serde_json::from_str::<YouTubeError>(&response_text) {
                return Err(format!(
                    "YouTube API error {}: {}",
                    error_response.error.code, error_response.error.message
                )
                .into());
            } else {
                return Err(format!("HTTP {}: {}", status, response_text).into());
            }
        }

        // Check if we have the expected items array
        if let Some(items) = json_value.get("items").and_then(|i| i.as_array()) {
            if items.is_empty() && page_count == 1 {
                println!(
                    "No items found in the first page. This might be an empty playlist or access issue."
                );
            }

            for item in items {
                if let (Some(video_id), Some(title)) = (
                    item.pointer("/snippet/resourceId/videoId")
                        .and_then(|v| v.as_str()),
                    item.pointer("/snippet/title").and_then(|t| t.as_str()),
                ) {
                    all_videos.push((video_id.to_string(), title.to_string()));
                }
            }
        } else {
            println!(
                "Unexpected response format. Full response: {}",
                response_text
            );
            return Err("Response does not contain expected 'items' array".into());
        }

        // Get next page token if it exists
        next_page_token = json_value
            .get("nextPageToken")
            .and_then(|t| t.as_str())
            .map(|s| s.to_string());

        if next_page_token.is_none() {
            break;
        }

        // Small delay to be nice to the API
        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    Ok(all_videos)
}

fn print_debug_info(api_key: &str, playlist_id: &str) {
    println!();
    println!("=== DEBUG INFORMATION ===");
    println!("API Key: {}...", &api_key[0..10]);
    println!("Playlist ID: {}", playlist_id);
    println!(
        "Test URL: https://www.googleapis.com/youtube/v3/playlistItems?part=snippet&maxResults=5&playlistId={}&key={}...",
        playlist_id,
        &api_key[0..10]
    );
    println!();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        println!("Usage: {} <API_KEY> <PLAYLIST_ID>", args[0]);
        println!();
        println!(
            "Example: {} YOUR_API_KEY PLl-K7zZEsYLkPZHe41m4jfAxUi0JjLgSM",
            args[0]
        );
        process::exit(1);
    }

    let api_key = &args[1];
    let playlist_id = &args[2];

    print_debug_info(api_key, playlist_id);

    match fetch_playlist_videos(api_key, playlist_id) {
        Ok(videos) => {
            println!("Successfully fetched {} videos:", videos.len());
            println!("{}", "=".repeat(80));

            for (i, (video_id, title)) in videos.iter().enumerate() {
                println!(r#""{}" : "{}""#, title, video_id);
            }

            println!("{}", "=".repeat(80));
            println!("Total: {} videos", videos.len());
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!();
            eprintln!("Troubleshooting steps:");
            eprintln!("1. Test your API key with this URL in browser:");
            eprintln!(
                "   https://www.googleapis.com/youtube/v3/playlistItems?part=snippet&maxResults=5&playlistId={}&key={}",
                playlist_id, api_key
            );
            eprintln!();
            eprintln!("2. Check if the playlist is public and exists");
            eprintln!("3. Verify your API key has YouTube Data API v3 enabled");
            eprintln!("4. Check API key restrictions in Google Cloud Console");
            process::exit(1);
        }
    }

    Ok(())
}
