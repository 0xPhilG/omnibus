# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-24

### Added

- `Bus<O, K, T>` — in-process publish/subscribe message bus, cloneable across threads.
- `BusReceiver<O, K, T>` — subscription handle with non-blocking (`recv`), blocking
  (`recv_blocking`), timeout (`recv_timeout`), drain, and resubscribe receive styles.
- `Filter<T>` — subscription filter supporting exact-match (`Is`) and wildcard (`Any`).
- `Event<O, K, T>` — typed event value carrying an origin, kind, and payload.
- `PublishResult` — delivery summary returned by every `Bus::publish` call.
- `OmnibusError` — error type for channel-disconnected and lock-poisoned conditions.
