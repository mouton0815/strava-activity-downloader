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

<img src="screenshot.png" alt="Screenshot of the web application" style="width:250px;"/>

## Preconditions
#### Required Tools
* Rust with `cargo` for the server.
* [Node.js](https://docs.npmjs.com/downloading-and-installing-node-js-and-npm) with `npm` for the UI.
* Optionally [sqlite3](https://www.sqlite.org) for querying the local DB.

#### Strava API Client
To connect to Strava, you need a Strava API client.
If you don't have one already, you can register a new API client at https://www.strava.com/settings/api.

Make sure that the `Authorization Callback Domain` of your client is `localhost:2525`.

You will need values for `Client ID` and `Client Secret` of your API client for configuring the server (see below).

## Setup

#### Build the Server
```shell
cargo build
```

#### Configure the Server
Create a server configuration from the template:
```shell
cp conf/application.yaml.example conf/application.yaml
```
Then edit `conf/application.yaml` and set the `Client ID` and `Client Secret` of your Strava API client:
```yaml
oauth:
  client_id: "<your-strava-client-id>"
  client_secret: "<your-strava-client-secret>"
```

#### Build the Web UI
```shell
cd web
npm install
npm run build
cd ..
```

## Running

Start the server:
```shell
RUST_LOG=info cargo run 
```
Then point your browser to http://localhost:2525 and start downloading your activities!

#### Dev Mode

It is also possible to run the web UI in vite's [preview mode](https://vitejs.dev/guide/cli#vite-preview).
The preview server runs at port `2020`. To ensure that the Rust server redirects to the vite preview server
after authenticating with Strava, the `target_url` configured in `conf/application.yaml` should be
```yaml
oauth:
  target_url: "http://localhost:2020" # Redirect to after authentication
```
Then start the dev server in another shell (tab):
```shell
cd web
npm run dev
```
Point your browser to http://localhost:2020.