use bytes::{Bytes, BytesMut};
use std::{
    io,
    num::NonZeroUsize,
    pin::Pin,
    task::{Poll, ready},
};
use tcio::io::{AsyncIoRead, AsyncIoWrite};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

type HandleTx = UnboundedSender<HandleMessage>;
type HandleRx = UnboundedReceiver<HandleMessage>;

type TaskTx = UnboundedSender<TaskMessage>;
type TaskRx = UnboundedReceiver<TaskMessage>;

#[derive(Debug)]
pub struct IoHandle {
    status: Option<Operation>,
    tx: HandleTx,
    rx: TaskRx,
}

#[derive(Debug)]
enum Operation {
    Read,
    Sync,
}

enum HandleMessage {
    /// Read with or without max capacity.
    Read(Option<NonZeroUsize>),
    Write(Bytes),
    Sync,
}

impl IoHandle {
    pub fn new<IO>(io: IO) -> (Self, impl Future<Output = ()>)
    where
        IO: AsyncIoRead + AsyncIoWrite + Send + 'static,
    {
        let (tx_handle, rx_handle) = unbounded_channel();
        let (tx_task, rx_task) = unbounded_channel();

        (
            Self {
                status: None,
                tx: tx_handle,
                rx: rx_task,
            },
            IoTask::new(tx_task, rx_handle, io),
        )
    }

    pub fn try_poll_read(
        &mut self,
        cx: &mut std::task::Context,
    ) -> Poll<io::Result<Option<BytesMut>>> {
        match &mut self.status {
            set @ None => if self.tx.send(HandleMessage::Read(None)).is_ok() {
                *set = Some(Operation::Read);
            } else {
                return Poll::Ready(Ok(None));
            },
            Some(Operation::Read) => {},
            Some(Operation::Sync) => {
                return Poll::Ready(Err(io::Error::other("`IoHandle::poll_sync` is pending")));
            },
        }
        let result = match ready!(self.rx.poll_recv(cx)) {
            Some(TaskMessage::Data(data)) => Ok(Some(data)),
            Some(TaskMessage::Err(err)) => Err(err),
            Some(TaskMessage::Sync(_)) => {
                unreachable!("unexpected message from io task, current status is `HandleStatus::Read`")
            }
            None => Ok(None),
        };
        self.status = None;
        Poll::Ready(result)
    }

    pub fn try_poll_sync(&mut self, cx: &mut std::task::Context) -> Poll<io::Result<Option<()>>> {
        match &mut self.status {
            set @ None => if self.tx.send(HandleMessage::Sync).is_ok() {
                *set = Some(Operation::Sync);
            } else {
                return Poll::Ready(Ok(None));
            },
            Some(Operation::Sync) => {},
            Some(Operation::Read) => {
                return Poll::Ready(Err(io::Error::other("`IoHandle::poll_read` is pending")));
            },
        }
        let result = match ready!(self.rx.poll_recv(cx)) {
            Some(TaskMessage::Sync(Poll::Pending)) => return Poll::Pending,
            Some(TaskMessage::Sync(Poll::Ready(()))) => Ok(Some(())),
            Some(TaskMessage::Err(err)) => Err(err),
            Some(TaskMessage::Data(_)) => {
                unreachable!("unexpected message from io task, current status is `HandleStatus::Sync`")
            }
            None => Ok(None),
        };
        self.status = None;
        Poll::Ready(result)
    }

    #[inline]
    pub fn poll_read(&mut self, cx: &mut std::task::Context) -> Poll<io::Result<BytesMut>> {
        match ready!(self.try_poll_read(cx)) {
            Ok(Some(data)) => Poll::Ready(Ok(data)),
            Ok(None) => Poll::Ready(Err(io::ErrorKind::ConnectionAborted.into())),
            Err(err) => Poll::Ready(Err(err)),
        }
    }

    #[inline]
    pub fn read(&mut self) -> impl Future<Output = io::Result<BytesMut>> {
        std::future::poll_fn(|cx|self.poll_read(cx))
    }

    #[inline]
    pub fn write(&self, data: Bytes) -> io::Result<()> {
        self.tx.send(HandleMessage::Write(data)).map_err(|_|io::ErrorKind::ConnectionAborted.into())
    }
}

// ===== Task =====

/// Can only handle one operation at a time
struct IoTask<IO> {
    tx: TaskTx,
    rx: HandleRx,
    io: IO,
    buffer: BytesMut,
    task: Option<Task>,
    error: Option<io::Error>,
}

enum Task {
    /// None -> Read only once
    /// Some(0) -> Exhausted, reset required
    /// Some(n) -> Dont send the data yet, read more until `n`
    Read(Option<usize>),
    Write(Bytes),
}

enum TaskMessage {
    Sync(Poll<()>),
    Data(BytesMut),
    Err(io::Error),
}

impl<IO> Unpin for IoTask<IO> { }

