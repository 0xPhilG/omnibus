use std::sync::Arc;
use std::time::Duration;

use omnibus::{Bus, Event, Filter, OmnibusError};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Origin {
    Sensor,
    Other,
    A,
    B,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Kind {
    Temperature,
    Humidity,
    X,
    Y,
    General,
}

#[test]
fn test_publish_receive() {
    let bus: Bus<Origin, Kind, i32> = Bus::new();
    let sub = bus
        .subscribe(Filter::Is(Origin::Sensor), Filter::Is(Kind::Temperature))
        .unwrap();
    bus.publish(Event::new(Origin::Sensor, Kind::Temperature, 42))
        .unwrap();
    let ev = sub.recv().unwrap().unwrap();
    assert_eq!(ev.payload(), &42);
    assert_eq!(ev.origin(), &Origin::Sensor);
    assert_eq!(ev.kind(), &Kind::Temperature);
}

#[test]
fn test_wildcard_subscription() {
    let bus: Bus<Origin, Kind, &str> = Bus::new();
    let sub = bus.subscribe(Filter::Any, Filter::Any).unwrap();
    bus.publish(Event::new(Origin::A, Kind::X, "first"))
        .unwrap();
    bus.publish(Event::new(Origin::B, Kind::Y, "second"))
        .unwrap();
    assert_eq!(sub.recv().unwrap().unwrap().payload(), &"first");
    assert_eq!(sub.recv().unwrap().unwrap().payload(), &"second");
    assert!(sub.recv().unwrap().is_none());
}

#[test]
fn test_origin_only_subscription() {
    let bus: Bus<Origin, Kind, i32> = Bus::new();
    let sub = bus
        .subscribe(Filter::Is(Origin::Sensor), Filter::Any)
        .unwrap();
    bus.publish(Event::new(Origin::Sensor, Kind::Temperature, 1))
        .unwrap();
    bus.publish(Event::new(Origin::Sensor, Kind::Humidity, 2))
        .unwrap();
    bus.publish(Event::new(Origin::Other, Kind::Temperature, 99))
        .unwrap();
    assert_eq!(sub.recv().unwrap().unwrap().payload(), &1);
    assert_eq!(sub.recv().unwrap().unwrap().payload(), &2);
    assert!(sub.recv().unwrap().is_none());
}

#[test]
fn test_type_only_subscription() {
    let bus: Bus<Origin, Kind, i32> = Bus::new();
    let sub = bus
        .subscribe(Filter::Any, Filter::Is(Kind::Temperature))
        .unwrap();
    bus.publish(Event::new(Origin::Sensor, Kind::Temperature, 1))
        .unwrap();
    bus.publish(Event::new(Origin::Other, Kind::Temperature, 2))
        .unwrap();
    bus.publish(Event::new(Origin::Sensor, Kind::Humidity, 99))
        .unwrap();
    assert_eq!(sub.recv().unwrap().unwrap().payload(), &1);
    assert_eq!(sub.recv().unwrap().unwrap().payload(), &2);
    assert!(sub.recv().unwrap().is_none());
}

#[test]
fn test_no_match() {
    let bus: Bus<Origin, Kind, i32> = Bus::new();
    let sub = bus
        .subscribe(Filter::Is(Origin::Sensor), Filter::Is(Kind::Temperature))
        .unwrap();
    bus.publish(Event::new(Origin::Other, Kind::Humidity, 1))
        .unwrap();
    assert!(sub.recv().unwrap().is_none());
}

#[test]
fn test_recv_empty() {
    let bus: Bus<Origin, Kind, i32> = Bus::new();
    let sub = bus.subscribe(Filter::Any, Filter::Any).unwrap();
    assert!(sub.recv().unwrap().is_none());
}

#[test]
fn test_resubscribe_receives_independently() {
    let bus: Bus<Origin, Kind, i32> = Bus::new();
    let sub1 = bus.subscribe(Filter::Any, Filter::Any).unwrap();
    let sub2 = sub1.resubscribe().unwrap();
    bus.publish(Event::new(Origin::Sensor, Kind::General, 1))
        .unwrap();
    assert_eq!(sub1.recv().unwrap().unwrap().payload(), &1);
    assert_eq!(sub2.recv().unwrap().unwrap().payload(), &1);
    assert!(sub1.recv().unwrap().is_none());
    assert!(sub2.recv().unwrap().is_none());
}

#[test]
fn test_buffer_full_does_not_panic() {
    let bus: Bus<Origin, Kind, i32> = Bus::with_capacity(10);
    let sub = bus.subscribe(Filter::Any, Filter::Any).unwrap();
    for i in 0..20 {
        bus.publish(Event::new(Origin::Sensor, Kind::General, i))
            .unwrap();
    }
    let mut count = 0;
    while sub.recv().unwrap().is_some() {
        count += 1;
    }
    assert!(count <= 10);
}

#[test]
fn test_multiple_subscribers_same_filter() {
    let bus: Bus<Origin, Kind, i32> = Bus::new();
    let sub1 = bus.subscribe(Filter::Any, Filter::Any).unwrap();
    let sub2 = bus.subscribe(Filter::Any, Filter::Any).unwrap();
    bus.publish(Event::new(Origin::Sensor, Kind::General, 7))
        .unwrap();
    assert_eq!(sub1.recv().unwrap().unwrap().payload(), &7);
    assert_eq!(sub2.recv().unwrap().unwrap().payload(), &7);
}

// --- New tests ---

#[test]
fn test_publish_result_counts() {
    let bus: Bus<Origin, Kind, i32> = Bus::new();
    let _sub1 = bus.subscribe(Filter::Any, Filter::Any).unwrap();
    let _sub2 = bus.subscribe(Filter::Any, Filter::Any).unwrap();
    let result = bus
        .publish(Event::new(Origin::Sensor, Kind::Temperature, 1))
        .unwrap();
    assert_eq!(result.delivered, 2);
    assert_eq!(result.dropped, 0);
}

#[test]
fn test_publish_result_dropped() {
    let bus: Bus<Origin, Kind, i32> = Bus::with_capacity(1);
    let sub = bus.subscribe(Filter::Any, Filter::Any).unwrap();
    let r1 = bus
        .publish(Event::new(Origin::Sensor, Kind::Temperature, 1))
        .unwrap();
    assert_eq!(r1.delivered, 1);
    assert_eq!(r1.dropped, 0);
    // Channel is now full (capacity=1), next publish should drop
    let r2 = bus
        .publish(Event::new(Origin::Sensor, Kind::Temperature, 2))
        .unwrap();
    assert_eq!(r2.delivered, 0);
    assert_eq!(r2.dropped, 1);
    // First event is still there
    assert_eq!(sub.recv().unwrap().unwrap().payload(), &1);
}

#[test]
fn test_recv_timeout_no_event() {
    let bus: Bus<Origin, Kind, i32> = Bus::new();
    let sub = bus.subscribe(Filter::Any, Filter::Any).unwrap();
    let result = sub.recv_timeout(Duration::from_millis(50)).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_recv_timeout_receives() {
    let bus: Bus<Origin, Kind, i32> = Bus::new();
    let sub = bus.subscribe(Filter::Any, Filter::Any).unwrap();
    bus.publish(Event::new(Origin::Sensor, Kind::Temperature, 42))
        .unwrap();
    let ev = sub
        .recv_timeout(Duration::from_millis(100))
        .unwrap()
        .unwrap();
    assert_eq!(ev.payload(), &42);
}

#[test]
fn test_recv_blocking_threaded() {
    let bus: Bus<Origin, Kind, i32> = Bus::new();
    let sub = bus.subscribe(Filter::Any, Filter::Any).unwrap();
    let bus_clone = bus.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        bus_clone
            .publish(Event::new(Origin::Sensor, Kind::Temperature, 99))
            .unwrap();
    });
    let ev = sub.recv_blocking().unwrap();
    assert_eq!(ev.payload(), &99);
}

