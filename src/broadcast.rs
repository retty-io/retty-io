//! Thread safe communication broadcast channel implementing `Evented`
use mio::{Evented, Poll, PollOpt, Ready, Registration, SetReadiness, Token};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{
    io,
    sync::{Arc, Mutex},
};

/// Create a pair of the [`Sender`] and the [`Receiver`].
///
/// The [`Receiver`] implements the [`mio::event::Evented`] so that it can be registered
/// with the [`mio::Poll`], while the [`Sender`] doesn't.
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
/// It implements the [`mio::event::Evented`] so that it can be registered with the [`mio::Poll`].
/// It ignores the [`mio::Ready`] and always cause readable events.
pub struct Receiver<T> {
    waker: Arc<Mutex<HashMap<usize, (Registration, SetReadiness)>>>,
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

impl<T> Evented for Receiver<T> {
    fn register(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        let (registration, set_readiness) = Registration::new2();
        poll.register(&registration, token, interest, opts)?;

        let mut waker_map = self.waker.lock().unwrap();
        if let std::collections::hash_map::Entry::Vacant(e) = waker_map.entry(self.id) {
            e.insert((registration, set_readiness));
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "receiver already registered",
            ))
        }
    }

    fn reregister(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        let waker_map = self.waker.lock().unwrap();
        if let Some((registration, _set_readiness)) = waker_map.get(&self.id) {
            poll.reregister(registration, token, interest, opts)
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "receiver not registered",
            ))
        }
    }

    fn deregister(&self, poll: &Poll) -> io::Result<()> {
        let waker_map = self.waker.lock().unwrap();
        if let Some((registration, _set_readiness)) = waker_map.get(&self.id) {
            poll.deregister(registration)
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "receiver not registered",
            ))
        }
    }
}

/// A wrapper of the [`crossbeam::channel::Sender`].
pub struct Sender<T> {
    waker: Arc<Mutex<HashMap<usize, (Registration, SetReadiness)>>>,
    tx: crossbeam::channel::Sender<T>,
}

impl<T> Sender<T> {
    /// Try to send a value. It works just like [`crossbeam::channel::Sender::send`].
    /// After sending it, it's waking up the [`mio::Poll`].
    ///
    /// Note that it does not return any I/O error even if it occurs
    /// when waking up the [`mio::Poll`].
    pub fn send(&self, t: T) -> Result<(), crossbeam::channel::SendError<T>> {
        self.tx.send(t)?;

        let mut waker_map = self.waker.lock().unwrap();
        for (_registration, set_readiness) in waker_map.values_mut() {
            let _ = set_readiness.set_readiness(Ready::readable());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam::sync::WaitGroup;
    use mio::Events;

    #[test]
    fn test_channel() -> Result<(), Box<dyn std::error::Error>> {
        let (tx, rx) = channel();

        let handler = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(1000));

            let _ = tx.send("Hello world!");
        });

        let wg = WaitGroup::new();
        for i in 0..2 {
            let rx = rx.clone();
            let wg = wg.clone();
            std::thread::spawn(move || {
                const CHANNEL: Token = Token(0);

                let poll = Poll::new()?;
                let mut events = Events::with_capacity(2);
                poll.register(&rx, CHANNEL, Ready::readable(), PollOpt::edge())?;

                poll.poll(&mut events, None)?;
                for event in events.iter() {
                    match event.token() {
                        CHANNEL => {
                            println!("receive CHANNEL {}", i);
                            let _ = rx.try_recv();
                            drop(wg);
                            return Ok(());
                        }
                        _ => unreachable!(),
                    }
                }

                Ok::<(), std::io::Error>(())
            });
        }

        wg.wait();
        let _ = handler.join();

        Ok(())
    }
}
