# Docker Desktop
```shell
docker image build --tag strava-downloader .
```

```shell
docker container run \
  --publish 2525:2525 --detach \
  --name strava-downloader \
  --env RUST_LOG=info \
  strava-downloader
```

# Google Cloud
```shell
gcloud config set project tiles-466516
```

```shell
gcloud storage buckets create gs://tiles-bucket --project=tiles-466516
```

```shell
gcloud builds submit --tag gcr.io/tiles-466516/strava-downloader
```

```shell
gcloud run deploy strava-downloader \
  --image gcr.io/tiles-466516/strava-downloader \
  --platform managed \
  --region europe-west1 \
  --allow-unauthenticated \
  --add-volume=name=tiles-volume,type=cloud-storage,bucket=mouton0815-tiles-bucket \
  --add-volume-mount=volume=tiles-volume,mount-path=/app/data \
  --set-env-vars REDIRECT_URL=https://strava-downloader-673667804137.europe-west1.run.app \
  --set-env-vars RUST_LOG=info
```