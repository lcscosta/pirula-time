const Https = require('https')
const QueryString = require('querystring')
const Secrets = require('./secrets.js')
const Aws = require('aws-sdk')

const playlistItemsUrl = 'https://www.googleapis.com/youtube/v3/playlistItems?'
const videosUrl = 'https://www.googleapis.com/youtube/v3/videos?'
const channelId = 'UUdGpd0gNn38UKwoncZd9rmA'

Aws.config.update({region: 'sa-east-1'})
const dynamodb = new Aws.DynamoDB.DocumentClient({apiVersion: '2012-08-10'});
const dbPut = Util.promisify(dynamodb.put)
const data = {}

const durationRe = /^PT(?:(\d+)H)?(?:(\d+)M)?(?:(\d+)S)?$/
function parseDuration(str) {
  const data = durationRe.exec(str).slice(1)
  seconds = Number.parseInt(data.pop()) || 0
  minutes = Number.parseInt(data.pop()) || 0
  hours = Number.parseInt(data.pop()) || 0

  return hours*60*60 + minutes*60 + seconds
}

function extractVideoIds(data) {
  const ids = data.items.map(item => {
    return item.contentDetails.videoId
  })
  return ids.join(',')
}

function gatherLatestData(videoData) {
  data.latestDuration = parseDuration(videoData.items[0].contentDetails.duration).toString()
  data.latestHate = videoData.items[0].statistics.dislikeCount
  data.latestVideos = []

  videoData.items.slice(0, 6).forEach((item, i) => {
    obj = {
      'id': i,
      'title': item.snippet.title,
      'url': `https://www.youtube.com/watch?v=${item.id}`
    }
    dislikeCount = Number(item.statistics.dislikeCount) || 0
    likeCount = Number(item.statistics.likeCount) || 0
    obj['quality'] = likeCount > dislikeCount*3

    data.latestVideos.push(obj)
  })
}

function sumVideoDurations(data) {
  let sum = 0
  data.items.forEach(item => {
    sum += parseDuration(item.contentDetails.duration)
  })
  return sum
}

function scrape() {

  let playlistItemsParamsObj = {
    key: Secrets.googleApiKey,
    playlistId: channelId,
    maxResults: 50,
    part: 'contentDetails',
    fields: 'items/contentDetails/videoId,nextPageToken,pageInfo'
  }

  let videosDurationParamsObj = {
    key: Secrets.googleApiKey,
    part: 'contentDetails, statistics, snippet',
    fields: 'items(contentDetails/duration, id, snippet/title, statistics(dislikeCount, likeCount))'
  }

  let nextPageToken = null
  let totalDuration = 0

  // parse only the first 200 videos
  // 50 per page
  let maxVideos = 200

  const sumVideoInfo = (videoCount = 0) => {
    if (videoCount < maxVideos) {

      return new Promise((resolve, reject) => {
        if (videoCount == 0 || nextPageToken) {
          playlistItemsParamsObj.nextPageToken = nextPageToken
          const playlistItemsParams = QueryString.stringify(playlistItemsParamsObj)
          const videoIdsUrl = `${playlistItemsUrl}${playlistItemsParams}`

          new Promise((resolve, reject) => {
            Https.get(videoIdsUrl, res => {
              res.setEncoding('utf8');
              let rawData = '';
              res.on('data', chunk => rawData += chunk);
              res.on('end', () => resolve(rawData))
            })
          }).then(videoIdsData => {
            new Promise((resolve, reject) => {
              videoIdsData = JSON.parse(videoIdsData)
              nextPageToken = videoIdsData.nextPageToken

              const videoIds = extractVideoIds(videoIdsData)
              videosDurationParamsObj.id = videoIds
              const videosDurationParams = QueryString.stringify(videosDurationParamsObj)
              const videosDurationUrl = `${videosUrl}${videosDurationParams}`

              Https.get(videosDurationUrl, res => {
                res.setEncoding('utf8');
                let rawData = '';
                res.on('data', chunk => rawData += chunk);
                res.on('end', () => resolve(rawData))
              })
            }).then(videoData => {
              videoData = JSON.parse(videoData)
              totalDuration += sumVideoDurations(videoData)

              // latest data is available on the first page
              if (videoCount == 0) {
                gatherLatestData(videoData)
              }

              videoCount += videoData.items.length
              resolve(sumVideoInfo(videoCount))
            })
          })
        }
      })
    } else {
      return videoCount
    }
  }

  sumVideoInfo().then(videoCount => handleData(videoCount))
}

function handleData(videoCount) {
  data.averageDuration = Math.ceil(totalDuration / videoCount).toString()
  const now = Math.round(new Date().getTime()/1000)
  const ttl = now + 604800 // 7 days

  const averageData = {
    TableName: 'Data-dev',
    Item: {
      'id': 0, 'timestamp': now,
      'latestVideos': data.latestVideos,
      'latestHate': data.latestHate,
      'averageDuration': data.averageDuration,
      'latestDuration': data.latestDuration,
      'ttl': ttl,
    },
  }

  // save the average data
  dynamodb.put(averageData, function(err, data) {
    if (err) console.log(err);
    else console.log(data);
  });

  // query the saved durations
  new Promise((resolve, reject) => {
    const params = {
      TableName: 'Duration',
      Limit: 30,
      ScanIndexForward: false,
      KeyConditionExpression: 'id = :id',
      ExpressionAttributeValues: { ':id': 0 }
    }
    dynamodb.query(params, function(err, data) {
      if (err) reject(err)
      else resolve(data.Items)
  }).then(durationData => {

    // update graph if there's a new video
    // TODO detect a new video by using it's id instead of duration
    if (latestDuration != durationData[0].duration) {
      const graphUrl = createNewGraph(durationData)
      const data = {
        TableName: 'Duration',
        Item: {
          'id': 0,
          'duration': latestDuration,
          'graphUrl': graphUrl,
        }
      }
      dynamodb.put(data, function(err, data) {
        if (err) console.log(err)
        else console.log(data)
      })
    }
  })
}

exports.doIt = function doIt(req, res) {
// exports.doIt = function doIt(event, context, callback) {
  scrape()
  // callback(null, 'scraping')
  res.end('scraping')
}

const http = require('http');
const server = http.createServer((req, res) => {
  if (req.url == '/scrape') {
    this.doIt(req, res)
  } else {
    res.end()
  }
});

const hostname = '127.0.0.1';
const port = 3000;
server.listen(port, hostname, () => {
  console.log(`Server running at http://${hostname}:${port}/`);
});
