//! Work spread across a few threads.

/// `work` done on every item, across `threads` threads, in the items' order.
pub(super) fn in_parallel<T: Send, U: Send>(
    items: Vec<T>,
    threads: usize,
    work: impl Fn(T) -> U + Sync,
) -> Vec<U> {
    let count = items.len();
    let queue = std::sync::Mutex::new(items.into_iter().enumerate().collect::<Vec<_>>());
    let results = std::sync::Mutex::new((0..count).map(|_| None).collect::<Vec<Option<U>>>());
    std::thread::scope(|scope| {
        for _ in 0..threads.min(count.max(1)) {
            scope.spawn(|| {
                loop {
                    let next = queue.lock().expect("the queue is never poisoned").pop();
                    let Some((index, item)) = next else { break };
                    let result = work(item);
                    results.lock().expect("the results are never poisoned")[index] = Some(result);
                }
            });
        }
    });
    results
        .into_inner()
        .expect("the results are never poisoned")
        .into_iter()
        .map(|r| r.expect("every item is worked"))
        .collect()
}
