#![feature(test)]

extern crate test;

use test::Bencher;

use conquer_util::BackOff;

#[bench]
fn spin_once(b: &mut Bencher) {
    b.iter(|| {
        let backoff = BackOff::new();
        backoff.spin();
    })
}

#[bench]
fn spin_full(b: &mut Bencher) {
    b.iter(|| {
        let backoff = BackOff::new();
        while !backoff.advise_yield() {
            backoff.spin();
        }
    })
}

#[bench]
fn spin_full_random(b: &mut Bencher) {
    b.iter(|| {
        let backoff = BackOff::random();
        while !backoff.advise_yield() {
            backoff.spin();
        }
    })
}

#[bench]
fn spin_for(b: &mut Bencher) {
    b.iter(|| {
        use std::time::Duration;
        BackOff::spin_for(Duration::from_nanos(100));
    })
}
