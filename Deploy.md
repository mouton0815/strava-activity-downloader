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
gcloud config set project <your-project>
```

```shell
gcloud storage buckets create gs://<your-bucket> --project=<your-project>
```

```shell
gcloud builds submit --tag gcr.io/<your-project>/activity-server
```

```shell
gcloud run deploy activity-server \
  --image gcr.io/<your-project>/activity-server \
  --platform managed \
  --region <your-region> \
  --allow-unauthenticated \
  --add-volume=name=activity-volume,type=cloud-storage,bucket=<your-bucket> \
  --add-volume-mount=volume=activity-volume,mount-path=/app/data \
  --set-env-vars REDIRECT_URL=<url-assigned-by-gcloud> \
  --set-env-vars DATA_DIR=/app/data \
  --set-env-vars RUST_LOG=info
```
You need to deploy twice in order to obtain the `<url-assigned-by-gcloud>`.
For the first time, just pass a dummy value.