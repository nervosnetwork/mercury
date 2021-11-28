use common::{async_trait, Result};

#[derive(Clone, Debug)]
pub struct Task {
    id: u64,
}

impl Task {
    const TASK_LEN: u64 = 100_000;

    pub fn new(id: u64) -> Task {
        Task { id }
    }
}
