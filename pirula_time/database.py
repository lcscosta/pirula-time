from datetime import datetime
import os
import sqlite3

import click
from dotenv import load_dotenv
from flask import current_app, g
from googleapiclient.discovery import build
import pandas as pd

from pirula_time.api import get_channel_videos, get_video_details, get_videos_statistics

def init_app(app):
    app.teardown_appcontext(close_db)
    app.cli.add_command(init_db_command)

def update_app(app):
    app.teardown_appcontext(close_db)
    app.cli.add_command(update_db_command)

@click.command("update-db")
def update_db_command():
    db = get_db()

    load_dotenv()
    api_key = os.getenv('GOOGLE_API_KEY')
    channel_id = os.getenv('CHANNEL_ID')

    youtube = build('youtube', 'v3', developerKey=api_key)
    videos_id = get_channel_videos(youtube, channel_id)
    videos_details = get_video_details(youtube, videos_id)
    api_videos_details = []
    for idx, video in enumerate(reversed(videos_details)):
        created_at = datetime.fromisoformat(video['publishedAt'].replace("Z", ""))
        api_videos_details.append([idx+1, created_at, video['videoTitle'], video['duration']])

    db_videos_details = db.execute('''
        SELECT * FROM videos
    ''').fetchall()
    db_videos_details = [list(row) for row in db_videos_details]

    if db_videos_details == api_videos_details:
        db.execute('''
            UPDATE statistics SET last_updated = CURRENT_TIMESTAMP
        ''')

        db.commit()
        db.close()
        click.echo("The database is up to date.")
        return

    click.echo("Updating the database...")

    difference = [
        list(it) for it in set(tuple(lst) for lst in api_videos_details) - set(tuple(lst) for lst in db_videos_details)
    ]

    for video in reversed(difference):
        if video[3] != 0:
            db.execute('''
                INSERT INTO videos (created, title, duration) VALUES (?, ?, ?)
            ''', (video[1], video[2], video[3]))

    db_videos_details = db.execute('''
        SELECT * FROM videos
    ''').fetchall()
    db_videos_details = [dict(row) for row in db_videos_details]

    for i in range(len(db_videos_details)):
        db_videos_details[i]['videoId'] = db_videos_details[i]['id']
        db_videos_details[i]['videoTitle'] = db_videos_details[i]['title']
        db_videos_details[i]['publishedAt'] = db_videos_details[i]['created'].isoformat()

    statistics = get_videos_statistics(db_videos_details)
    mean_time_sec, stddev_time_sec, total_time_sec, number_of_videos = statistics
    db.execute('''
        UPDATE statistics SET mean_time_sec = ?, stddev_time_sec = ?, total_time_sec = ?, number_of_videos = ?, last_updated = CURRENT_TIMESTAMP
    ''', (mean_time_sec, stddev_time_sec, total_time_sec, number_of_videos))

    db.commit()
    db.close()

    click.echo("You successfully updated the database!")

@click.command("init-db")
def init_db_command():
    db = get_db()

    with current_app.open_resource("schemas/init.sql") as f:
        db.executescript(f.read().decode("utf-8"))

    load_dotenv()
    api_key = os.getenv('GOOGLE_API_KEY')
    channel_id = os.getenv('CHANNEL_ID')

    youtube = build('youtube', 'v3', developerKey=api_key)
    videos_id = get_channel_videos(youtube, channel_id)
    videos_details = get_video_details(youtube, videos_id)

    for video in reversed(videos_details):
        if video['duration'] != 0:
            created_at = datetime.fromisoformat(video['publishedAt'].replace("Z", ""))  # Convert to datetime
            db.execute('''
                INSERT INTO videos (created, title, duration) VALUES (?, ?, ?)
            ''', (created_at, video['videoTitle'], video['duration']))

    statistics = get_videos_statistics(videos_details)
    mean_time_sec, stddev_time_sec, total_time_sec, number_of_videos = statistics
    db.execute('''
        INSERT INTO statistics 
        (mean_time_sec, stddev_time_sec, total_time_sec, number_of_videos) VALUES (?, ?, ?, ?)
    ''', (mean_time_sec, stddev_time_sec, total_time_sec, number_of_videos))

    db.commit()
    
    all_videos_excel = db.execute('''
        SELECT * FROM videos
    ''').fetchall()

    all_videos_excel = [dict(row) for row in all_videos_excel]

    df = pd.DataFrame(all_videos_excel)
    df.to_excel('pirula_time/static/files/pirula_planilha.xlsx', index=False)

    db.close()

    click.echo("You successfully initialized the database!")

def get_db():
    if "db" not in g:
        g.db = sqlite3.connect(
            current_app.config["DATABASE"],
            detect_types=sqlite3.PARSE_DECLTYPES,
        )
        g.db.row_factory = sqlite3.Row

    return g.db

def close_db(e=None):
    db = g.pop("db", None)

    if db is not None:
        db.close()