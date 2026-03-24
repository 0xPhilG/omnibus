#[derive(Debug, Clone)]
pub struct Event<O, K, T> {
    origin: O,
    kind: K,
    payload: T,
}

impl<O: Clone, K: Clone, T: Clone> Event<O, K, T> {
    pub fn new(origin: O, kind: K, payload: T) -> Self {
        Self {
            origin,
            kind,
            payload,
        }
    }

    pub fn origin(&self) -> &O {
        &self.origin
    }

    pub fn kind(&self) -> &K {
        &self.kind
    }

    pub fn payload(&self) -> &T {
        &self.payload
    }
}
