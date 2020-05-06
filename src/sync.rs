#[cfg(not(feature = "async_std_unstable"))]
mod imp {
    use std::sync::mpsc;

    pub struct Sender<T>(mpsc::SyncSender<T>);

    impl<T> Sender<T> {
        #[must_use]
        pub fn send_blocking(&self, item: T) -> bool {
            self.0.send(item).is_ok()
        }
    }

    pub type Receiver<T> = mpsc::Receiver<T>;

    pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
        let (s, r) = mpsc::sync_channel(0);
        (Sender(s), r)
    }
}

#[cfg(feature = "async_std_unstable")]
mod imp {
    use async_std::sync;

    pub struct Sender<T>(sync::Sender<T>);

    impl<T> Sender<T> {
        #[must_use]
        pub fn send_blocking(&self, item: T) -> bool {
            async_std::task::block_on(async {
                self.0.send(item).await;
                true
            })
        }
    }

    pub type Receiver<T> = sync::Receiver<T>;

    pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
        let (s, r) = sync::channel(1);
        (Sender(s), r)
    }
}

pub use imp::*;