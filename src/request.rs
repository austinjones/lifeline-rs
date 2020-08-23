use futures::Future;
use tokio::sync::oneshot;

/// If you need synchronous RPC, you can use this utility
/// ```rust
/// use lifeline::request::Request;
///
/// struct Send(usize);
/// #[derive(Debug, Eq, PartialEq)]
/// struct Recv(usize);
///
/// lifeline::test::block_on(async {
///   let (request, mut recv) = Request::send(Send(42));
///
///   // send the request along a channel, and in a service:
///   request.reply(|send| async move { Recv(send.0) }).await;
///
///   let resp = recv.await;
///   assert_eq!(Ok(Recv(42)), resp);
/// })
/// ```
pub struct Request<Send, Recv> {
    send: Send,
    recv: oneshot::Sender<Recv>,
}

impl<Send, Recv> Request<Send, Recv> {
    /// Constructs a pair of Request, and Receiver for the response
    pub fn send(send: Send) -> (Self, oneshot::Receiver<Recv>) {
        let (tx, rx) = oneshot::channel();
        let request = Self { send, recv: tx };
        (request, rx)
    }

    /// Asynchronously replies to the given request, using the provided closure
    pub async fn reply<Fn, Fut>(self, respond: Fn) -> Result<(), Recv>
    where
        Fn: FnOnce(Send) -> Fut,
        Fut: Future<Output = Recv>,
    {
        let response = respond(self.send).await;
        self.recv.send(response)
    }
}
