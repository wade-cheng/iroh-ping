use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use iroh::{
    endpoint::Connection,
    protocol::{AcceptError, ProtocolHandler},
    Endpoint, NodeAddr,
};
use iroh_metrics::{Counter, MetricsGroup};

/// Each protocol is identified by its ALPN string.
///
/// The ALPN, or application-layer protocol negotiation, is exchanged in the connection handshake,
/// and the connection is aborted unless both nodes pass the same bytestring.
pub const ALPN: &[u8] = b"iroh/ping/0";

/// Ping is a struct that holds both the client ping method, and the endpoint
/// protocol implementation
#[derive(Debug, Clone)]
pub struct Ping {
    metrics: Arc<Metrics>,
}

impl Default for Ping {
    fn default() -> Self {
        Self::new()
    }
}

impl Ping {
    /// create a new Ping
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Metrics::default()),
        }
    }

    /// handle to ping metrics
    pub fn metrics(&self) -> &Arc<Metrics> {
        &self.metrics
    }

    /// send a ping on the provided endpoint to a given node address
    pub async fn ping(&self, endpoint: &Endpoint, addr: NodeAddr) -> anyhow::Result<Duration> {
        let start = Instant::now();
        // Open a connection to the accepting node
        let conn = endpoint.connect(addr, ALPN).await?;

        // Open a bidirectional QUIC stream
        let (mut send, mut recv) = conn.open_bi().await?;

        // Send some data to be pinged
        send.write_all(b"PING").await?;

        // Signal the end of data for this particular stream
        send.finish()?;

        // read the response, which must be PONG as bytes
        let response = recv.read_to_end(4).await?;
        assert_eq!(&response, b"PONG");

        // Explicitly close the whole connection.
        conn.close(0u32.into(), b"bye!");

        // The above call only queues a close message to be sent (see how it's not async!).
        // We need to actually call this to make sure this message is sent out.
        endpoint.close().await;

        // at this point we've successfully pinged, mark the metric
        self.metrics.pings_sent.inc();

        // If we don't call this, but continue using the endpoint, we then the queued
        // close call will eventually be picked up and sent.
        // But always try to wait for endpoint.close().await to go through before dropping
        // the endpoint to ensure any queued messages are sent through and connections are
        // closed gracefully.
        Ok(Duration::from_millis(
            Instant::now().duration_since(start).as_millis() as u64,
        ))
    }
}

impl ProtocolHandler for Ping {
    /// The `accept` method is called for each incoming connection for our ALPN.
    ///
    /// The returned future runs on a newly spawned tokio task, so it can run as long as
    /// the connection lasts.
    async fn accept(&self, connection: Connection) -> n0_snafu::Result<(), AcceptError> {
        let metrics = self.metrics.clone();

        // We can get the remote's node id from the connection.
        let node_id = connection.remote_node_id()?;
        println!("accepted connection from {node_id}");

        // Our protocol is a simple request-response protocol, so we expect the
        // connecting peer to open a single bi-directional stream.
        let (mut send, mut recv) = connection.accept_bi().await?;

        let req = recv.read_to_end(4).await.map_err(AcceptError::from_err)?;
        assert_eq!(&req, b"PING");

        // send back "PONG" bytes
        send.write_all(b"PONG")
            .await
            .map_err(AcceptError::from_err)?;

        // By calling `finish` on the send stream we signal that we will not send anything
        // further, which makes the receive stream on the other end terminate.
        send.finish()?;

        // Wait until the remote closes the connection, which it does once it
        // received the response.
        connection.closed().await;

        // increment count of pings we've received
        metrics.pings_recv.inc();

        Ok(())
    }
}

/// Enum of metrics for the module
#[derive(Debug, Default, MetricsGroup)]
#[metrics(name = "ping")]
pub struct Metrics {
    /// count of valid ping messages sent
    pub pings_sent: Counter,
    /// count of valid ping messages received
    pub pings_recv: Counter,
}

#[cfg(test)]
mod tests {
    use iroh::{protocol::Router, Endpoint, Watcher};

    use super::*;

    #[tokio::test]
    async fn test_ping() -> anyhow::Result<()> {
        let ep = Endpoint::builder().discovery_n0().bind().await?;
        let router = Router::builder(ep).accept(ALPN, Ping::new()).spawn();
        let addr = router.endpoint().node_addr().initialized().await?;

        let client = Endpoint::builder().discovery_n0().bind().await?;
        let ping_client = Ping::new();
        let res = ping_client.ping(&client, addr.clone()).await?;
        println!("ping response: {res:?}");

        Ok(())
    }
}
