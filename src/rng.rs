use std::time::{SystemTime, UNIX_EPOCH};

pub struct Rng {
    state: i32
}

impl Rng {
    pub fn new() -> Rng {
        let start = SystemTime::now();
        Rng { state: start.duration_since(UNIX_EPOCH).unwrap().as_secs() as i32 }
    }

    // Lehmer random number generator
    pub fn next(&mut self) -> i32 {
        self.state = ((self.state as u64) * 48271 % 0x7fff_ffff) as i32;
        self.state
    }

    pub(crate) fn gen_range(&mut self, arg: std::ops::Range<u8>) -> u32 {
        let res = (arg.start as usize) + (self.next() as usize) % (arg.end - arg.start) as usize;
        res as u32
    }
}