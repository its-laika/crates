#![forbid(unsafe_code)]
//! # A dead simple one-shot single producer, multiple consumer (SPMC) channel
//!
//! Shotgun is a simple oneshot single producer, multiple consumer (SPMC)
//! channel. Internally using [`std::sync::Mutex`] and [`std::sync::Arc`], not
//! containing any unsafe code.

use std::{
    clone::Clone,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

/// Oneshot receiver of a [`channel`]
///
/// Use [`Receiver::try_recv`] or [`Receiver::recv`] to (try to) receive a value
/// from the channel, if it has been sent. As this is a oneshot receiver, only
/// one value can be received.
///
/// # Examples
///
/// ## Synchronous
///
/// ```rust
/// let (mut tx, rx) = laika::shotgun::channel();
///
/// // Initialy, oneshot receiver has no value
/// assert_eq!(rx.try_recv(), None);
///
/// // Send a value
/// tx.send(12);
///
/// // Now, oneshot receiver has the value
/// assert_eq!(rx.try_recv(), Some(12));
/// ```
///
/// ## Asynchronous
///
/// ```no_run
/// let (mut tx, rx) = laika::shotgun::channel();
///
/// // ... in any async runtime
///
/// let fun1 = async move {
///     rx.recv().await;
///     return 1;
/// };
///
/// // Send a value
/// tx.send(12);
/// ```
#[derive(Clone, Debug)]
pub struct Receiver<T>
where
    T: Clone,
{
    /// Inner receiver that holds the sent value and possible wakers
    inner: Arc<Mutex<_Receiver<T>>>,
}

/// Oneshot sender of a [`channel`]
///
/// Use [`Sender::send`] to send a value to all receivers of the channel.
/// As this is a oneshot sender, only one value can be sent.
///
/// # Examples
/// ## Send a value
///
/// ```rust
/// let (mut tx, rx) = laika::shotgun::channel();
///
/// // Send a value
/// tx.send(12);
/// ```
///
/// ## Sender is dropped after sending
///
/// ```compile_fail
/// let (mut tx, rx) = laika::shotgun::channel();
///
/// // Send a value
/// tx.send(12);
/// tx.send(13); // This won't compile
/// ```
#[derive(Debug)]
pub struct Sender<T>
where
    T: Clone,
{
    inner: _Sender<T>,
}

impl<T> Receiver<T>
where
    T: Clone,
{
    /// Try to receive a value from the channel, if it has been sent.
    /// As this is a oneshot receiver, only one value can be received.
    /// This function is **non-blocking** and just returns [`None`] if no value
    /// has been sent.
    ///
    /// # Panics
    ///
    /// Panics if mutex is poisened due to another thread panicking while using
    /// inner receiver too.
    ///
    /// # Examples
    /// ```rust
    /// let (mut tx, rx) = laika::shotgun::channel();
    ///
    /// // Initialy, oneshot receiver has no value
    /// assert_eq!(rx.try_recv(), None);
    ///
    /// // Send a value
    /// tx.send(12);
    ///
    /// // Now, oneshot receiver has the value
    /// assert_eq!(rx.try_recv(), Some(12));
    /// // Value is kept after being received
    /// assert_eq!(rx.try_recv(), Some(12));
    /// ```
    pub fn try_recv(&self) -> Option<T>
    where
        T: Clone,
    {
        self.inner
            .as_ref()
            .lock()
            .expect("Mutex is poisoned")
            .try_recv()
    }

    /// Receive a value from the channel.
    /// Waits until value has been sent and then returns it.
    /// This function is blocking asynchronously.
    ///
    /// # Note
    /// You can directly [`Future`]'s `.await` on the receiver too.
    ///
    /// # Examples
    /// (*Note that this won't compile because no async runtime exists here.*)
    /// ```compile_fail
    /// let (mut tx, rx) = laika::shotgun::channel();
    ///
    /// let fun1 = async move {
    ///     rx.recv().await;
    ///     return 1;
    /// };
    ///
    /// std::thread::sleep(std::time::Duration::from_secs(1));
    ///
    /// // Send a value
    /// tx.send(());
    ///
    /// // Now, oneshot receiver has the value
    /// assert_eq!(fun1.await, 1);
    /// ```
    pub async fn recv(self) -> T {
        self.await
    }
}

impl<T> Sender<T>
where
    T: Clone,
{
    /// Send a value to all receivers of the channel.
    /// As this is a oneshot sender, only one value can be sent.
    ///
    /// # Examples
    /// ## Send a value
    ///
    /// ```rust
    /// let (mut tx, rx) = laika::shotgun::channel();
    ///
    /// // Send a value
    /// tx.send(12);
    /// ```
    pub fn send(self, value: T) {
        self.inner.send(value);
    }
}

/// Inner receiver of a [`channel`]
#[derive(Clone, Debug)]
struct _Receiver<T>
where
    T: Clone,
{
    /// Value that was sent by [`_Sender`]
    value: Option<T>,
    /// Wakers that will be woken up when value is sent by [`_Sender`]
    wakers: Vec<Waker>,
}

/// Inner sender of a [`channel`]
#[derive(Clone, Debug)]
struct _Sender<T>
where
    T: Clone,
{
    /// [`_Receiver`] instance that will receive the value and is referecend by
    /// all [`Receiver`]s.
    receiver: Option<Arc<Mutex<_Receiver<T>>>>,
}

impl<T> _Receiver<T>
where
    T: Clone,
{
    /// Clones the value (if it has been given by [`_Sender`]) and returns clone
    /// of it.
    fn try_recv(&self) -> Option<T> {
        self.value.clone()
    }

    /// Sets the value to be received by all [`Receiver`]s from [`_Sender`].
    fn set(&mut self, value: T) {
        self.value = Some(value);

        for waker in self.wakers.clone() {
            waker.wake();
        }
    }
}

/// Implement [`Future`] for [`Receiver`] to be able to use it in async
/// functions.
impl<T> Future for Receiver<T>
where
    T: Clone,
{
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut inner = self.inner.lock().expect("Mutex is poisoned");

        if let Some(value) = &inner.value {
            Poll::Ready(value.clone())
        } else {
            if inner.wakers.iter().all(|w| !w.will_wake(cx.waker())) {
                inner.wakers.push(cx.waker().clone());
            }
            Poll::Pending
        }
    }
}

