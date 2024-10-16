import sqlite3
import os
from datetime import datetime, timedelta
from time import gmtime, strftime

from dotenv import load_dotenv
import pandas as pd
import matplotlib.pyplot as plt
from matplotlib.figure import Figure
import matplotlib.dates as mdates


def seconds_to_hms(seconds):
    return str(timedelta(seconds=seconds))


def histogram(db):
    # Create a figure for the histogram
    fig = Figure()
    axis = fig.add_subplot(1, 1, 1)

    # Get the data from the database
    data = db.execute('SELECT duration FROM video_details').fetchall()
    data = [row[0] for row in data]

    axis.hist(data, bins=30, color='skyblue', edgecolor='black')
    ticks = axis.get_xticks()
    axis.set_xticklabels([seconds_to_hms(int(tick)) for tick in ticks])

    # Add labels and title
    axis.set_xlabel('Time (HH:MM:SS)')  # X-axis label
    axis.set_ylabel('Count')          # Y-axis label
    axis.set_title("Histogram of Pirula's Video Durations")  # Title

    fig.savefig("static/histogram.png",dpi=300)

def mean_series(db):
    # Create a figure for the histogram
    fig = Figure((10,5))
    axis = fig.add_subplot(1, 1, 1)

    # Get the data from the database
    y_data = db.execute('SELECT duration FROM video_details').fetchall()
    y_data = [row[0] for row in y_data][::-1]
    x_data = db.execute('SELECT published_at FROM video_details').fetchall()
    x_data = [datetime.fromisoformat(row[0].replace('Z', '+00:00')) for row in x_data][::-1]

    cumulative_means = []
    cumulative_sum = 0

    for i in range(len(y_data)):
        cumulative_sum += y_data[i]
        cumulative_means.append(cumulative_sum / (i + 1))

    axis.plot(x_data, cumulative_means, marker='.', linestyle='-', color='skyblue')
    #
    #axis.set_xticklabels([x_data[i].strftime("%Y-%m-%d") for i in tick_indices], rotation=45)
    # Start date
    start_date = datetime(2006, 1, 1)

    # Current date
    end_date = datetime.now()

    # Create a list of datetime objects for each day from start_date to end_date
    date_array = []

    # Use a while loop to generate each date
    current_date = start_date
    while current_date <= end_date:
        date_array.append(current_date)
        current_date += timedelta(days=1)

    tick_indices = range(0, len(date_array), max(1, len(date_array) // 5))
    axis.set_xticks([date_array[i] for i in tick_indices])
    axis.xaxis.set_major_formatter(mdates.DateFormatter('%Y-%m-%d'))
    plt.setp(axis.xaxis.get_majorticklabels(), rotation=45, ha='right')
    ticks = axis.get_yticks()
    axis.set_yticklabels([seconds_to_hms(int(tick)) for tick in ticks])

    # Add labels and title
    axis.set_xlabel('Date (YYYY-MM-DD)')  # X-axis label
    axis.set_ylabel('Pirula Duration')          # Y-axis label
    axis.set_title('Time Series of Pirula Duration')  # Title

    #axis.xaxis.set_major_formatter(mdates.DateFormatter('%Y-%m-%d'))
    fig.tight_layout()

    fig.savefig("static/mean_series.png",dpi=300)
    


def generate_excel(db):

    all_videos_excel = db.execute('''
        SELECT * FROM video_details
    ''').fetchall()

    videos_excel = []
    for row in all_videos_excel:
        videos_excel.append({
            "id": row[0],
            "title": row[2],
            "duration_seconds": row[3],
            "published_at": row[4]
        })

    df = pd.DataFrame(videos_excel)
    df.to_excel('static/files/pirula_planilha.xlsx', index=False)

def main():
    load_dotenv()

    db = sqlite3.connect(os.getenv('FILEPATH_DATABASE'))

    generate_excel(db)

    histogram(db)

    mean_series(db)

main()