# omnibus

A lightweight, in-process publish/subscribe event bus for Rust.

`omnibus` lets multiple parts of an application exchange typed events without direct dependencies between them. The bus is cheaply cloneable — any clone shares the same subscription registry — making it easy to pass a handle across threads or into async tasks.

## Features

- **Fully generic** — parameterise the bus over any `origin`, `kind`, and `payload` types.
- **Flexible subscriptions** — filter by exact origin, exact kind, both, or neither (`Filter::Any`).
- **Arc-shared payloads** — each event is wrapped in an `Arc` once; subscribers receive a cheap pointer clone, not a deep copy.
- **Non-blocking, blocking, and timeout receive** — choose the receive style that fits your context.
- **Drain iterator** — consume all currently queued events in a single loop.
- **Resubscribe** — create an independent subscription with the same filters as an existing one.
- **Back-pressure via bounded channels** — each subscriber has a fixed-capacity channel; `PublishResult` reports how many deliveries succeeded and how many were dropped.
- **Automatic cleanup** — dropping a `BusReceiver` unregisters it from the bus.

## Installation

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
omnibus = "0.1.0"
```

## Quick start

```rust
use omnibus::{Bus, Event, Filter};

let bus = Bus::<String, String, String>::new();

let sub = bus
    .subscribe(
        Filter::Is("sensor".into()),
        Filter::Is("temperature".into()),
    )
    .unwrap();

bus.publish(Event::new(
    "sensor".into(),
    "temperature".into(),
    "42.5°C".into(),
))
.unwrap();

if let Some(event) = sub.recv().unwrap() {
    println!("[{} / {}] {}", event.origin(), event.kind(), event.payload());
}
```

## Concepts

### `Bus<O, K, T>`

The central hub. Three type parameters describe every event that flows through it:

| Parameter | Meaning |
|-----------|---------|
| `O`       | **Origin** — where the event came from (e.g. a component name or enum variant) |
| `K`       | **Kind** — what type of event it is |
| `T`       | **Payload** — the data carried by the event |

`O` and `K` must implement `Clone + Eq + Hash + Send + Sync`; `T` additionally needs `Clone + Send + Sync`.

```rust
// String-based bus
let bus = Bus::<String, String, String>::new();

// Enum-based bus (recommended for larger projects)
let bus: Bus<Origin, Kind, String> = Bus::new();
```

`Bus::with_capacity(n)` sets the per-subscriber channel size (default: 64). Events published to a full channel are silently dropped and counted in `PublishResult::dropped`.

### `Filter<T>`

Controls which events a subscription receives:

| Variant | Receives |
|---------|---------|
| `Filter::Any` | Events with any value for this dimension |
| `Filter::Is(v)` | Only events whose origin/kind equals `v` |

### `Event<O, K, T>`

Created with `Event::new(origin, kind, payload)` and accessed via `.origin()`, `.kind()`, and `.payload()`.

### `BusReceiver<O, K, T>`

Returned by `Bus::subscribe`. Provides several ways to read events:

| Method | Behaviour |
|--------|-----------|
| `recv()` | Non-blocking; returns `Ok(None)` if the queue is empty |
| `recv_blocking()` | Blocks until an event arrives or the bus is dropped |
| `recv_timeout(duration)` | Blocks up to `duration`; returns `Ok(None)` on timeout |
| `drain()` | Iterator that yields all currently queued events |
| `resubscribe()` | New independent receiver with the same filters |

Dropping a `BusReceiver` automatically removes it from the bus.

## Example — typed enums with multiple filters

```rust
use omnibus::{Bus, Event, Filter};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Origin { Sensor, Other }

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Kind { Temperature, Humidity }

let bus: Bus<Origin, Kind, String> = Bus::new();

// Wildcard — every event
let all = bus.subscribe(Filter::Any, Filter::Any).unwrap();

// Only events from Origin::Sensor, any kind
let sensor = bus.subscribe(Filter::Is(Origin::Sensor), Filter::Any).unwrap();

// Only Temperature events, any origin
let temp = bus.subscribe(Filter::Any, Filter::Is(Kind::Temperature)).unwrap();

// Exact match
let exact = bus
    .subscribe(Filter::Is(Origin::Sensor), Filter::Is(Kind::Temperature))
    .unwrap();

let result = bus
    .publish(Event::new(Origin::Sensor, Kind::Temperature, "42.5°C".into()))
    .unwrap();

println!("delivered={}, dropped={}", result.delivered, result.dropped);
// delivered=4, dropped=0

for ev in all.drain() {
    println!("[{:?}/{:?}] {}", ev.origin(), ev.kind(), ev.payload());
}
```

## Error handling

All fallible methods return `omnibus::Result<T>`, which is an alias for `Result<T, OmnibusError>`.

| Error | Cause |
|-------|-------|
| `OmnibusError::Disconnected` | All senders were dropped (bus went away) |
| `OmnibusError::Poisoned` | An internal lock was poisoned by a panic in another thread |

## Running the examples

```sh
cargo run --example simple
cargo run --example basic_pubsub
```

## License

Licensed under the terms in [LICENSE](LICENSE).
