import os

from dotenv import load_dotenv
from flask import Flask

from pirula_time import database
from pirula_time.routes import pages

load_dotenv()

def create_app() -> Flask:
    app = Flask(__name__)
    app.config.from_prefixed_env()
    
    database.init_app(app)
    database.update_app(app)

    app.register_blueprint(pages.bp, url_prefix="/")
    print(f"Current Environment: {os.getenv('ENVIRONMENT')}")
    print(f"Using Database: {app.config.get('DATABASE')}")
    return app