#[test]
fn test_subscriber_cleanup_after_drop() {
    let bus: Bus<Origin, Kind, i32> = Bus::new();
    let sub = bus.subscribe(Filter::Any, Filter::Any).unwrap();
    drop(sub);
    let result = bus
        .publish(Event::new(Origin::Sensor, Kind::Temperature, 1))
        .unwrap();
    assert_eq!(result.delivered, 0);
    assert_eq!(result.dropped, 0);
}

#[test]
fn test_drain_iterator() {
    let bus: Bus<Origin, Kind, i32> = Bus::new();
    let sub = bus.subscribe(Filter::Any, Filter::Any).unwrap();
    bus.publish(Event::new(Origin::Sensor, Kind::Temperature, 1))
        .unwrap();
    bus.publish(Event::new(Origin::Sensor, Kind::Humidity, 2))
        .unwrap();
    bus.publish(Event::new(Origin::Other, Kind::Temperature, 3))
        .unwrap();
    let events: Vec<Arc<Event<Origin, Kind, i32>>> = sub.drain().collect();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].payload(), &1);
    assert_eq!(events[1].payload(), &2);
    assert_eq!(events[2].payload(), &3);
    // Drained — nothing left
    assert!(sub.recv().unwrap().is_none());
}

#[test]
fn test_publish_no_subscribers() {
    let bus: Bus<Origin, Kind, i32> = Bus::new();
    let result = bus
        .publish(Event::new(Origin::Sensor, Kind::Temperature, 1))
        .unwrap();
    assert_eq!(result.delivered, 0);
    assert_eq!(result.dropped, 0);
}

