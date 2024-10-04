# pirula-time

Measures the average duration of Pirula's videos

This is intended to be run on AWS.

The app is a Python 3.12 Flask App.

The update is called hourly, gathers youtube data from the Google APIs, and stores it on a sqlite; which is then read by the app.

## Contributions/Contribuições

You're welcome to edit if you have any funny jokes.

# Development/Desenvolvimento

Create a .env with your Google API Token. CHANNEL_ID is the Canal do Pirula Youtube Channel ID.

```
GOOGLE_API_KEY=<your-secret>
CHANNEL_ID=UCdGpd0gNn38UKwoncZd9rmA
FLASK_DATABASE=pirula.sqlite
ENVIRONMENT=development
```

Install python dependencies

```bash
poetry install
```

Create database

```bash
flask --app pirula_time init-db
```

Execute Flask application

```bash
flask --app pirula_time run --debug
```

(Optional) To update your database with new videos use:

```bash
flask --app pirula_time update-db
```