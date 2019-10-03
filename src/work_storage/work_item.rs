pub enum WorkItem<T> {
    Value(T),
    Dropped,
    Stop
}

pub struct TimestampedWorkItem<T>(pub WorkItem<T>, pub u64);
