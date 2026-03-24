use std::time::Duration;

use omnibus::{Bus, Event, Filter};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Origin {
    Sensor,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Kind {
    Temperature,
    Humidity,
}

fn main() {
    let bus: Bus<Origin, Kind, String> = Bus::new();

    // Wildcard: receives every event
    let wildcard = bus.subscribe(Filter::Any, Filter::Any).unwrap();

    // Filtered by origin only
    let sensor_sub = bus
        .subscribe(Filter::Is(Origin::Sensor), Filter::Any)
        .unwrap();

    // Filtered by event kind only
    let temp_sub = bus
        .subscribe(Filter::Any, Filter::Is(Kind::Temperature))
        .unwrap();

    // Exact match
    let exact_sub = bus
        .subscribe(Filter::Is(Origin::Sensor), Filter::Is(Kind::Temperature))
        .unwrap();

    // Resubscribe: creates a new independent subscription with the same filters
    let wildcard2 = wildcard.resubscribe().unwrap();

    // Publish events and inspect delivery results
    let r = bus
        .publish(Event::new(
            Origin::Sensor,
            Kind::Temperature,
            "42.5°C".to_string(),
        ))
        .unwrap();
    println!(
        "Published Sensor/Temperature: delivered={}, dropped={}",
        r.delivered, r.dropped
    );

    let r = bus
        .publish(Event::new(
            Origin::Sensor,
            Kind::Humidity,
            "60%".to_string(),
        ))
        .unwrap();
    println!(
        "Published Sensor/Humidity:    delivered={}, dropped={}",
        r.delivered, r.dropped
    );

    let r = bus
        .publish(Event::new(
            Origin::Other,
            Kind::Temperature,
            "38.0°C".to_string(),
        ))
        .unwrap();
    println!(
        "Published Other/Temperature:  delivered={}, dropped={}",
        r.delivered, r.dropped
    );

    println!("\n=== wildcard (drain, receives all 3) ===");
    for ev in wildcard.drain() {
        println!("  [{:?} / {:?}] {}", ev.origin(), ev.kind(), ev.payload());
    }

    println!("=== wildcard2 (resubscribed, also receives all 3) ===");
    for ev in wildcard2.drain() {
        println!("  [{:?} / {:?}] {}", ev.origin(), ev.kind(), ev.payload());
    }

    println!("=== sensor_sub (origin=Sensor, receives 2) ===");
    while let Ok(Some(ev)) = sensor_sub.recv() {
        println!("  [{:?} / {:?}] {}", ev.origin(), ev.kind(), ev.payload());
    }

    println!("=== temp_sub (kind=Temperature, receives 2) ===");
    while let Ok(Some(ev)) = temp_sub.recv() {
        println!("  [{:?} / {:?}] {}", ev.origin(), ev.kind(), ev.payload());
    }

    println!("=== exact_sub (Sensor/Temperature, receives 1) ===");
    while let Ok(Some(ev)) = exact_sub.recv() {
        println!("  [{:?} / {:?}] {}", ev.origin(), ev.kind(), ev.payload());
    }

    // Demonstrate recv_timeout — no events queued, times out
    println!("\n=== recv_timeout (50ms, no event) ===");
    match wildcard.recv_timeout(Duration::from_millis(50)) {
        Ok(None) => println!("  Timed out as expected"),
        Ok(Some(ev)) => println!("  Got: {:?}", ev.payload()),
        Err(e) => println!("  Error: {e}"),
    }
}
