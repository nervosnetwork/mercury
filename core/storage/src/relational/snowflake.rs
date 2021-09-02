use dashmap::DashMap; 

use std::sync::atomic::{AtomicI64, Ordering};

#[derive(Default, Debug)]
pub struct Snowflake {
    center_id: AtomicI64,
    machine_id: AtomicI64,
    sequence: DashMap<i64, i64>,
}

impl Clone for Snowflake {
    fn clone(&self) -> Self {
        Snowflake {
            center_id: AtomicI64::new(self.center_id.load(Ordering::Relaxed)),
            machine_id: AtomicI64::new(self.machine_id.load(Ordering::Relaxed)),
            sequence: self.sequence.clone(),
        }
    }
}

impl Snowflake {
    pub fn new(center_id: u16, machine_id: u16) -> Self {
        Snowflake {
            center_id: AtomicI64::new(center_id as i64),
            machine_id: AtomicI64::new(machine_id as i64),
            sequence: DashMap::new(),
        }
    }

    pub fn set_info(&self, center_id: u16, machine_id: u16) {
        self.center_id.swap(center_id as i64, Ordering::Acquire);
        self.machine_id.swap(machine_id as i64, Ordering::Acquire);
    }

    pub fn generate(&self, number: i64) -> i64 {
        let center_id = self.center_id.load(Ordering::Relaxed);
        let machine_id = self.machine_id.load(Ordering::Relaxed);
        let seq = if let Some(mut last) = self.sequence.get_mut(&number) {
            let now = (*last + 1) & (-1 ^ (-1 << 22));
            *last = now;
            now
        } else {
            self.sequence.insert(number, 1);
            1i64
        };

        (number << 32) | (center_id << 27) | (machine_id << 22) | seq
    }

    pub fn get_info(&self) -> (i64, i64) {
        (
            self.center_id.load(Ordering::Relaxed),
            self.machine_id.load(Ordering::Relaxed),
        )
    }

    pub fn clear_sequence(&self) {
        self.sequence.clear();
    }

    pub fn prepare(&self, start: i64, end: i64) {
        for i in start..=end {
            self.sequence.insert(i, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::relational::generate_id;
    use std::thread;

    #[test]
    fn test_snowflake() {
        let block_number = 0u64;
        let mut id = 0i64;

        for _ in 0..100 {
            let new = generate_id(block_number);
            assert!(new > id);
            id = new;
        }
    }

    #[test]
    fn test_par_snowflake() {
        let block_number = 0u64;
        let id_1 = generate_id(block_number);

        thread::spawn(move || {
            let id_2 = generate_id(block_number);
            assert!(id_2 > id_1);
        });
    }
}
