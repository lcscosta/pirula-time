#[macro_use] extern crate rocket;

use chrono::Duration;
use clap::{Parser, Subcommand};
use dotenv::dotenv;
use reqwest::Client;
use rocket::fs::{FileServer, relative};
use rocket::http::Status;
use rocket::get;
use rocket_dyn_templates::Template;
use rusqlite::{params, Connection, Result, Row};
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::env;

fn create_db(conn: &Connection) -> Result<(), Box<dyn Error>> {

    conn.execute(
        "DROP TABLE IF EXISTS video_details",
        [],
    )?;

    conn.execute(
        "DROP TABLE IF EXISTS statistics",
        [],
    )?;

    conn.execute(
        "DROP TABLE IF EXISTS unkowns",
        [],
    )?;


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

    conn.execute(
        "CREATE TABLE IF NOT EXISTS statistics (
            mean_duration TEXT NOT NULL,
            std_duration TEXT NOT NULL,
            total_duration TEXT NOT NULL,
            total_count INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS unkowns (
            speed_of_light TEXT NOT NULL,
            time_sun_earth TEXT NOT NULL,
            closest_star TEXT NOT NULL
        )",
        [],
    )?;

    Ok(())
}

fn db_insert_video_details(conn: &Connection, videos: &[VideoDetail]) -> Result<(), Box<dyn Error>> {
    let mut stmt = conn.prepare("INSERT INTO video_details (video_id, video_title, duration, published_at) VALUES (?1, ?2, ?3, ?4)")?;

    for video in videos {
        stmt.execute(params![video.video_id, video.video_title, video.duration, video.published_at])?;
    }
    Ok(())
}

fn db_insert_statistics(conn: &Connection, stats: &Statistics) -> Result<(), Box<dyn Error>> {
    let mut stmt = conn.prepare(
        "INSERT INTO statistics (mean_duration, std_duration, total_duration, total_count) 
         VALUES (?1, ?2, ?3, ?4)"
    )?;

    stmt.execute(params![
        stats.mean_duration,
        stats.std_duration,
        stats.total_duration,
        stats.total_count,
    ])?;

    Ok(())
}

fn db_insert_unkowns(conn: &Connection, unkowns: &UnkownFacts) -> Result<(), Box<dyn Error>> {
    let mut stmt = conn.prepare(
        "INSERT INTO unkowns (speed_of_light, time_sun_earth, closest_star) 
         VALUES (?1, ?2, ?3)"
    )?;

    stmt.execute(params![
        unkowns.speed_of_light,
        unkowns.time_sun_earth,
        unkowns.closest_star,
    ])?;

    Ok(())
}

fn db_get_video_details(conn: &Connection) -> Result<Vec<VideoDetail>, Box<dyn Error>> {
    let mut stmt = conn.prepare("SELECT video_id, video_title, duration, published_at FROM video_details")?;
    let video_iter = stmt.query_map([], |row| {
        Ok(VideoDetail {
            video_id: row.get(0)?,
            video_title: row.get(1)?,
            duration: row.get(2)?,
            published_at: row.get(3)?,
        })
    })?;

    let mut videos = Vec::new();
    for video in video_iter {
        videos.push(video?);
    }
    Ok(videos)
}

fn db_get_statistics(conn: &Connection) -> Result<Statistics, Box<dyn Error>> {
    let mut stmt = conn.prepare("SELECT mean_duration, std_duration, total_duration, total_count FROM statistics")?;
    let statistics = stmt.query_row([], |row: &Row| {
        Ok(Statistics {
            mean_duration: row.get(0)?,
            std_duration: row.get(1)?,
            total_duration: row.get(2)?,
            total_count: row.get(3)?,
        })
    })?;

    Ok(statistics)
}

fn db_get_unkowns(conn: &Connection) -> Result<UnkownFacts, Box<dyn Error>> {
    let mut stmt = conn.prepare("SELECT speed_of_light, time_sun_earth, closest_star FROM unkowns")?;
    let unkowns = stmt.query_row([], |row: &Row| {
        Ok(UnkownFacts {
            speed_of_light: row.get(0)?,
            time_sun_earth: row.get(1)?,
            closest_star: row.get(2)?,
        })
    })?;

    Ok(unkowns)
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

#[derive(Debug)]
struct Statistics {
    mean_duration: String,
    std_duration: String,
    total_duration: String,
    total_count: i32
}

#[derive(Debug)]
struct UnkownFacts {
    speed_of_light: String,
    time_sun_earth: String,
    closest_star: String
}

fn parse_duration(duration_str: &str) -> Result<i64, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = duration_str.split(':').collect();

    // Parse each part and handle the number of parts to support "HH:MM:SS" or "MM:SS"
    let seconds = match parts.len() {
        3 => {
            let hours: i64 = parts[0].parse().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            let minutes: i64 = parts[1].parse().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            let seconds: i64 = parts[2].parse().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            hours * 3600 + minutes * 60 + seconds
        },
        2 => {
            let minutes: i64 = parts[0].parse().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            let seconds: i64 = parts[1].parse().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            minutes * 60 + seconds
        },
        _ => return Err("Invalid duration format".into()), // Use a string error instead
    };

    Ok(seconds)
}

fn calculate_pirula_unkowns(stats: &Statistics) -> Result<(UnkownFacts), Box<dyn Error>> {
    // Astronomical Unit in m
    let AU = 149597870700.0;
    // light speed in m/s
    let C = 299792458.0;
    // distance to closest start in ly
    let CLOSEST_STAR_DISTANCE = 4.22;
    // seconds in a year
    let SECONDS_IN_YEAR = 365.4 * 24.0 * 3600.0;

    let duration_sec = parse_duration(&stats.mean_duration)?; 

    let unkowns = UnkownFacts {
        speed_of_light:  format!("{:.2}", C / duration_sec as f64),
        time_sun_earth:  format!("{:.2}", (AU / C ) / duration_sec as f64),
        closest_star:  format!("{:.2}", CLOSEST_STAR_DISTANCE * SECONDS_IN_YEAR / duration_sec as f64)
    };

    Ok(unkowns)
}

