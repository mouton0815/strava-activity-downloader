# Strava Activity Downloader

This is a Rust server that downloads all Strava activities and the corresponding activity streams (tracks)
for the authenticated user. The activities are stored in local a [SQLite 3](https://www.sqlite.org) database;
the activity streams are written to GPX files.

The downloader respects the rate limits defined for your Strava API client.
By default, the built-in scheduler issues a request every 10 seconds.
The delay can be adapted in the server configuration.
When the daily request limit is reached, the scheduler suspends.
The downloading can be resumed later from point where it stopped.
This also works after a server restart.

The server can be controlled by a React UI, which is also part of this project. 
It takes care of authenticating the application with Strava, lets you start and stop the downloading,
and shows the download progress.

TODO: Screenshot

## Installation
You need Rust with `cargo` for the server
and [Node.js](https://docs.npmjs.com/downloading-and-installing-node-js-and-npm) with `npm` for the UI.


TODOs
* Remove application.yml and change credentials
* Move server part to subdir
* Server Sent Events
* Download GPX
* Count downloaded GPXs and show in UI
