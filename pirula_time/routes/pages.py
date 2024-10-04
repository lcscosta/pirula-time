from datetime import datetime, timedelta
from time import gmtime, strftime
import random
import io

from flask import Blueprint, render_template, make_response, send_from_directory
from matplotlib.backends.backend_agg import FigureCanvasAgg as FigureCanvas
from matplotlib.figure import Figure
import matplotlib.dates as mdates

from pirula_time.database import get_db


# Atronomical Unit in m
AU = 149597870700
# light speed in m/s
C = 299792458
# ISS' orbit period in s
ISS_ORBIT_PERIOD = 5480.4
# distance to closest start in ly
CLOSEST_STAR_DISTANCE = 4.22
# seconds in a year
SECONDS_IN_YEAR = 365.4 * 24 * 3600

bp = Blueprint("home", __name__)

@bp.route("/", methods=["GET"])
def homepage():
    db = get_db()

    stats = dict(db.execute(
        "SELECT mean_time_sec, stddev_time_sec, total_time_sec, number_of_videos, last_updated FROM statistics"
    ).fetchone())
    
    unkowns = {
        'lightPirula': int(C / stats['mean_time_sec'] ),
        'pirulaSun2Earth': round(AU / C / stats['mean_time_sec'], 5),
        'pirulaClosestStar': int(CLOSEST_STAR_DISTANCE * SECONDS_IN_YEAR / stats['mean_time_sec'])
    }

    stats['mean_time_sec'] = strftime("%H:%M:%S", gmtime(stats['mean_time_sec']))
    stats['stddev_time_sec'] = strftime("%H:%M:%S", gmtime(stats['stddev_time_sec']))
    hours, remainder = divmod(stats['total_time_sec'], 3600)
    minutes, seconds = divmod(remainder, 60)
    stats['total_time_sec'] = f"{int(hours):02}:{int(minutes):02}:{int(seconds):02}"
    stats['last_updated'] = stats['last_updated'] + timedelta(hours=-3)

    videos = db.execute(
        "SELECT created, title, duration FROM videos ORDER BY created DESC"
    ).fetchmany(10)

    videos = [dict(row) for row in videos]
    
    for video in videos:
        video['duration'] = strftime("%H:%M:%S", gmtime(video['duration']))

    return render_template("pages/home.html", stats=stats, videos=videos, unkowns=unkowns)

def seconds_to_hms(seconds):
    return str(timedelta(seconds=seconds))

@bp.route('/histogram.png')
def histogram():
    # Create a figure for the histogram
    fig = Figure()
    axis = fig.add_subplot(1, 1, 1)

    # Create a database connection
    db = get_db()

    # Get the data from the database
    data = db.execute('SELECT duration FROM videos').fetchall()
    data = [row[0] for row in data]

    axis.hist(data, bins=30, color='skyblue', edgecolor='black')
    ticks = axis.get_xticks()
    axis.set_xticklabels([seconds_to_hms(int(tick)) for tick in ticks])

    # Add labels and title
    axis.set_xlabel('Time (HH:MM:SS)')  # X-axis label
    axis.set_ylabel('Count')          # Y-axis label
    axis.set_title("Histogram of Pirula's Video Durations")  # Title

    # Convert the plot to PNG and send it as a response
    output = io.BytesIO()
    FigureCanvas(fig).print_png(output)
    return make_response(output.getvalue(), 200, {'Content-Type': 'image/png'})

@bp.route('/mean_series.png')
def mean_series():
    # Create a figure for the histogram
    fig = Figure((10,5))
    axis = fig.add_subplot(1, 1, 1)

    # Create a database connection
    db = get_db()

    # Get the data from the database
    y_data = db.execute('SELECT duration FROM videos').fetchall()
    y_data = [row[0] for row in y_data]
    x_data = db.execute('SELECT created FROM videos').fetchall()
    x_data = [row[0] for row in x_data]

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

    tick_indices = range(0, len(date_array), 250)
    axis.set_xticks([date_array[i] for i in tick_indices])
    axis.set_xticklabels([date_array[i].strftime("%Y-%m-%d") for i in tick_indices], rotation=45)
    ticks = axis.get_yticks()
    axis.set_yticklabels([seconds_to_hms(int(tick)) for tick in ticks])

    # Add labels and title
    axis.set_xlabel('Date (YYYY-MM-DD)')  # X-axis label
    axis.set_ylabel('Pirula Duration')          # Y-axis label
    axis.set_title('Time Series of Pirula Duration')  # Title

    #axis.xaxis.set_major_formatter(mdates.DateFormatter('%Y-%m-%d'))
    fig.tight_layout()

    # Convert the plot to PNG and send it as a response
    output = io.BytesIO()
    FigureCanvas(fig).print_png(output)
    return make_response(output.getvalue(), 200, {'Content-Type': 'image/png'})

@bp.route('/download/<filename>')
def download_file(filename):
    # Serve the file from the 'static/files' directory
    return send_from_directory('static/files', filename, as_attachment=True)