impl<IO> IoTask<IO>
where
    IO: AsyncIoRead + AsyncIoWrite,
{
    fn new(tx: TaskTx, rx: HandleRx, io: IO) -> Self {
        Self {
            tx,
            rx,
            io,
            buffer: BytesMut::with_capacity(0x0400),
            task: None,
            error: None,
        }
    }

    fn send(&self, message: TaskMessage) {
        let _ = self.tx.send(message);
    }

    fn terminate(&mut self) -> io::Error {
        self.error = Some(io::ErrorKind::ConnectionAborted.into());
        io::ErrorKind::ConnectionAborted.into()
    }

    fn is_terminating(&mut self) -> bool {
        matches!(&self.error, Some(err) if err.kind() == io::ErrorKind::ConnectionAborted)
    }

    /// Check is current buffer is enough to send back to handle.
    fn handle_buffer_read(&mut self) {
        let Some(Task::Read(remaining)) = &mut self.task else {
            return;
        };

        match remaining {
            None => if !self.buffer.is_empty() {
                let data = self.buffer.split();
                self.send(TaskMessage::Data(data));
                self.task = None;
            },
            Some(0) => {
                self.send(TaskMessage::Err(io::ErrorKind::QuotaExceeded.into()));
            },
            Some(remaining) => if self.buffer.len() >= *remaining {
                let data = self.buffer.split_to(*remaining);
                *remaining = 0;
                self.send(TaskMessage::Data(data));
            },
        }
    }

    fn try_poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<()> {
        let me = self.as_mut().get_mut();

        me.poll_message(cx);
        me.poll_task(cx);

        Poll::Pending
    }

    fn poll_message(&mut self, cx: &mut std::task::Context) {
        if self.is_terminating() {
            return;
        }

        let msg = match self.rx.poll_recv(cx) {
            Poll::Ready(Some(msg)) => msg,
            Poll::Ready(None) => {
                self.terminate();
                return;
            }
            Poll::Pending => return,
        };

        match (msg, &mut self.task) {
            // operation request
            (HandleMessage::Read(cap), None) => self.task = Some(Task::Read(cap.map(NonZeroUsize::get))),
            (HandleMessage::Write(bytes), None) => self.task = Some(Task::Write(bytes)),

            // sync
            (HandleMessage::Sync, None) => self.send(TaskMessage::Sync(Poll::Ready(()))),
            (HandleMessage::Sync, Some(_)) => self.send(TaskMessage::Sync(Poll::Pending)),

            // busy
            (HandleMessage::Read(_) | HandleMessage::Write(_), Some(_)) => {
                self.send(TaskMessage::Err(io::ErrorKind::ResourceBusy.into()))
            }
        }

        self.handle_buffer_read();
        if self.task.is_none() {
            self.poll_message(cx);
        }
    }

    fn poll_task(&mut self, cx: &mut std::task::Context) {
        let Some(task) = &mut self.task else {
            return;
        };

        match task {
            Task::Read(None) if !self.buffer.is_empty() => {
                let data = self.buffer.split();
                self.send(TaskMessage::Data(data));
                self.task = None;
            },
            Task::Read(Some(0)) => {
                self.send(TaskMessage::Err(io::ErrorKind::QuotaExceeded.into()));
            },
            Task::Read(Some(remaining)) if self.buffer.len() >= *remaining => {
                let data = self.buffer.split_to(*remaining);
                *remaining = 0;
                self.send(TaskMessage::Data(data));
            },

            // io call
            Task::Read(remaining) => {
                if self.buffer.capacity() < 0x0100 {
                    self.buffer.reserve(0x0400 - self.buffer.len());
                }

                let Poll::Ready(result) = self.io.poll_read(&mut self.buffer, cx) else {
                    return;
                };

                let mut new_task = None;

                let msg = match (result, remaining) {
                    (Ok(0), _) => TaskMessage::Err(self.terminate()),
                    (Ok(read), Some(remaining)) => {
                        let data = match remaining.checked_sub(read) {
                            Some(0) => {
                                // exactly exhausted
                                self.buffer.split()
                            },
                            Some(new_remain) => {
                                // not yet exhausted
                                // new_task = Some(Task::Read(Some(new_remain)));
                                self.buffer.split()
                            },
                            None => {
                                // read too much
                                // self.buffer.split_to(read - remaining)
                                todo!()
                            },
                        };

                        TaskMessage::Data(data)
                    },
                    (Ok(_), None) => TaskMessage::Data(self.buffer.split()),
                    (Err(err), _) => TaskMessage::Err(err),
                };

                self.send(msg);
                self.task = new_task;
            }
            Task::Write(bytes) => {
                let Poll::Ready(result) = self.io.poll_write_all_buf(bytes, cx) else {
                    return;
                };
                if let Err(err) = result && !self.is_terminating() {
                    self.error = Some(err)
                }
                self.task = None;
            }
        }
    }
}

impl<IO> Future for IoTask<IO>
where
    IO: AsyncIoRead + AsyncIoWrite,
{
    type Output = ();

    #[inline]
    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Self::Output> {
        match self.as_mut().try_poll(cx) {
            Poll::Ready(()) => Poll::Ready(()),
            Poll::Pending => {
                if self.is_terminating() && self.task.is_none() {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            }
        }
    }
}

