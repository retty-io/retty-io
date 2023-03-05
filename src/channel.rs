use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{
    io,
    sync::{Arc, Mutex},
};

use mio::{event, Token, Waker};

/// Create a pair of the [`Sender`] and the [`Receiver`].
///
/// The [`Receiver`] implements the [`event::Source`] so that it can be registered
/// with the [`mio::poll::Poll`], while the [`Sender`] doesn't.
pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let (tx, rx) = crossbeam::channel::unbounded();

    let waker = Arc::new(Mutex::new(HashMap::new()));

    (
        Sender {
            waker: waker.clone(),
            tx,
        },
        Receiver {
            waker,
            rx,
            id: 0,
            next_id: Arc::new(AtomicUsize::new(1)),
        },
    )
}

/// A wrapper of the [`crossbeam::channel::Receiver`].
///
/// It implements the [`event::Source`] so that it can be registered with the [`mio::poll::Poll`].
/// It ignores the [`mio::Interest`] and always cause readable events.
pub struct Receiver<T> {
    waker: Arc<Mutex<HashMap<usize, Option<Waker>>>>,
    rx: crossbeam::channel::Receiver<T>,
    id: usize,
    next_id: Arc<AtomicUsize>,
}

impl<T> Clone for Receiver<T> {
    fn clone(&self) -> Self {
        let next_id = self.next_id.clone();
        let id = next_id.fetch_add(1, Ordering::Relaxed);
        Self {
            waker: self.waker.clone(),
            rx: self.rx.clone(),
            id,
            next_id,
        }
    }
}

impl<T> Receiver<T> {
    /// Try to receive a value. It works just like [`crossbeam::channel::Receiver::try_recv`].
    pub fn try_recv(&self) -> Result<T, crossbeam::channel::TryRecvError> {
        self.rx.try_recv()
    }
}

impl<T> event::Source for Receiver<T> {
    fn register(
        &mut self,
        registry: &mio::Registry,
        token: Token,
        _: mio::Interest,
    ) -> io::Result<()> {
        let mut waker_map = self.waker.lock().unwrap();
        if let Some(waker) = waker_map.get_mut(&self.id) {
            if waker.is_none() {
                *waker = Some(Waker::new(registry, token)?);
            }
        } else {
            waker_map.insert(self.id, Some(Waker::new(registry, token)?));
        }

        Ok(())
    }

    fn reregister(
        &mut self,
        registry: &mio::Registry,
        token: Token,
        _: mio::Interest,
    ) -> io::Result<()> {
        let mut waker_map = self.waker.lock().unwrap();
        if let Some(waker) = waker_map.get_mut(&self.id) {
            *waker = Some(Waker::new(registry, token)?);
        } else {
            waker_map.insert(self.id, Some(Waker::new(registry, token)?));
        }

        Ok(())
    }

    fn deregister(&mut self, _: &mio::Registry) -> io::Result<()> {
        let mut waker_map = self.waker.lock().unwrap();
        if let Some(waker) = waker_map.get_mut(&self.id) {
            *waker = None;
        }

        Ok(())
    }
}

/// A wrapper of the [`crossbeam::channel::Sender`].
pub struct Sender<T> {
    waker: Arc<Mutex<HashMap<usize, Option<Waker>>>>,
    tx: crossbeam::channel::Sender<T>,
}

impl<T> Sender<T> {
    /// Try to send a value. It works just like [`crossbeam::channel::Sender::send`].
    /// After sending it, it's waking up the [`mio::poll::Poll`].
    ///
    /// Note that it does not return any I/O error even if it occurs
    /// when waking up the [`mio::poll::Poll`].
    pub fn send(&self, t: T) -> Result<(), crossbeam::channel::SendError<T>> {
        self.tx.send(t)?;

        let mut waker_map = self.waker.lock().unwrap();
        for waker in waker_map.values_mut().flatten() {
            let _ = waker.wake();
        }

        Ok(())
    }
}
