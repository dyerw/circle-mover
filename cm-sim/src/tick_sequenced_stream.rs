use std::{
    collections::HashMap,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

use futures::{FutureExt, Stream, StreamExt};
use queues::{IsQueue, Queue};
use tokio::time::sleep;

pub trait GetTick {
    fn get_tick(self) -> i32;
}

pub enum TickSequencedEvent<T: GetTick> {
    Tick { dt: Duration, number: i32 },
    Event(T),
}

pub struct TickSequencedStream<T: GetTick + Clone, S: Stream<Item = T>> {
    inner: S,
    /// If this duration has passed the next stream poll will return Tick
    minimum_tick_duration: Duration,
    last_tick_instant: Instant,
    current_tick: i32,
    // Using HashMap as a sparse array, might create determinism issues?
    event_buffer: HashMap<i32, Queue<T>>,
}

impl<T: GetTick + Clone, S: Stream<Item = T>> TickSequencedStream<T, S> {
    pub fn new(stream: S, minimum_tick_duration: Duration) -> Self {
        return Self {
            inner: stream,
            minimum_tick_duration,
            last_tick_instant: Instant::now(),
            current_tick: 0,
            event_buffer: HashMap::new(),
        };
    }
}

impl<T: GetTick + Clone, S: Stream<Item = T> + Unpin> Unpin for TickSequencedStream<T, S> {}

impl<T: GetTick + Clone, S: Stream<Item = T> + Unpin> Stream for TickSequencedStream<T, S> {
    type Item = TickSequencedEvent<T>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // The latest we want to be woken is on the next tick
        let now = Instant::now();
        let duration_since_last_tick = now.duration_since(self.last_tick_instant);
        // Share the context with the waker with a sleep? Maybe???
        let _ =
            Box::pin(sleep(self.minimum_tick_duration - duration_since_last_tick)).poll_unpin(cx);

        // Ingest all events from the inner stream into the buffer
        while let Poll::Ready(inner_next) = self.inner.poll_next_unpin(cx) {
            match inner_next {
                Some(inner_event) => {
                    let event_clone = inner_event.clone();
                    let for_tick = inner_event.get_tick();

                    if let Some(tick_buffer) = self.event_buffer.get_mut(&for_tick) {
                        tick_buffer.add(event_clone).unwrap();
                    } else {
                        let mut queue = Queue::new();
                        queue.add(event_clone).unwrap();
                        self.event_buffer.insert(for_tick, queue);
                    }
                }
                None => {
                    // Inner stream finished
                    // TODO: deal with this, finish the outer stream after processing the buffer
                    return Poll::Ready(None);
                }
            }
        }

        // Forward all inputs for the current tick
        let current_tick = self.current_tick;
        if let Some(current_tick_buffer) = self.event_buffer.get_mut(&current_tick) {
            if let Ok(next_buffered_event) = current_tick_buffer.remove() {
                return Poll::Ready(Some(TickSequencedEvent::Event(next_buffered_event)));
            }
        }

        // // If there are no events for the current tick check if we need to tick forward
        if duration_since_last_tick >= self.minimum_tick_duration {
            self.current_tick += 1;
            self.last_tick_instant = now;
            return Poll::Ready(Some(TickSequencedEvent::Tick {
                dt: duration_since_last_tick,
                number: self.current_tick,
            }));
        }

        return Poll::Pending;
    }
}