// ---------- Additional coverage tests ----------

#[test]
fn test_bus_default() {
    let bus: Bus<Origin, Kind, i32> = Bus::default();
    let sub = bus.subscribe(Filter::Any, Filter::Any).unwrap();
    bus.publish(Event::new(Origin::Sensor, Kind::Temperature, 5))
        .unwrap();
    assert_eq!(sub.recv().unwrap().unwrap().payload(), &5);
}

#[test]
fn test_bus_clone_shares_state() {
    let bus1: Bus<Origin, Kind, i32> = Bus::new();
    let bus2 = bus1.clone();
    let sub = bus1.subscribe(Filter::Any, Filter::Any).unwrap();
    // Publishing through the clone reaches subscribers registered on the original
    bus2.publish(Event::new(Origin::Sensor, Kind::Temperature, 10))
        .unwrap();
    assert_eq!(sub.recv().unwrap().unwrap().payload(), &10);
}

#[test]
fn test_with_capacity_zero_drops_everything() {
    let bus: Bus<Origin, Kind, i32> = Bus::with_capacity(0);
    let sub = bus.subscribe(Filter::Any, Filter::Any).unwrap();
    let result = bus
        .publish(Event::new(Origin::Sensor, Kind::Temperature, 1))
        .unwrap();
    // sync_channel(0) is a rendezvous channel — try_send always returns Full when
    // nobody is blocking on recv, so everything is dropped.
    assert_eq!(result.delivered, 0);
    assert_eq!(result.dropped, 1);
    assert!(sub.recv().unwrap().is_none());
}

#[test]
fn test_resubscribe_preserves_exact_filters() {
    let bus: Bus<Origin, Kind, i32> = Bus::new();
    let sub1 = bus
        .subscribe(Filter::Is(Origin::Sensor), Filter::Is(Kind::Temperature))
        .unwrap();
    let sub2 = sub1.resubscribe().unwrap();

    // Matching event: both should get it.
    bus.publish(Event::new(Origin::Sensor, Kind::Temperature, 1))
        .unwrap();
    assert_eq!(sub1.recv().unwrap().unwrap().payload(), &1);
    assert_eq!(sub2.recv().unwrap().unwrap().payload(), &1);

    // Non-matching event: neither should get it.
    bus.publish(Event::new(Origin::Other, Kind::Humidity, 2))
        .unwrap();
    assert!(sub1.recv().unwrap().is_none());
    assert!(sub2.recv().unwrap().is_none());
}

