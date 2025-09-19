// This code is handed from Embedded Rust documentation and
// is accessible from https://docs.rust-embedded.org/cortex-m-rt/0.6.0/rand/trait.RngCore.html

use rand_core::{impls, RngCore};

pub struct CountingRng(pub u64);

impl RngCore for CountingRng {
    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    fn next_u64(&mut self) -> u64 {
        self.0 += 1;
        if self.0 > u16::MAX as u64 {
            self.0 = 1;
        }
        self.0
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        impls::fill_bytes_via_next(self, dest)
    }
}
