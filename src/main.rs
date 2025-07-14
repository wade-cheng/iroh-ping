use std::str::FromStr;

use anyhow::{Error, Result};
use iroh::Watcher;
use iroh::{protocol::Router, Endpoint, NodeAddr};
use iroh_base::ticket::NodeTicket;
use iroh_ping::{Ping, ALPN as PingALPN};

/// Return whether our process is a client.
///
/// If not, we must be the server.
///
/// Decides based on command line arguments. If no arguments
/// are supplied, we assume the user wants the process to be
/// a server.
fn is_client() -> Result<bool> {
    let mut is_client = false;
    let mut is_server = false;
    for arg in std::env::args() {
        if arg == "client" {
            is_client = true;
        }
        if arg == "server" {
            is_server = true;
        }
    }
    if is_client && is_server {
        Err(Error::msg(
            "This process cannot be both the client and the server.",
        ))
    } else {
        Ok(is_client)
    }
}

/// Gets the first ticket string from the command line arguments.
fn ticket() -> Result<String> {
    for arg in std::env::args() {
        if let Some(("--ticket", t)) = arg.split_once("=") {
            return Ok(t.to_string());
        }
    }

    Err(Error::msg(
        "No ticket provided. Clients must provide a ticket to find a server.",
    ))
}

#[tokio::main]
async fn main() -> Result<()> {
    if is_client()? {
        // create a send side & send a ping
        let send_ep = Endpoint::builder().discovery_n0().bind().await?;
        let send_pinger = Ping::new();
        let rtt = send_pinger
            .ping(&send_ep, NodeAddr::from(NodeTicket::from_str(&ticket()?)?))
            .await?;
        println!("ping took: {:?} to complete", rtt);
    } else {
        // create the receive side
        let recv_ep = Endpoint::builder().discovery_n0().bind().await?;
        let recv_router = Router::builder(recv_ep)
            .accept(PingALPN, Ping::new())
            .spawn();
        let addr = recv_router.endpoint().node_addr().initialized().await?;

        println!(
            "Connect to this server with:\n\
            cargo run client --ticket={}\n\
            \n\
            ctrl-c to quit.",
            NodeTicket::new(addr)
        );

        loop {}
    }

    Ok(())
}
