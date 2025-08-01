# Stage 1: Build the React app
FROM node AS web-builder
WORKDIR /web

COPY web/ ./
RUN npm install
RUN npm run build

# Stage 2: Build the Rust binary
FROM rust:1.85 AS rust-builder

WORKDIR /app
COPY . .

RUN cargo build --release

# Stage 3: Minimal final image
FROM gcr.io/distroless/cc

# Copy the compiled binary from the builder stages
COPY --from=web-builder /web/dist /web/dist
COPY --from=rust-builder /app/target/release/strava_downloader /strava_downloader
COPY --from=rust-builder /app/conf/application.yaml /conf/application.yaml

CMD ["/strava_downloader"]
