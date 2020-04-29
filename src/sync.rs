use std::sync::mpsc;


pub type Sender<T> = mpsc::SyncSender<T>;
pub type Receiver<T> = mpsc::Receiver<T>;

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    mpsc::sync_channel(0)
}
