/// A single message carrying an origin, a kind, and a payload.
///
/// Create events with [`Event::new`] and publish them through a [`crate::Bus`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Event<O, K, T> {
    origin: O,
    kind: K,
    payload: T,
}

impl<O: Clone, K: Clone, T: Clone> Event<O, K, T> {
    /// Creates a new event with the given origin, kind, and payload.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use omnibus::Event;
    ///
    /// let ev = Event::new("sensor", "temperature", "42.5\u{b0}C");
    /// assert_eq!(ev.origin(), &"sensor");
    /// assert_eq!(ev.kind(), &"temperature");
    /// assert_eq!(ev.payload(), &"42.5\u{b0}C");
    /// ```
    pub fn new(origin: O, kind: K, payload: T) -> Self {
        Self {
            origin,
            kind,
            payload,
        }
    }

    /// Returns a reference to the event's origin.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use omnibus::Event;
    ///
    /// let ev = Event::new("sensor", "temperature", 42);
    /// assert_eq!(ev.origin(), &"sensor");
    /// ```
    pub fn origin(&self) -> &O {
        &self.origin
    }

    /// Returns a reference to the event's kind.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use omnibus::Event;
    ///
    /// let ev = Event::new("sensor", "temperature", 42);
    /// assert_eq!(ev.kind(), &"temperature");
    /// ```
    pub fn kind(&self) -> &K {
        &self.kind
    }

    /// Returns a reference to the event's payload.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use omnibus::Event;
    ///
    /// let ev = Event::new("sensor", "temperature", 42);
    /// assert_eq!(ev.payload(), &42);
    /// ```
    pub fn payload(&self) -> &T {
        &self.payload
    }
}
