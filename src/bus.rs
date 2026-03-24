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

/// Summary returned by every [`Bus::publish`] call.
///
/// Reports how many subscribers received the event and how many were skipped
/// because their channel was full.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PublishResult {
    delivered: usize,
    dropped: usize,
}

impl PublishResult {
    /// Returns the number of subscribers that successfully received the event.
    pub fn delivered(&self) -> usize {
        self.delivered
    }

    /// Returns the number of subscribers whose channel was full; the event was
    /// silently discarded for those subscribers.
    pub fn dropped(&self) -> usize {
        self.dropped
    }
}

#[derive(Debug)]
struct BusInner<O, K, T> {
    subscribers: RwLock<SubscriberMap<O, K, T>>,
    next_id: AtomicU64,
    capacity: usize,
}

/// A cloneable, in-process publish/subscribe message bus.
///
/// `Bus<O, K, T>` routes [`Event`]s to [`BusReceiver`]s based on [`Filter`]
/// pairs. Every clone of a `Bus` shares the same subscription registry, so
/// handles can freely be moved across threads or into tasks.
///
/// # Type parameters
///
/// | Parameter | Constraint | Meaning |
/// |-----------|------------|---------|
/// | `O` | `Clone + Eq + Hash + Send + Sync` | Origin — where the event came from |
/// | `K` | `Clone + Eq + Hash + Send + Sync` | Kind — what type of event it is |
/// | `T` | `Clone + Send + Sync` | Payload — the data carried by the event |
///
/// # Examples
///
/// ```rust
/// use omnibus::{Bus, Event, Filter};
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// let bus = Bus::<String, String, String>::new();
/// let sub = bus.subscribe(Filter::Any, Filter::Any)?;
/// bus.publish(Event::new("src".to_string(), "ping".to_string(), "hello".to_string()))?;
/// assert!(sub.recv()?.is_some());
/// # Ok(())
/// # }
/// ```
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
    /// Creates a new `Bus` with a default per-subscriber channel capacity of 64.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use omnibus::Bus;
    ///
    /// let bus = Bus::<String, String, String>::new();
    /// ```
    pub fn new() -> Self {
        Self::with_capacity(64)
    }

    /// Creates a new `Bus` whose per-subscriber channels each hold at most
    /// `capacity` events before silently dropping new ones for that subscriber.
    ///
    /// A capacity of `0` creates a rendezvous channel: every non-blocking
    /// publish will be dropped unless a thread is already blocking on
    /// [`BusReceiver::recv_blocking`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use omnibus::Bus;
    ///
    /// let bus = Bus::<String, String, String>::with_capacity(128);
    /// ```
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Arc::new(BusInner {
                subscribers: RwLock::new(HashMap::new()),
                next_id: AtomicU64::new(0),
                capacity,
            }),
        }
    }

    /// Subscribes to events matching the given origin and kind filters.
    ///
    /// Returns a [`BusReceiver`] that will receive all future matching events.
    /// Dropping the receiver automatically unregisters it.
    ///
    /// # Errors
    ///
    /// Returns [`crate::OmnibusError::Poisoned`] if an internal lock was poisoned by
    /// a panic in another thread.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use omnibus::{Bus, Filter};
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let bus = Bus::<String, String, String>::new();
    ///
    /// // Subscribe to all events:
    /// let _all = bus.subscribe(Filter::Any, Filter::Any)?;
    ///
    /// // Subscribe to events from "sensor" only:
    /// let _sensor = bus.subscribe(Filter::Is("sensor".to_string()), Filter::Any)?;
    /// # Ok(())
    /// # }
    /// ```
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

    /// Publishes `event` to all matching subscribers.
    ///
    /// An event matches a subscriber when both the origin and kind filters
    /// match. Subscribers across all four filter-bucket combinations
    /// (`Is`/`Is`, `Is`/`Any`, `Any`/`Is`, `Any`/`Any`) are checked.
    ///
    /// Delivery is non-blocking: if a subscriber's channel is full the event
    /// is silently dropped for that subscriber and counted in
    /// [`PublishResult::dropped`].
    ///
    /// # Errors
    ///
    /// Returns [`crate::OmnibusError::Poisoned`] if an internal lock was poisoned by
    /// a panic in another thread.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use omnibus::{Bus, Event, Filter};
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let bus = Bus::<String, String, String>::new();
    /// let sub = bus.subscribe(Filter::Any, Filter::Any)?;
    ///
    /// let result = bus.publish(Event::new(
    ///     "sensor".to_string(),
    ///     "temperature".to_string(),
    ///     "42.5\u{b0}C".to_string(),
    /// ))?;
    ///
    /// assert_eq!(result.delivered(), 1);
    /// assert_eq!(result.dropped(), 0);
    /// # Ok(())
    /// # }
    /// ```
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