impl<T> _Sender<T>
where
    T: Clone,
{
    /// Send a value to all [`Receiver`]s.
    ///
    /// # Panics
    ///
    /// Panics if mutex is poisened due to another thread panicking while using
    /// referenced receiver too.
    fn send(self, value: T) {
        if let Some(recv) = self.receiver.as_ref() {
            recv.lock().expect("Mutex is poisoned").set(value);
        }
    }
}

/// Creates a one-shot, single producer multiple consumer channel that can be
/// used to send one value to multiple receivers.
///
/// # Examples
///
/// ```rust
/// let (mut tx, rx) = laika::shotgun::channel::<u8>();
/// // do something with tx and rx
/// ```
pub fn channel<T>() -> (Sender<T>, Receiver<T>)
where
    T: Clone,
{
    let mut sender = Sender {
        inner: _Sender { receiver: None },
    };

    let receiver_ref = Arc::new(Mutex::new(_Receiver {
        value: None,
        wakers: Vec::new(),
    }));

    let receiver = Receiver {
        inner: receiver_ref.clone(),
    };

    sender.inner.receiver = Some(receiver_ref);

    (sender, receiver)
}

#[cfg(test)]
mod test {
    use super::*;
    use tokio::task::JoinSet;

    #[test]
    fn test_basic() {
        let (tx, rx) = channel();

        assert_eq!(rx.try_recv(), None);
        assert_eq!(rx.try_recv(), None);

        tx.send(());

        assert_eq!(rx.try_recv(), Some(()));
        assert_eq!(rx.try_recv(), Some(()));
    }

    #[test]
    fn test_work_without_receiver() {
        let (tx, rx) = channel();
        assert_eq!(rx.try_recv(), None);

        drop(rx);

        tx.send(());
    }

    #[test]
    fn test_work_without_sender() {
        let (tx, rx) = channel::<()>();

        assert_eq!(rx.try_recv(), None);

        drop(tx);

        assert_eq!(rx.try_recv(), None);
    }

    #[test]
    fn test_work_with_multiple_receivers() {
        let (tx, rx) = channel();

        let rx1 = rx.clone();
        let rx2 = rx.clone();

        assert_eq!(rx.try_recv(), None);
        assert_eq!(rx1.try_recv(), None);
        assert_eq!(rx2.try_recv(), None);

        tx.send(1337);

        assert_eq!(rx.try_recv(), Some(1337));
        assert_eq!(rx1.try_recv(), Some(1337));
        assert_eq!(rx2.try_recv(), Some(1337));
    }

    #[test]
    fn test_works_in_threads() {
        use std::thread;
        use std::time;

        let (tx, rx) = channel();

        let rx1 = rx.clone();

        let thread1 = thread::spawn(move || loop {
            if rx1.try_recv().is_some() {
                return 1;
            }

            thread::sleep(time::Duration::from_secs(1));
        });

        let rx2 = rx.clone();
        let thread2 = thread::spawn(move || loop {
            if rx2.try_recv().is_some() {
                return 2;
            }

            thread::sleep(time::Duration::from_secs(1));
        });

        thread::sleep(time::Duration::from_secs(2));

        tx.send(());

        assert!(thread1.join().is_ok_and(|v| v == 1));
        assert!(thread2.join().is_ok_and(|v| v == 2));
    }

    #[tokio::test]
    async fn test_recv() {
        use std::thread;
        use std::time;

        let (tx, rx) = channel();

        let mut join_set = JoinSet::new();
        let rx1 = rx.clone();
        join_set.spawn(async move {
            rx1.await;
            1
        });

        let rx2 = rx.clone();
        join_set.spawn(async move {
            rx2.recv().await; // Explicit call to recv
            2
        });

        thread::sleep(time::Duration::from_secs(2));

        tx.send(());

        let rx3 = rx.clone();
        let fun3 = async move {
            rx3.await;
            3
        };

        let result = join_set.join_all().await;

        assert_eq!(result[0], 1);
        assert_eq!(result[1], 2);
        assert_eq!(fun3.await, 3);
    }
}
