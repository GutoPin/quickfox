//! Local engine microbenchmark; excludes terminal rendering and filesystem I/O.
use quickfox::game::Session;
use std::{
    hint::black_box,
    time::{Duration, Instant},
};
fn main() {
    let mut samples = Vec::with_capacity(20000);
    for _ in 0..100 {
        let mut session = Session::new("a ".repeat(150), None, 0, None);
        for i in 0..200 {
            let start = Instant::now();
            session.input(
                if i % 11 == 0 {
                    'x'
                } else if i % 2 == 0 {
                    'a'
                } else {
                    ' '
                },
                Duration::from_millis(i * 100),
            );
            black_box(session.metrics());
            samples.push(start.elapsed().as_nanos());
        }
    }
    samples.sort_unstable();
    println!("input + metric calculation (20,000 samples; max 200 retained characters)");
    for percentile in [50, 95, 99] {
        println!(
            "p{percentile}: {} ns",
            samples[(samples.len() - 1) * percentile / 100]
        );
    }
    println!("Does not measure terminal rendering, input-device delay, or display latency.");
}