fn format_duration(duration: Duration) -> String {
    let seconds = duration.num_seconds();
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

fn calculate_pirula_stats(videos: &[VideoDetail]) -> (Duration, Duration, Duration, usize) {

    let total_count = videos.len();

    if total_count == 0 {
        return (Duration::zero(), Duration::zero(), Duration::zero(), 0);
    }

    // Calculate total duration
    let total_duration = videos.iter()
        .map(|video| video.duration)
        .sum::<i64>();

    // Calculate mean duration
    let mean_duration = total_duration as f64 / total_count as f64;

    // Calculate standard deviation
    let variance = videos.iter()
        .map(|video| {
            let diff = video.duration as f64 - mean_duration;
            diff * diff
        })
        .sum::<f64>()
        / total_count as f64;
    let std_deviation = variance.sqrt();

    // Convert to chrono Duration for formatted output
    let total_duration_dur = Duration::seconds(total_duration);
    let mean_duration_dur = Duration::seconds(mean_duration as i64);
    let std_deviation_dur = Duration::seconds(std_deviation as i64);

    (mean_duration_dur, std_deviation_dur, total_duration_dur, total_count)
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

    let filepath_database = env::var("FILEPATH_DATABASE").map_err(|_| rocket::http::Status::InternalServerError)?;
    let conn = Connection::open(filepath_database).map_err(|_| Status::InternalServerError)?;
    let get_videos_db = db_get_video_details(&conn).map_err(|_| rocket::http::Status::InternalServerError)?;

    println!("{:?}", get_videos_db);

    let get_statistics_db = db_get_statistics(&conn).map_err(|_| rocket::http::Status::InternalServerError)?;

    println!("{:?}", get_statistics_db);

    let get_unkowns_db = db_get_unkowns(&conn).map_err(|_| rocket::http::Status::InternalServerError)?;

    println!("{:?}", get_unkowns_db);

    Ok(())
}

async fn create_db_cli() -> Result<(), rocket::http::Status> {
    dotenv().ok();

    let api_key = env::var("GOOGLE_API_KEY").map_err(|_| rocket::http::Status::InternalServerError)?;
    let channel_id = env::var("CHANNEL_ID").map_err(|_| rocket::http::Status::InternalServerError)?;
    let filepath_database = env::var("FILEPATH_DATABASE").map_err(|_| rocket::http::Status::InternalServerError)?;
 
    let conn = Connection::open(filepath_database).map_err(|_| Status::InternalServerError)?;

    let client = Client::new();
    let videos = get_channel_videos(&client, &api_key, &channel_id).await.map_err(|_| rocket::http::Status::InternalServerError)?;
    let details = get_video_details(&client, &api_key, videos).await.map_err(|_| rocket::http::Status::InternalServerError)?;

    create_db(&conn).map_err(|_| Status::InternalServerError)?;

     // Insert the video details into the database
    db_insert_video_details(&conn, &details).map_err(|_| Status::InternalServerError)?;

    println!("Inserted video details successfully.");

    let (mean, std_dev, total, count) = calculate_pirula_stats(&details);
    
    let stats = Statistics {
        mean_duration: format_duration(mean),
        std_duration: format_duration(std_dev),
        total_duration: format_duration(total),
        total_count: count as i32,
    };

    db_insert_statistics(&conn, &stats).map_err(|_| Status::InternalServerError)?;
    
    println!("Inserted statistics successfully.");

    let unkown = calculate_pirula_unkowns(&stats).map_err(|_| Status::InternalServerError)?;

    db_insert_unkowns(&conn, &unkown).map_err(|_| Status::InternalServerError)?;

    println!("Inserted unkowns successfully.");

    Ok(())
}

#[derive(Parser)]
#[command(name = "Pirula-Time Server")]
#[command(about = "Tracking Pirula Time")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Server,
    CreateDb
}

#[get("/")]
async fn index() -> Result<Template, Status> {
    dotenv().ok();

    let filepath_database = env::var("FILEPATH_DATABASE").map_err(|_| rocket::http::Status::InternalServerError)?;
    let conn = Connection::open(filepath_database).map_err(|_| Status::InternalServerError)?;    
    let stats = db_get_statistics(&conn).map_err(|_| Status::InternalServerError)?;
    let unkowns = db_get_unkowns(&conn).map_err(|_| Status::InternalServerError)?;

    let mut context: HashMap<String, String> = HashMap::new();
    context.insert("mean_duration".to_string(), stats.mean_duration);
    context.insert("std_duration".to_string(), stats.std_duration);
    context.insert("total_duration".to_string(), stats.total_duration);
    context.insert("total_count".to_string(), stats.total_count.to_string());
    context.insert("speed_of_light".to_string(), unkowns.speed_of_light);
    context.insert("time_sun_earth".to_string(), unkowns.time_sun_earth);
    context.insert("closest_star".to_string(), unkowns.closest_star);

    Ok(Template::render("index", &context))
}

#[launch]
async fn rocket() -> _ {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Server => {
            rocket::build()
                .mount("/", routes![videos, index])
                .attach(Template::fairing())
                .mount("/static", FileServer::from(relative!("static")))
        },
        Commands::CreateDb => {
            let _ = create_db_cli().await;
            std::process::exit(0);
        }
    }
    
}