#[test]
fn test_overlapping_filter_buckets_no_double_delivery() {
    let bus: Bus<Origin, Kind, i32> = Bus::new();
    let wildcard = bus.subscribe(Filter::Any, Filter::Any).unwrap();
    let exact = bus
        .subscribe(Filter::Is(Origin::Sensor), Filter::Is(Kind::Temperature))
        .unwrap();

    bus.publish(Event::new(Origin::Sensor, Kind::Temperature, 42))
        .unwrap();

    // Each subscriber sits in a distinct bucket, so each receives exactly one copy.
    let w: Vec<_> = wildcard.drain().collect();
    let e: Vec<_> = exact.drain().collect();
    assert_eq!(w.len(), 1);
    assert_eq!(e.len(), 1);
}

#[test]
fn test_mixed_filter_overlap_all_four_buckets() {
    let bus: Bus<Origin, Kind, i32> = Bus::new();
    let exact = bus
        .subscribe(Filter::Is(Origin::Sensor), Filter::Is(Kind::Temperature))
        .unwrap();
    let origin_wild = bus
        .subscribe(Filter::Is(Origin::Sensor), Filter::Any)
        .unwrap();
    let kind_wild = bus
        .subscribe(Filter::Any, Filter::Is(Kind::Temperature))
        .unwrap();
    let all_wild = bus.subscribe(Filter::Any, Filter::Any).unwrap();

    let result = bus
        .publish(Event::new(Origin::Sensor, Kind::Temperature, 1))
        .unwrap();
    assert_eq!(result.delivered, 4);
    assert_eq!(result.dropped, 0);

    assert!(exact.recv().unwrap().is_some());
    assert!(origin_wild.recv().unwrap().is_some());
    assert!(kind_wild.recv().unwrap().is_some());
    assert!(all_wild.recv().unwrap().is_some());
}

#[test]
fn test_drain_empty_queue() {
    let bus: Bus<Origin, Kind, i32> = Bus::new();
    let sub = bus.subscribe(Filter::Any, Filter::Any).unwrap();
    let events: Vec<_> = sub.drain().collect();
    assert!(events.is_empty());
}

