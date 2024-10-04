import isodate
import numpy as np

def get_channel_videos(youtube, channel_id):
    # Obtém a lista de uploads do canal
    response = youtube.channels().list(
        part='contentDetails',
        id=channel_id
    ).execute()

    playlist_id = response['items'][0]['contentDetails']['relatedPlaylists']['uploads']

    # Lista todos os vídeos da playlist de uploads
    videos = []
    next_page_token = None
    while True:
        playlist_response = youtube.playlistItems().list(
            part='snippet',
            playlistId=playlist_id,
            maxResults=50,
            pageToken=next_page_token
        ).execute()

        for item in playlist_response['items']:
            video_id = item['snippet']['resourceId']['videoId']
            videos.append(video_id)

        next_page_token = playlist_response.get('nextPageToken')
        if not next_page_token:
            break

    return videos

def get_video_details(youtube, video_ids):
    all_video_details = []
    batch_size = 50  # Maximum number of IDs per request

    # Process video IDs in batches
    for start in range(0, len(video_ids), batch_size):
        end = min(start + batch_size, len(video_ids))
        batch_ids = video_ids[start:end]

        response = youtube.videos().list(
            part='snippet,contentDetails',
            id=','.join(batch_ids)
        ).execute()

        # Collect video details from this batch
        for item in response['items']:
            video_id = item['id']
            video_title = item['snippet']['title']
            published_at = item['snippet']['publishedAt']
            duration = item['contentDetails']['duration']
            # Parse ISO 8601 duration
            duration = isodate.parse_duration(duration)
            # Convert duration to total seconds
            duration_seconds = int(duration.total_seconds())
            all_video_details.append({
                "videoId": video_id, 
                "videoTitle": video_title, 
                "duration": duration_seconds, 
                "publishedAt": published_at
            })

    return all_video_details

def get_videos_statistics(videos_details):
    videos_details = np.array([
        [video["videoId"], video["videoTitle"], video["duration"], video["publishedAt"]]
        for video in videos_details
    ])

    number_of_videos = len(videos_details)
    mean_time_sec = int(videos_details[:,2].astype(int).mean())
    stddev_time_sec = int(videos_details[:,2].astype(int).std())
    total_time_sec = int(sum(videos_details[:,2].astype(int)))

    results = [mean_time_sec, stddev_time_sec, total_time_sec, number_of_videos]
    return results
    