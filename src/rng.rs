use std::{
    ops::{Bound, RangeBounds},
    time::{SystemTime, UNIX_EPOCH},
};

pub struct Rng {
    state: i32,
}

impl Rng {
    pub fn new() -> Rng {
        let start = SystemTime::now();
        Rng {
            state: start.duration_since(UNIX_EPOCH).unwrap().as_nanos() as i32,
        }
    }

    // Lehmer random number generator
    pub fn next(&mut self) -> i32 {
        self.state = ((self.state as u64) * 48271 % 0x7fff_ffff) as i32;
        self.state
    }

    pub(crate) fn gen_range<R: RangeBounds<u8>>(&mut self, arg: R) -> u32 {
        let bounds = (arg.start_bound(), arg.end_bound());
        let res = match bounds {
            (Bound::Included(a), Bound::Included(b)) => {
                (*a as usize) + (self.next() as usize) % (*b + 1 - *a) as usize
            }

            (Bound::Included(a), Bound::Excluded(b)) => {
                (*a as usize) + (self.next() as usize) % (*b - *a) as usize
            }
            _ => panic!("Unsupported range bounds"),
        };

        res as u32
    }
}
