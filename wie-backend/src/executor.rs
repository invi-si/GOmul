use alloc::{boxed::Box, sync::Arc};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

use hashbrown::HashMap;
use spin::Mutex;

use wie_util::{Result, WieError};

use crate::time::Instant;

type Task = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

pub struct ExecutorInner {
    current_task_id: Option<usize>,
    tasks: HashMap<usize, Task>,
    sleeping_tasks: HashMap<usize, Instant>,
    last_task_id: usize,
    last_now: Instant,
}

pub trait AsyncCallable<R>: Send
where
    R: Send,
{
    fn call(self) -> impl Future<Output = R> + Send;
}

impl<F, R, Fut> AsyncCallable<R> for F
where
    F: FnOnce() -> Fut + 'static + Send,
    R: AsyncCallableResult,
    Fut: Future<Output = R> + 'static + Send,
{
    async fn call(self) -> R {
        self().await
    }
}

pub trait AsyncCallableResult: Send {
    fn err(self) -> Option<WieError>;
}

impl<R> AsyncCallableResult for core::result::Result<R, WieError>
where
    R: Send,
{
    fn err(self) -> Option<WieError> {
        self.err()
    }
}

impl AsyncCallableResult for () {
    fn err(self) -> Option<WieError> {
        None
    }
}

#[derive(Clone)]
pub struct Executor {
    inner: Arc<Mutex<ExecutorInner>>,
}

impl Executor {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let inner = Arc::new(Mutex::new(ExecutorInner {
            current_task_id: None,
            tasks: HashMap::new(),
            sleeping_tasks: HashMap::new(),
            last_task_id: 0,
            last_now: Instant::from_epoch_millis(0),
        }));

