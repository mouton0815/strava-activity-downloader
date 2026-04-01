# Stage 1: Build the React console app
FROM node AS console-builder
WORKDIR /console

COPY console/ ./
RUN npm ci
RUN npm run build

# Stage 2: Build the Rust binary
FROM rust:1.88 AS server-builder

WORKDIR /server
COPY server/ ./

RUN cargo build --release

# Stage 3: Minimal final image
FROM gcr.io/distroless/cc-debian12

# Copy the compiled binary from the builder stages
COPY --from=console-builder /console/dist /console/dist
COPY --from=server-builder /server/target/release/strava_downloader /server
COPY --from=server-builder /server/conf/application.yaml /conf/application.yaml

CMD ["/server"]
