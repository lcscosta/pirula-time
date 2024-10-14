#[macro_use] extern crate rocket;

use reqwest::Client;
use rocket::get;
use rocket::http::Status;
use rusqlite::{params, Connection, Result};
use serde_json::Value;
use std::error::Error;
use dotenv::dotenv;
use std::env;

fn create_table(conn: &Connection) -> Result<(), Box<dyn Error>> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS video_details (
            id INTEGER PRIMARY KEY,
            video_id TEXT NOT NULL,
            video_title TEXT NOT NULL,
            duration INTEGER NOT NULL,
            published_at TEXT NOT NULL
        )",
        [],
    )?;
    Ok(())
}

fn insert_video_details(conn: &Connection, videos: &[VideoDetail]) -> Result<(), Box<dyn Error>> {
    let mut stmt = conn.prepare("INSERT INTO video_details (video_id, video_title, duration, published_at) VALUES (?1, ?2, ?3, ?4)")?;

    for video in videos {
        stmt.execute(params![video.video_id, video.video_title, video.duration, video.published_at])?;
    }
    Ok(())
}

async fn get_video_details(client: &Client, api_key: &str, video_ids: Vec<String>) -> Result<Vec<VideoDetail>, Box<dyn Error>> {
    let mut all_video_details = Vec::new();
    let batch_size = 50; // Maximum number of IDs per request

    // Process video IDs in batches
    for chunk in video_ids.chunks(batch_size) {
        let batch_ids = chunk.join(",");

        let response: Value = client
            .get(format!(
                "https://www.googleapis.com/youtube/v3/videos?part=snippet,contentDetails&id={}&key={}",
                batch_ids, api_key
            ))
            .send()
            .await?
            .json()
            .await?;

        // Collect video details from this batch
        for item in response["items"].as_array().unwrap_or(&vec![]) {
            let video_id = item["id"].as_str().unwrap_or("").to_string();
            // Attempt to extract video title and other details, handling potential errors
            let video_title = match item["snippet"]["title"].as_str() {
                Some(title) => title.to_string(),
                None => {
                    eprintln!("Warning: Title not found for video ID: {}", video_id);
                    "".to_string() // Default value or handle as needed
                }
            };

            let published_at = match item["snippet"]["publishedAt"].as_str() {
                Some(date) => date.to_string(),
                None => {
                    eprintln!("Warning: Published date not found for video ID: {}", video_id);
                    "".to_string() // Default value or handle as needed
                }
            };

            let duration = match item["contentDetails"]["duration"].as_str() {
                Some(duration_str) => duration_str.to_string(),
                None => {
                    eprintln!("Warning: Duration not found for video ID: {}", video_id);
                    "".to_string() // Default value or handle as needed
                }
            };

            // Parse ISO 8601 duration with error handling
            let duration_seconds = match parse_iso_duration(&duration) {
                Ok(seconds) => seconds,
                Err(e) => {
                    eprintln!("Error parsing duration for {}: {}", &duration, e);
                0 // Default value or handle as needed
                }
            };

            // Collect video details (assuming a struct VideoDetail exists)
            all_video_details.push(VideoDetail {
                video_id,
                video_title,
                duration: duration_seconds,
                published_at,
            });
       }
    }

    Ok(all_video_details)
}

fn parse_iso_duration(duration: &str) -> Result<i64, Box<dyn Error>> {
    let duration = duration.trim_start_matches("PT"); // Remove the "P" prefix
    let mut seconds = 0;

    let mut hours = 0;
    let mut minutes = 0;

    if let Some(h_pos) = duration.find('H') {
        let hours_part = &duration[..h_pos];
        hours = hours_part.parse::<i64>()?;
    }

    if let Some(m_pos) = duration.find('M') {
        let minutes_part = if let Some(h_pos) = duration.find('H') {
            &duration[h_pos + 1..m_pos]
        } else {
            &duration[..m_pos]
        };
        minutes = minutes_part.parse::<i64>()?;
    }


    if let Some(s_pos) = duration.find('S') {
        let seconds_part = if let Some(m_pos) = duration.find('M') {
            &duration[m_pos + 1..s_pos] // Between minutes and seconds
        } else if let Some(h_pos) = duration.find('H') {
            &duration[h_pos + 1..s_pos] // Between hours and seconds
        } else {
            &duration[..s_pos] // From start to seconds
        };

        if !seconds_part.is_empty() { // Check for empty string
            let sec: i64 = seconds_part.parse::<i64>()?;
            seconds += sec;
        }
    }

    seconds += hours * 3600 + minutes * 60;
    Ok(seconds)
}

// Struct to hold video details
#[derive(Debug)]
struct VideoDetail {
    video_id: String,
    video_title: String,
    duration: i64,
    published_at: String,
}

async fn get_channel_videos(client: &Client, api_key: &str, channel_id: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let url = format!(
        "https://www.googleapis.com/youtube/v3/channels?part=contentDetails&id={}&key={}",
        channel_id, api_key
    );

    let response: Value = client.get(&url).send().await?.json().await?;
    let playlist_id = response["items"][0]["contentDetails"]["relatedPlaylists"]["uploads"]
        .as_str()
        .ok_or("No uploads playlist found")?;

    let mut videos = Vec::new();
    let mut next_page_token: Option<String> = None;

    loop {
        let mut playlist_url = format!(
            "https://www.googleapis.com/youtube/v3/playlistItems?part=snippet&playlistId={}&maxResults=50&key={}",
            playlist_id,
            api_key
        );

        if let Some(token) = &next_page_token {
            playlist_url.push_str(&format!("&pageToken={}", token));
        }

        let playlist_response: Value = client.get(&playlist_url).send().await?.json().await?;

        for item in playlist_response["items"].as_array().unwrap_or(&vec![]) {
            if let Some(video_id) = item["snippet"]["resourceId"]["videoId"].as_str() {
                videos.push(video_id.to_string());
            }
        }

        next_page_token = playlist_response.get("nextPageToken").and_then(Value::as_str).map(String::from);
        if next_page_token.is_none() {
            break;
        }
    }

    Ok(videos)
}

#[get("/videos")]
async fn videos() -> Result<(), rocket::http::Status> {
    dotenv().ok();

    let api_key = env::var("GOOGLE_API_KEY").map_err(|_| rocket::http::Status::InternalServerError)?;
    let channel_id = env::var("CHANNEL_ID").map_err(|_| rocket::http::Status::InternalServerError)?;
    let filepath_database = env::var("FILEPATH_DATABASE").map_err(|_| rocket::http::Status::InternalServerError)?;
 
    let conn = Connection::open(filepath_database).map_err(|_| Status::InternalServerError)?;

    let client = Client::new();
    let videos = get_channel_videos(&client, &api_key, &channel_id).await.map_err(|_| rocket::http::Status::InternalServerError)?;
    let details = get_video_details(&client, &api_key, videos).await.map_err(|_| rocket::http::Status::InternalServerError)?;

    create_table(&conn).map_err(|_| Status::InternalServerError)?;

     // Insert the video details into the database
    insert_video_details(&conn, &details).map_err(|_| Status::InternalServerError)?;

    println!("Inserted video details successfully.");

    Ok(())
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![videos])
}