        Self { inner }
    }

    /// Called only after the owning emulator has stopped polling its tasks.
    /// Futures can own Executor/System clones; release them outside the lock.
    pub fn shutdown(&self) {
        let tasks = {
            let mut inner = self.inner.lock();
            inner.sleeping_tasks.clear();
            inner.current_task_id = None;
            core::mem::take(&mut inner.tasks)
        };
        drop(tasks);
    }

    pub fn spawn<C, R>(&self, callable: C) -> usize
    where
        C: AsyncCallable<R> + 'static,
        R: AsyncCallableResult,
    {
        let fut = async move {
            let result = callable.call().await;
            if let Some(err) = result.err() {
                return Err(err);
            }

            Ok(())
        };

        let task_id = {
            let mut inner = self.inner.lock();
            inner.last_task_id += 1;
            inner.last_task_id
        };

        self.inner.lock().tasks.insert(task_id, Box::pin(fut));

        task_id
    }

    // TODO we need to remove error handling from here. we need to JoinHandle like on spawn..
    pub fn tick<T>(&mut self, now: T) -> Result<()>
    where
        T: Fn() -> Instant,
    {
        self.tick_with_order(now, |_| Ok(()))
    }

    pub fn tick_with_order<T, O>(&mut self, now: T, order: O) -> Result<()>
    where
        T: Fn() -> Instant,
        O: Fn(&mut [usize]) -> Result<()>,
    {
        let mut trace = wie_util::input_trace::span(wie_util::input_trace::EXECUTOR, 0);
        let end = now() + 8; // TODO hardcoded
        loop {
            let now = now();

            if now > end {
                trace.result = 1;
                break;
            }

            {
                let inner = self.inner.lock();
                // No work can consume the remaining slice. In particular, a
                // deterministic clock need not advance after its last task ends.
                if inner.tasks.is_empty() {
                    break;
                }
                let running_task_count = inner.tasks.len() - inner.sleeping_tasks.len();
                if running_task_count == 0 && !inner.sleeping_tasks.is_empty() {
                    let next_wakeup = *inner.sleeping_tasks.values().min().unwrap();
                    if now < next_wakeup {
                        trace.result = 2;
                        break;
                    }
                }
            }

            self.step(now, &order)?;
        }

        Ok(())
    }

    pub fn current_task_id(&self) -> u64 {
        self.inner.lock().current_task_id.unwrap() as _
    }

    fn step(&mut self, now: Instant, _order: &impl Fn(&mut [usize]) -> Result<()>) -> Result<()> {
        let _trace = wie_util::input_trace::span(wie_util::input_trace::PASS, now.raw());
        self.inner.lock().last_now = now;

        let mut next_tasks = HashMap::new();
        let tasks = self.inner.lock().tasks.drain().collect::<HashMap<_, _>>();
        let mut sleeping_tasks = self.inner.lock().sleeping_tasks.drain().collect::<HashMap<_, _>>();

        wie_util::input_trace::event(wie_util::input_trace::TASK_COUNTS, b'I', tasks.len() as u64, sleeping_tasks.len() as u64);
        let mut first_error = None;

        #[cfg(feature = "checkpoint-replay")]
        let tasks = {
            let mut tasks = tasks;
            let mut task_ids: alloc::vec::Vec<_> = tasks.keys().copied().collect();
            _order(&mut task_ids)?;
            task_ids
                .into_iter()
                .map(|id| {
                    tasks
                        .remove(&id)
                        .map(|task| (id, task))
                        .ok_or_else(|| WieError::FatalError("Checkpoint task order mismatch".into()))
                })
                .collect::<Result<alloc::vec::Vec<_>>>()?
        };
        for (task_id, mut task) in tasks.into_iter() {
            let item = sleeping_tasks.get(&task_id);
            if let Some(item) = item {
                if *item <= now {
                    sleeping_tasks.remove(&task_id);
                } else {
                    next_tasks.insert(task_id, task);
                    continue;
                }
            }

            let waker = self.create_waker();
            let mut context = Context::from_waker(&waker);
            self.inner.lock().current_task_id = Some(task_id);

            let poll = {
                let mut trace = wie_util::input_trace::span(wie_util::input_trace::TASK, task_id as u64);
                let poll = task.as_mut().poll(&mut context);
                trace.result = u64::from(poll.is_ready());
                poll
            };
            match poll {
                Poll::Ready(Ok(())) => {}
                Poll::Ready(Err(err)) => {
                    if first_error.is_none() {
                        first_error = Some(err);
                    }
                }
                Poll::Pending => {
                    next_tasks.insert(task_id, task);
                }
            }

            self.inner.lock().current_task_id = None;
        }

        self.inner.lock().sleeping_tasks.extend(sleeping_tasks);
        self.inner.lock().tasks.extend(next_tasks);

        if let Some(err) = first_error { Err(err) } else { Ok(()) }
    }

    pub(crate) fn sleep(&self, timeout: u64) {
        let task_id = self.inner.lock().current_task_id.unwrap();

        let until = self.inner.lock().last_now + timeout;
        self.inner.lock().sleeping_tasks.insert(task_id, until);
    }

    fn create_waker(&self) -> Waker {
        unsafe fn noop_clone(_data: *const ()) -> RawWaker {
            noop_raw_waker()
        }

        unsafe fn noop(_data: *const ()) {}

        const NOOP_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(noop_clone, noop, noop, noop);

        const fn noop_raw_waker() -> RawWaker {
            RawWaker::new(core::ptr::null(), &NOOP_WAKER_VTABLE)
        }

        unsafe { Waker::from_raw(noop_raw_waker()) }
    }
}

#[cfg(test)]
mod tests {
    use alloc::sync::Arc;
    use core::{
        cell::Cell,
        future::Future,
        pin::Pin,
        sync::atomic::{AtomicBool, Ordering},
        task::{Context, Poll},
    };

    use wie_util::WieError;

    use super::Executor;
    use crate::time::Instant;

