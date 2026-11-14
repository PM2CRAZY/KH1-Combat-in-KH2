//! KH1's held hits under Stop, as a queue KH2's hit delivery feeds (pure; the hook side is
//! `win::held_hits`).
//!
//! KH1 (Checked (decomp), KH1-in-ReCom `recom-m7-magic.md` "M7 f3"): a hit on a stopped actor
//! (`+0x88 & 1`) is built as a record into a 128-record pool shared by everyone (`0x1402BF630`,
//! pool `0x142DE6E10`) and queued on the actor (`+0x3EC`). If the queue already holds a record, the
//! new one gets a delay of `6 − (int)(r × −6)` frames, `r` = KH1's `0x14031BA30` random in [0, 1):
//! 6..11 frames. While stopped the actor's update skips its hit pass (`0x140297FE0`). Once Stop is
//! over, each frame the hit pass (`0x1402B8E10` → `0x1402BF160`) pops: a head with a delay loses one
//! and ends the pop for the frame; a head with none is applied (`0x1402B7B90`, which drops it when
//! the actor is gone or dead) and the next head is looked at in the same frame. The damage was
//! computed when the record was built (`0x1402B7B90` reads it from the record, `+8`), as KH2's is
//! (`0x1403CE740`).

/// KH2's hit record (`0x1403CE740`, pool `0x142A24530`): 0x40 bytes.
pub const RECORD: usize = 0x40;
/// KH1's record pool: 128 for everyone.
pub const POOL: usize = 128;

#[derive(Clone)]
pub struct Held {
    /// The struck object's KH2 handle.
    pub target: u32,
    pub record: [u8; RECORD],
    /// Frames still to wait at the head of its queue.
    pub delay: u8,
    /// The attack's centre at the hit, for a push away from it (KH2's push kind 5 asks the attack
    /// object, which may be gone by the landing).
    pub centre: Option<[f32; 4]>,
}

/// KH1's delay for a record joining a queue that isn't empty: `6 − (int)(r × −6)`.
pub fn kh1_delay(r: f32) -> u8 {
    (6 - (r * -6.0) as i32) as u8
}

/// A small generator standing in for KH1's `rand()` (Assumption: only its law matters, 6..11
/// evenly; KH1's draws are shared with everything else it rolls).
pub struct Rand(u32);

impl Rand {
    pub fn new(seed: u32) -> Rand {
        Rand(seed | 1)
    }
    /// In [0, 1), 24 bits, as KH1's three low bytes / 2^24.
    pub fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 & 0xFF_FFFF) as f32 / 16_777_216.0
    }
}

#[derive(Default)]
pub struct Queue {
    held: Vec<Held>,
}

impl Queue {
    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }
    pub fn len(&self) -> usize {
        self.held.len()
    }
    /// Holds a hit on `target`: false (deliver it now) when KH1's pool is full.
    pub fn hold(&mut self, target: u32, record: &[u8; RECORD], centre: Option<[f32; 4]>, rand: &mut Rand) -> bool {
        if self.held.len() >= POOL {
            return false;
        }
        let delay = if self.held.iter().any(|h| h.target == target) { kh1_delay(rand.next()) } else { 0 };
        self.held.push(Held { target, record: *record, delay, centre });
        true
    }
    /// The targets with hits held.
    pub fn targets(&self) -> Vec<u32> {
        let mut t: Vec<u32> = self.held.iter().map(|h| h.target).collect();
        t.dedup();
        t.sort_unstable();
        t.dedup();
        t
    }
    /// KH1's hit pass for one target no longer stopped, this frame: the records to apply now, in
    /// order.
    pub fn pop(&mut self, target: u32) -> Vec<Held> {
        let mut out = Vec::new();
        while let Some(i) = self.held.iter().position(|h| h.target == target) {
            if self.held[i].delay > 0 {
                self.held[i].delay -= 1;
                break;
            }
            out.push(self.held.remove(i));
        }
        out
    }
    /// Forgets a target's hits (it is gone).
    pub fn drop_target(&mut self, target: u32) -> usize {
        let n = self.held.len();
        self.held.retain(|h| h.target != target);
        n - self.held.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kh1_delays_are_6_to_11_evenly() {
        assert_eq!(kh1_delay(0.0), 6);
        assert_eq!(kh1_delay(0.1666), 6);
        assert_eq!(kh1_delay(0.17), 7);
        assert_eq!(kh1_delay(0.9999), 11);
        let mut r = Rand::new(12345);
        let mut seen = [0u32; 12];
        for _ in 0..60000 {
            seen[kh1_delay(r.next()) as usize] += 1;
        }
        assert!(seen[..6].iter().all(|&n| n == 0));
        assert!(seen[6..].iter().all(|&n| (9000..11000).contains(&n)), "{seen:?}");
    }

    #[test]
    fn held_hits_land_as_kh1s_hit_pass() {
        let mut q = Queue::default();
        let mut r = Rand::new(7);
        let rec = |n: u8| [n; RECORD];
        for n in 0..4 {
            assert!(q.hold(5, &rec(n), None, &mut r));
        }
        q.hold(9, &rec(100), None, &mut r);
        let delays: Vec<u8> = q.held.iter().map(|h| h.delay).collect();
        assert_eq!(delays[0], 0);
        assert_eq!(delays[4], 0, "another target's first record has none");
        assert!(delays[1..4].iter().all(|d| (6..=11).contains(d)));
        // Frame by frame after Stop: the first lands at once, each next one its own delay later.
        let mut landed = Vec::new();
        for frame in 0..60 {
            for r in q.pop(5) {
                landed.push((frame, r.record[0]));
            }
        }
        let want: Vec<(i32, u8)> = {
            let mut f = 0;
            let mut v = vec![(0, 0)];
            for n in 1..4 {
                f += delays[n as usize] as i32;
                v.push((f, n));
            }
            v
        };
        assert_eq!(landed, want);
        assert_eq!(q.len(), 1);
        assert_eq!(q.pop(9).len(), 1);
    }

    #[test]
    fn kh1s_pool_is_128_for_everyone() {
        let mut q = Queue::default();
        let mut r = Rand::new(1);
        for i in 0..POOL {
            assert!(q.hold(i as u32 % 3, &[0; RECORD], None, &mut r));
        }
        assert!(!q.hold(1, &[0; RECORD], None, &mut r));
        assert_eq!(q.drop_target(1), 43);
    }
}
