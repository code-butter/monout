use futures::stream::FuturesUnordered;
use futures::StreamExt;

pub struct FuturesCounter<T> {
    inner: FuturesUnordered<T>,
    count: usize,
}

impl<T: futures::Future> FuturesCounter<T> {
    pub fn new() -> Self {
        Self {
            inner: FuturesUnordered::new(),
            count: 0
        }
    }

    pub fn push(&mut self, future: T) {
        self.inner.push(future);
        self.count += 1;
    }

    pub async fn next(&mut self) -> Option<<T as std::future::Future>::Output> {
        if let Some(result) = self.inner.next().await {
            self.count -= 1;
            Some(result)
        } else {
            None
        }
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}