    #[test]
    fn shutdown_releases_tasks_that_own_the_executor() {
        let executor = Executor::new();
        let weak = Arc::downgrade(&executor.inner);
        let captured = executor.clone();
        executor.spawn(async move || {
            let _keep_alive = captured;
            core::future::pending::<()>().await;
            Ok::<(), WieError>(())
        });
        assert!(Arc::strong_count(&executor.inner) > 1);
        executor.shutdown();
        executor.shutdown();
        assert!(executor.inner.lock().tasks.is_empty());
        drop(executor);
        assert!(weak.upgrade().is_none());
    }

    struct YieldOnce(bool);

    impl Future for YieldOnce {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
            if self.0 {
                Poll::Ready(())
            } else {
                self.0 = true;
                Poll::Pending
            }
        }
    }

    fn advancing_clock(start: u64) -> impl Fn() -> Instant {
        let time = Cell::new(start);
        move || {
            let now = time.get();
            time.set(now + 1);
            Instant::from_epoch_millis(now)
        }
    }

    #[test]
    fn idle_executor_returns_with_a_frozen_clock_and_can_resume() {
        fn frozen_clock() -> impl Fn() -> Instant {
            let reads = Cell::new(0);
            move || {
                reads.set(reads.get() + 1);
                assert!(reads.get() < 32, "idle executor waited for wall-clock progress");
                Instant::from_epoch_millis(0)
            }
        }

        let mut executor = Executor::new();
        executor.tick(frozen_clock()).unwrap();
        let completed = Arc::new(AtomicBool::new(false));
        let task_completed = completed.clone();
        executor.spawn(move || async move {
            YieldOnce(false).await;
            task_completed.store(true, Ordering::Relaxed);
        });
        executor.tick(frozen_clock()).unwrap();
        assert!(completed.load(Ordering::Relaxed));
        executor.tick(frozen_clock()).unwrap();
    }

    #[test]
    fn test_failed_task_preserves_others() {
        let mut executor = Executor::new();

        executor.spawn(|| async { Err::<(), _>(WieError::FatalError("test error".into())) });

        let completed = Arc::new(AtomicBool::new(false));
        let completed_clone = completed.clone();
        executor.spawn(move || async move {
            YieldOnce(false).await;
            completed_clone.store(true, Ordering::Relaxed);
        });

        assert!(executor.tick(advancing_clock(0)).is_err());
        assert!(!completed.load(Ordering::Relaxed));

        executor.tick(advancing_clock(100)).unwrap();
        assert!(completed.load(Ordering::Relaxed));
    }

    #[test]
    fn test_failed_task_preserves_sleeping_tasks() {
        let mut executor = Executor::new();

        let completed = Arc::new(AtomicBool::new(false));
        let completed_clone = completed.clone();
        let executor_clone = executor.clone();
        executor.spawn(move || async move {
            executor_clone.sleep(100);
            YieldOnce(false).await;
            completed_clone.store(true, Ordering::Relaxed);
        });

        executor.spawn(|| async { Err::<(), _>(WieError::FatalError("test error".into())) });

        assert!(executor.tick(advancing_clock(0)).is_err());
        assert!(!completed.load(Ordering::Relaxed));

        executor.tick(advancing_clock(50)).unwrap();
        assert!(!completed.load(Ordering::Relaxed));

        executor.tick(advancing_clock(200)).unwrap();
        assert!(completed.load(Ordering::Relaxed));
    }

    #[test]
    fn test_all_ok_tasks_complete() {
        let mut executor = Executor::new();

        let completed_a = Arc::new(AtomicBool::new(false));
        let completed_a_clone = completed_a.clone();
        executor.spawn(move || async move {
            completed_a_clone.store(true, Ordering::Relaxed);
        });

        let completed_b = Arc::new(AtomicBool::new(false));
        let completed_b_clone = completed_b.clone();
        executor.spawn(move || async move {
            YieldOnce(false).await;
            completed_b_clone.store(true, Ordering::Relaxed);
        });

        executor.tick(advancing_clock(0)).unwrap();

        assert!(completed_a.load(Ordering::Relaxed));
        assert!(completed_b.load(Ordering::Relaxed));
    }
}
