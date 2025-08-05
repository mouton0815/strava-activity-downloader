# Docker Desktop
```shell
docker volume create activity-volume
```

```shell
docker image build --tag activity-server .
```

```shell
docker container run \
  --publish 2525:2525 --detach \
  --name activity-server \
  --volume activity-volume:/app/data \
  --env DATA_DIR=/app/data \
  --env RUST_LOG=info \
  activity-server
```

# Google Cloud
```shell
gcloud config set project activity-server-468119
```

```shell
gcloud storage buckets create gs://mouton0815-activity-bucket --project=activity-server-468119
```

```shell
gcloud builds submit --tag gcr.io/activity-server-468119/activity-server
```

```shell
gcloud run deploy activity-server \
  --image gcr.io/activity-server-468119/activity-server \
  --platform managed \
  --region europe-west1 \
  --allow-unauthenticated \
  --add-volume=name=activity-volume,type=cloud-storage,bucket=mouton0815-activity-bucket \
  --add-volume-mount=volume=activity-volume,mount-path=/app/data \
  --set-env-vars REDIRECT_URL=https://activity-server-775683106576.europe-west1.run.app \
  --set-env-vars DATA_DIR=/app/data \
  --set-env-vars RUST_LOG=info
```