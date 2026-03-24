use std::{
    collections::HashMap,
    hash::Hash,
    sync::{
        Arc, RwLock,
        atomic::{AtomicU64, Ordering},
        mpsc::{Receiver, SyncSender, TrySendError},
    },
    time::Duration,
};

use crate::event::Event;
use crate::filter::Filter;

type SubId = u64;
type SubscriberMap<O, K, T> =
    HashMap<(Filter<O>, Filter<K>), Vec<(SubId, SyncSender<Arc<Event<O, K, T>>>)>>;

/// Summary of a single `publish` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublishResult {
    /// Number of subscribers that received the event.
    pub delivered: usize,
    /// Number of subscribers whose channel was full (event silently dropped).
    pub dropped: usize,
}

#[derive(Debug)]
struct BusInner<O, K, T> {
    subscribers: RwLock<SubscriberMap<O, K, T>>,
    next_id: AtomicU64,
    capacity: usize,
}

#[derive(Debug)]
pub struct Bus<O, K, T> {
    inner: Arc<BusInner<O, K, T>>,
}

impl<O, K, T> Clone for Bus<O, K, T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<O, K, T> Bus<O, K, T>
where
    O: Clone + Eq + Hash + Send + Sync,
    K: Clone + Eq + Hash + Send + Sync,
    T: Clone + Send + Sync,
{
    /// Creates a bus with a default per-subscriber channel capacity of 64.
    pub fn new() -> Self {
        Self::with_capacity(64)
    }

    /// Creates a bus whose per-subscriber channels each hold `capacity` events
    /// before silently dropping new ones for that subscriber.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Arc::new(BusInner {
                subscribers: RwLock::new(HashMap::new()),
                next_id: AtomicU64::new(0),
                capacity,
            }),
        }
    }

    pub fn subscribe(
        &self,
        origin: Filter<O>,
        kind: Filter<K>,
    ) -> crate::Result<BusReceiver<O, K, T>> {
        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = std::sync::mpsc::sync_channel(self.inner.capacity);
        self.inner
            .subscribers
            .write()
            .map_err(|_| crate::OmnibusError::Poisoned)?
            .entry((origin.clone(), kind.clone()))
            .or_default()
            .push((id, tx));
        Ok(BusReceiver {
            bus: self.clone(),
            origin,
            kind,
            id,
            rx,
        })
    }

    pub fn publish(&self, event: Event<O, K, T>) -> crate::Result<PublishResult> {
        // Wrap once; each subscriber receives a cheap pointer clone.
        let event = Arc::new(event);
        let keys = [
            (
                Filter::Is(event.origin().clone()),
                Filter::Is(event.kind().clone()),
            ),
            (Filter::Is(event.origin().clone()), Filter::Any),
            (Filter::Any, Filter::Is(event.kind().clone())),
            (Filter::Any, Filter::Any),
        ];

        // Snapshot matching senders under a read lock, then release before fanning out
        // so concurrent publishes are not serialised by the fan-out itself.
        let senders: Vec<SyncSender<Arc<Event<O, K, T>>>> = {
            let subscribers = self
                .inner
                .subscribers
                .read()
                .map_err(|_| crate::OmnibusError::Poisoned)?;
            keys.iter()
                .filter_map(|key| subscribers.get(key))
                .flat_map(|v| v.iter().map(|(_, tx)| tx.clone()))
                .collect()
        };

        let mut delivered = 0;
        let mut dropped = 0;
        for tx in senders {
            match tx.try_send(Arc::clone(&event)) {
                Ok(()) => delivered += 1,
                Err(TrySendError::Full(_)) => dropped += 1,
                Err(TrySendError::Disconnected(_)) => {}
            }
        }
        Ok(PublishResult { delivered, dropped })
    }
}

impl<O, K, T> Default for Bus<O, K, T>
where
    O: Clone + Eq + Hash + Send + Sync,
    K: Clone + Eq + Hash + Send + Sync,
    T: Clone + Send + Sync,
{
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct BusReceiver<O, K, T>
where
    O: Clone + Eq + Hash + Send + Sync,
    K: Clone + Eq + Hash + Send + Sync,
    T: Clone + Send + Sync,
{
    bus: Bus<O, K, T>,
    origin: Filter<O>,
    kind: Filter<K>,
    id: SubId,
    rx: Receiver<Arc<Event<O, K, T>>>,
}

impl<O, K, T> Drop for BusReceiver<O, K, T>
where
    O: Clone + Eq + Hash + Send + Sync,
    K: Clone + Eq + Hash + Send + Sync,
    T: Clone + Send + Sync,
{
    fn drop(&mut self) {
        let key = (self.origin.clone(), self.kind.clone());
        // Recover from a poisoned lock in drop — we cannot propagate errors here.
        let mut subscribers = self
            .bus
            .inner
            .subscribers
            .write()
            .unwrap_or_else(|e| e.into_inner());
        let is_empty = if let Some(vec) = subscribers.get_mut(&key) {
            vec.retain(|(id, _)| *id != self.id);
            vec.is_empty()
        } else {
            false
        };
        if is_empty {
            subscribers.remove(&key);
        }
    }
}

impl<O, K, T> BusReceiver<O, K, T>
where
    O: Clone + Eq + Hash + Send + Sync,
    K: Clone + Eq + Hash + Send + Sync,
    T: Clone + Send + Sync,
{
    /// Creates a new independent subscription on the same bus with the same filters.
    pub fn resubscribe(&self) -> crate::Result<BusReceiver<O, K, T>> {
        self.bus.subscribe(self.origin.clone(), self.kind.clone())
    }

    /// Non-blocking receive. Returns `Ok(None)` if no event is queued.
    pub fn recv(&self) -> crate::Result<Option<Arc<Event<O, K, T>>>> {
        match self.rx.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(std::sync::mpsc::TryRecvError::Empty) => Ok(None),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                Err(crate::OmnibusError::Disconnected)
            }
        }
    }

    /// Blocking receive. Parks the thread until an event arrives or the bus is dropped.
    pub fn recv_blocking(&self) -> crate::Result<Arc<Event<O, K, T>>> {
        self.rx
            .recv()
            .map_err(|_| crate::OmnibusError::Disconnected)
    }

    /// Blocking receive with timeout. Returns `Ok(None)` if the timeout elapses.
    pub fn recv_timeout(&self, timeout: Duration) -> crate::Result<Option<Arc<Event<O, K, T>>>> {
        match self.rx.recv_timeout(timeout) {
            Ok(event) => Ok(Some(event)),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Ok(None),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                Err(crate::OmnibusError::Disconnected)
            }
        }
    }

    /// Returns a non-blocking iterator that drains all currently queued events.
    pub fn drain(&self) -> impl Iterator<Item = Arc<Event<O, K, T>>> + '_ {
        std::iter::from_fn(move || self.rx.try_recv().ok())
    }
}
