//! Caller-admitted, caller-polled native fanout. First failure is observable
//! without dropping other native calls; the same owner subsequently drains them.
use crate::HostError;
use futures_util::{FutureExt, StreamExt, future::BoxFuture, stream::FuturesUnordered};
use std::{
    collections::VecDeque,
    future::poll_fn,
    task::{Context, Poll},
};

pub(super) const FUTURE_BYTES: usize = 4096;
pub(super) const NODE_BYTES: usize = 1024;

pub(super) struct Fanout<'a> {
    unstarted: Vec<BoxFuture<'a, Result<(), HostError>>>,
    pending: FuturesUnordered<BoxFuture<'a, Result<(), HostError>>>,
    immediate_error: Option<HostError>,
    started: bool,
    group_sizes: VecDeque<usize>,
    grouped: usize,
}

impl<'a> Fanout<'a> {
    pub(super) fn new() -> Self {
        Self {
            unstarted: Vec::with_capacity(super::MAX_ROUTES),
            pending: FuturesUnordered::new(),
            immediate_error: None,
            started: false,
            group_sizes: VecDeque::with_capacity(super::MAX_ROUTES),
            grouped: 0,
        }
    }
    pub(super) fn push(
        &mut self,
        future: impl Future<Output = Result<(), HostError>> + Send + 'a,
    ) -> Result<(), HostError> {
        if self.started {
            return Err(HostError::Backend(
                "header fanout is already started".into(),
            ));
        }
        if self.unstarted.len() == super::MAX_ROUTES
            || std::mem::size_of_val(&future) > FUTURE_BYTES
        {
            return Err(HostError::ResourceExhausted(
                "header native fanout storage exhausted".into(),
            ));
        }
        self.unstarted.push(future.boxed());
        Ok(())
    }
    /// Close one sequential group. Members within a group start together;
    /// later groups start only after every member of this one succeeds.
    pub(super) fn end_group(&mut self) -> Result<(), HostError> {
        if self.started || self.grouped == self.unstarted.len() {
            return Err(HostError::Backend("invalid header fanout group".into()));
        }
        self.group_sizes
            .push_back(self.unstarted.len() - self.grouped);
        self.grouped = self.unstarted.len();
        Ok(())
    }
    fn prime(&mut self, cx: &mut Context<'_>) {
        if self.started {
            return;
        }
        self.started = true;
        // All members get their first poll before exposing an immediate error,
        // matching eager Promise.all dispatch rather than serial short-circuit.
        let count = self.group_sizes.pop_front().unwrap_or(self.unstarted.len());
        for mut future in self.unstarted.drain(..count) {
            match future.as_mut().poll(cx) {
                Poll::Pending => self.pending.push(future),
                Poll::Ready(Ok(())) => {}
                Poll::Ready(Err(error)) => {
                    if self.immediate_error.is_none() {
                        self.immediate_error = Some(error);
                    }
                }
            }
        }
    }
    pub(super) async fn start(&mut self) {
        poll_fn(|cx| {
            self.prime(cx);
            Poll::Ready(())
        })
        .await;
    }
    pub(super) async fn first_result(&mut self) -> Result<(), HostError> {
        loop {
            self.start().await;
            if let Some(error) = self.immediate_error.take() {
                return Err(error);
            }
            while let Some(result) = self.pending.next().await {
                result?;
            }
            if self.unstarted.is_empty() {
                return Ok(());
            }
            self.started = false;
        }
    }
    pub(super) async fn drain(&mut self) {
        self.start().await;
        // Secondary failures cannot replace the already selected primary. Their
        // futures still finish normally and release their exact native owners.
        while self.pending.next().await.is_some() {}
    }
    pub(super) async fn settle(&mut self) -> Result<(), HostError> {
        let result = self.first_result().await;
        self.drain().await;
        result
    }
}

#[cfg(test)]
mod tests;
