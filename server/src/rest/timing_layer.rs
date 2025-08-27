use axum::http::Request;
use std::task::{Context, Poll};
use std::time::Instant;
use log::info;
use tower::{Layer, Service};

#[derive(Clone)]
pub struct TimingLayer;

impl<S> Layer<S> for TimingLayer {
    type Service = TimingMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        TimingMiddleware { inner }
    }
}

#[derive(Clone, Debug)]
pub struct TimingMiddleware<S> {
    inner: S
}

impl<S, ReqBody> Service<Request<ReqBody>> for TimingMiddleware<S>
    where
        S: Service<Request<ReqBody>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let path = req.uri().path().to_string();
        let start = Instant::now();
        let result = self.inner.call(req);
        let elapsed = start.elapsed();
        info!("Request {path} took {}", humantime::format_duration(elapsed));
        result
    }
}