#[test]
fn test_concurrent_publish_from_multiple_threads() {
    let bus: Bus<Origin, Kind, i32> = Bus::with_capacity(1024);
    let sub = bus.subscribe(Filter::Any, Filter::Any).unwrap();
    let num_threads: usize = 8;
    let events_per_thread: usize = 100;

    let handles: Vec<_> = (0..num_threads)
        .map(|t| {
            let bus = bus.clone();
            std::thread::spawn(move || {
                for i in 0..events_per_thread {
                    bus.publish(Event::new(
                        Origin::Sensor,
                        Kind::Temperature,
                        (t * 1000 + i) as i32,
                    ))
                    .unwrap();
                }
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }

    let events: Vec<_> = sub.drain().collect();
    assert_eq!(events.len(), num_threads * events_per_thread);
}

#[test]
fn test_concurrent_subscribe_and_publish() {
    let bus: Bus<Origin, Kind, i32> = Bus::new();
    let bus_pub = bus.clone();
    let bus_sub = bus.clone();

    // One thread subscribing/unsubscribing repeatedly
    let sub_handle = std::thread::spawn(move || {
        for _ in 0..100 {
            let _sub = bus_sub.subscribe(Filter::Any, Filter::Any).unwrap();
            // _sub dropped here, exercising Drop under contention
        }
    });

    // Another thread publishing repeatedly
    let pub_handle = std::thread::spawn(move || {
        for i in 0..100 {
            let _ = bus_pub.publish(Event::new(Origin::Sensor, Kind::Temperature, i));
        }
    });

    sub_handle.join().unwrap();
    pub_handle.join().unwrap();
}

#[test]
fn test_publish_result_equality_and_copy() {
    let a = omnibus::PublishResult {
        delivered: 1,
        dropped: 2,
    };
    let b = omnibus::PublishResult {
        delivered: 1,
        dropped: 2,
    };
    let c = omnibus::PublishResult {
        delivered: 0,
        dropped: 0,
    };
    assert_eq!(a, b);
    assert_ne!(a, c);

    // Copy and Clone
    let d = a;
    let e = a.clone();
    assert_eq!(d, e);
}

#[test]
fn test_filter_default_is_any() {
    let f: Filter<Origin> = Filter::default();
    assert_eq!(f, Filter::Any);
}

#[test]
fn test_filter_equality_and_clone() {
    let a = Filter::Is(Origin::Sensor);
    let b = a.clone();
    assert_eq!(a, b);
    assert_ne!(a, Filter::Is(Origin::Other));
    assert_ne!(a, Filter::<Origin>::Any);
}

#[test]
fn test_event_clone() {
    let ev = Event::new(Origin::Sensor, Kind::Temperature, 42);
    let ev2 = ev.clone();
    assert_eq!(ev.origin(), ev2.origin());
    assert_eq!(ev.kind(), ev2.kind());
    assert_eq!(ev.payload(), ev2.payload());
}

#[test]
fn test_error_display_disconnected() {
    let err = OmnibusError::Disconnected;
    assert_eq!(err.to_string(), "channel disconnected");
}

#[test]
fn test_error_display_poisoned() {
    let err = OmnibusError::Poisoned;
    assert_eq!(err.to_string(), "internal lock poisoned");
}

#[test]
fn test_drop_multiple_subscribers_same_filter() {
    let bus: Bus<Origin, Kind, i32> = Bus::new();
    let sub1 = bus.subscribe(Filter::Any, Filter::Any).unwrap();
    let sub2 = bus.subscribe(Filter::Any, Filter::Any).unwrap();
    let sub3 = bus.subscribe(Filter::Any, Filter::Any).unwrap();

    // Drop in arbitrary order — map cleanup should handle partial vec removal.
    drop(sub2);
    let result = bus
        .publish(Event::new(Origin::Sensor, Kind::Temperature, 1))
        .unwrap();
    assert_eq!(result.delivered, 2);

    drop(sub1);
    let result = bus
        .publish(Event::new(Origin::Sensor, Kind::Temperature, 2))
        .unwrap();
    assert_eq!(result.delivered, 1);

    drop(sub3);
    let result = bus
        .publish(Event::new(Origin::Sensor, Kind::Temperature, 3))
        .unwrap();
    assert_eq!(result.delivered, 0);
    assert_eq!(result.dropped, 0);
}

#[test]
fn test_recv_blocking_threaded_multiple_events() {
    let bus: Bus<Origin, Kind, i32> = Bus::new();
    let sub = bus.subscribe(Filter::Any, Filter::Any).unwrap();
    let bus_clone = bus.clone();

    std::thread::spawn(move || {
        for i in 0..5 {
            std::thread::sleep(Duration::from_millis(10));
            bus_clone
                .publish(Event::new(Origin::Sensor, Kind::Temperature, i))
                .unwrap();
        }
    });

    for i in 0..5 {
        let ev = sub.recv_blocking().unwrap();
        assert_eq!(ev.payload(), &i);
    }
}

#[test]
fn test_subscribe_after_publish_misses_earlier_events() {
    let bus: Bus<Origin, Kind, i32> = Bus::new();
    bus.publish(Event::new(Origin::Sensor, Kind::Temperature, 1))
        .unwrap();
    let sub = bus.subscribe(Filter::Any, Filter::Any).unwrap();
    assert!(sub.recv().unwrap().is_none());
}

#[test]
fn test_publish_result_debug_format() {
    let r = omnibus::PublishResult {
        delivered: 3,
        dropped: 1,
    };
    let dbg = format!("{:?}", r);
    assert!(dbg.contains("delivered"));
    assert!(dbg.contains("dropped"));
}

#[test]
fn test_error_is_std_error() {
    fn assert_std_error<E: std::error::Error>() {}
    assert_std_error::<OmnibusError>();
}
