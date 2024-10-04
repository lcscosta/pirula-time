DROP TABLE IF EXISTS videos;

CREATE TABLE videos (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  created TIMESTAMP NOT NULL,
  title TEXT NOT NULL,
  duration INTEGER
);

DROP TABLE IF EXISTS statistics;

CREATE TABLE statistics (
  mean_time_sec INTEGER,
  stddev_time_sec INTEGER,
  total_time_sec INTEGER,
  number_of_videos INTEGER,
  last_updated TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);