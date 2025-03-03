//! actor model
use std::marker::PhantomData;
use tokio::sync::mpsc;


pub struct Actor<M> {
    _msg: PhantomData<M>,
}

impl<M> Actor<M> {
    pub fn new<F,Fut>(action: F) -> ActorHandle<M>
    where
        M: Send,
        F: Fn(mpsc::Receiver<M>) -> Fut + Send,
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        let (send,recv) = mpsc::channel::<M>(12);
        tokio::spawn(action(recv));
        ActorHandle {
            send,
            _msg: PhantomData,
        }
    }
}

pub struct ActorHandle<M> {
    send: mpsc::Sender<M>,
    _msg: PhantomData<M>
}

impl<M> ActorHandle<M> {
    pub fn send(&self, value: M) -> impl Future<Output = Result<(), mpsc::error::SendError<M>>> {
        self.send.send(value)
    }

    pub fn try_send(&self, value: M) -> Result<(), mpsc::error::TrySendError<M>> {
        self.send.try_send(value)
    }

    pub fn blocking_send(&self, value: M) -> Result<(), mpsc::error::SendError<M>> {
        self.send.blocking_send(value)
    }
}

