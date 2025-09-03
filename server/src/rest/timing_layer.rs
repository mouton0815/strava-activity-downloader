use std::future::Future;
use std::pin::Pin;
use axum::http::{Request, Response};
use std::task::{Context, Poll, ready};
use std::time::Instant;
use humantime::format_duration;
use log::info;
use pin_project_lite::pin_project;
use tower::{Layer, Service};

#[derive(Clone)]
pub struct TimingLayer;

impl<S> Layer<S> for TimingLayer {
    type Service = TimingMiddleware<S>;
    fn layer(&self, service: S) -> Self::Service {
        TimingMiddleware { service }
    }
}

#[derive(Clone, Debug)]
pub struct TimingMiddleware<S> {
    service: S
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for TimingMiddleware<S>
    where
        S: Service<Request<ReqBody>, Response = Response<ResBody>>,
        ResBody: Default
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = TimingFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let path = req.uri().to_string();
        let timer = Instant::now();
        let future = self.service.call(req);
        TimingFuture { future, timer, path }
    }
}

pin_project! {
    pub struct TimingFuture<F> {
        #[pin]
        future: F,
        timer: Instant,
        path: String
    }
}

impl<F, B, E> Future for TimingFuture<F>
    where
        F: Future<Output = Result<Response<B>, E>>,
{
    type Output = Result<Response<B>, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let response: Response<B> = ready!(this.future.poll(cx))?;
        info!("{} took {}", this.path, format_duration(this.timer.elapsed()));
        Poll::Ready(Ok(response))
    }
}