#![feature(test)]

extern crate test;

use test::Bencher;

use conquer_util::BackOff;

#[bench]
fn spin_once(b: &mut Bencher) {
    b.iter(|| {
        let mut backoff = BackOff::new();
        backoff.spin();
    })
}

#[bench]
fn spin_full(b: &mut Bencher) {
    b.iter(|| {
        let mut backoff = BackOff::new();
        while !backoff.advise_yield() {
            backoff.spin();
        }
    })
}
