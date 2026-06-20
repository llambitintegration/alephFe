//! `marathon-fleet` daemon entry point.
//!
//! This binary is the out-of-process fleet daemon. It runs separately from the
//! simulation process and must never block the sim tick loop; the sim consumes
//! the daemon's output over a latest-wins channel and degrades gracefully when
//! the daemon is absent. No real pipeline behavior is wired up yet — this is the
//! process seam only.

fn main() {
    // Touch the library so the binary depends on it; real wiring lands later.
    let _ = marathon_fleet::EntityKind::Unknown;
    println!("marathon-fleet daemon");
}
