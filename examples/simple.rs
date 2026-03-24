use omnibus::{Bus, Event, Filter};

fn main() {
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

    let event = sub.recv().unwrap();

    println!("Received event: {:?}", event);
}