/// A subscription handle returned by [`Bus::subscribe`].
///
/// Provides multiple ways to read events from the channel. Dropping a
/// `BusReceiver` automatically unregisters it from the [`Bus`].
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
    /// Creates a new independent subscription on the same [`Bus`] with the
    /// same origin and kind filters.
    ///
    /// The new receiver starts empty; it will receive events published after
    /// this call, not events already queued on `self`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::OmnibusError::Poisoned`] if an internal lock was poisoned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use omnibus::{Bus, Event, Filter};
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let bus = Bus::<String, String, String>::new();
    /// let sub1 = bus.subscribe(Filter::Any, Filter::Any)?;;
    /// let sub2 = sub1.resubscribe()?;
    ///
    /// bus.publish(Event::new("src".to_string(), "ping".to_string(), "hello".to_string()))?;
    ///
    /// assert!(sub1.recv()?.is_some());
    /// assert!(sub2.recv()?.is_some());
    /// # Ok(())
    /// # }
    /// ```
    pub fn resubscribe(&self) -> crate::Result<BusReceiver<O, K, T>> {
        self.bus.subscribe(self.origin.clone(), self.kind.clone())
    }

    /// Non-blocking receive.
    ///
    /// Returns `Ok(None)` if no event is currently queued.
    ///
    /// # Errors
    ///
    /// Returns [`crate::OmnibusError::Disconnected`] if the [`Bus`] has been dropped
    /// and no more events can ever arrive.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use omnibus::{Bus, Event, Filter};
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let bus = Bus::<String, String, String>::new();
    /// let sub = bus.subscribe(Filter::Any, Filter::Any)?;
    ///
    /// assert!(sub.recv()?.is_none()); // nothing queued yet
    ///
    /// bus.publish(Event::new("src".to_string(), "ping".to_string(), "hello".to_string()))?;
    /// assert!(sub.recv()?.is_some());
    /// # Ok(())
    /// # }
    /// ```
    pub fn recv(&self) -> crate::Result<Option<Arc<Event<O, K, T>>>> {
        match self.rx.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(std::sync::mpsc::TryRecvError::Empty) => Ok(None),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                Err(crate::OmnibusError::Disconnected)
            }
        }
    }

    /// Blocking receive.
    ///
    /// Parks the current thread until an event arrives or the [`Bus`] is
    /// dropped.
    ///
    /// # Errors
    ///
    /// Returns [`crate::OmnibusError::Disconnected`] if the [`Bus`] was dropped
    /// before an event arrived.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use omnibus::{Bus, Event, Filter};
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let bus = Bus::<String, String, String>::new();
    /// let sub = bus.subscribe(Filter::Any, Filter::Any)?;
    /// bus.publish(Event::new("src".to_string(), "ping".to_string(), "hi".to_string()))?;
    /// let ev = sub.recv_blocking()?;
    /// assert_eq!(*ev.payload(), "hi".to_string());
    /// # Ok(())
    /// # }
    /// ```
    pub fn recv_blocking(&self) -> crate::Result<Arc<Event<O, K, T>>> {
        self.rx
            .recv()
            .map_err(|_| crate::OmnibusError::Disconnected)
    }

    /// Blocking receive with a timeout.
    ///
    /// Parks the current thread until an event arrives or `timeout` elapses.
    /// Returns `Ok(None)` on timeout.
    ///
    /// # Errors
    ///
    /// Returns [`crate::OmnibusError::Disconnected`] if the [`Bus`] was dropped
    /// before an event arrived or the timeout elapsed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use omnibus::{Bus, Filter};
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let bus = Bus::<String, String, String>::new();
    /// let sub = bus.subscribe(Filter::Any, Filter::Any)?;
    ///
    /// // No event arrives within 1 ms:
    /// assert!(sub.recv_timeout(Duration::from_millis(1))?.is_none());
    /// # Ok(())
    /// # }
    /// ```
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
    ///
    /// The iterator yields events until the channel is empty; it does not
    /// block waiting for new events.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use omnibus::{Bus, Event, Filter};
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let bus = Bus::<String, String, String>::new();
    /// let sub = bus.subscribe(Filter::Any, Filter::Any)?;
    ///
    /// for i in 0..3u32 {
    ///     bus.publish(Event::new("src".to_string(), "ping".to_string(), i.to_string()))?;
    /// }
    ///
    /// let events: Vec<_> = sub.drain().collect();
    /// assert_eq!(events.len(), 3);
    /// # Ok(())
    /// # }
    /// ```
    pub fn drain(&self) -> impl Iterator<Item = Arc<Event<O, K, T>>> + '_ {
        std::iter::from_fn(move || self.rx.try_recv().ok())
    }
}
