//! The RCON session task: one UDP socket, one login, one command in flight at a time.
//!
//! A command packet is sent again under the same sequence number until its response is
//! complete or [`RconTimings::transmission_attempts`] transmissions went unanswered; the session
//! then counts as lost, and the command is sent once more after a new login. A session lost
//! while idle logs in again on the next command. While logged in and idle, the task
//! acknowledges server messages and sends an empty command packet whenever
//! [`RconTimings::keep_alive_interval`] passes without a command packet.

use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};

use tokio::net::UdpSocket;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Instant, sleep_until, timeout_at};
use tracing::{debug, info, trace, warn};

use super::command_sequence::CommandSequence;
use super::packet_codec::{
    ServerPacket, command_packet, decode_server_packet, login_packet,
    server_message_acknowledgement,
};
use super::rcon_client::{RconError, RconSettings, RconTimings};
use super::reforger_commands::SESSION_LOGOUT_COMMAND;
use super::response_assembly::ResponseAssembly;
use super::server_message_window::ServerMessageWindow;
use crate::secret_text::SecretText;

/// Large enough for any UDP datagram.
const RECEIVE_BUFFER_BYTES: usize = 65_536;

pub(super) enum SessionRequest {
    Execute {
        command: String,
        reply: oneshot::Sender<Result<String, RconError>>,
    },
    LogOut {
        reply: oneshot::Sender<()>,
    },
}

pub(super) struct RconSession {
    socket: UdpSocket,
    server: SocketAddr,
    password: SecretText,
    timings: RconTimings,
    logged_in: bool,
    sequence: CommandSequence,
    /// When the last command packet left: the server's 45 s timeout counts from it.
    last_command_sent: Instant,
    server_messages: ServerMessageWindow,
    buffer: Vec<u8>,
}

impl RconSession {
    /// Binds an ephemeral local port of the server's address family and fixes the server as the
    /// only peer, so datagrams from any other address never reach the session.
    pub(super) async fn open(settings: RconSettings) -> std::io::Result<Self> {
        let local: SocketAddr = if settings.server.is_ipv4() {
            (Ipv4Addr::UNSPECIFIED, 0).into()
        } else {
            (Ipv6Addr::UNSPECIFIED, 0).into()
        };
        let socket = UdpSocket::bind(local).await?;
        socket.connect(settings.server).await?;
        Ok(Self {
            socket,
            server: settings.server,
            password: settings.password,
            timings: settings.timings,
            logged_in: false,
            sequence: CommandSequence::default(),
            last_command_sent: Instant::now(),
            server_messages: ServerMessageWindow::default(),
            buffer: vec![0; RECEIVE_BUFFER_BYTES],
        })
    }

    pub(super) async fn run(mut self, mut requests: mpsc::Receiver<SessionRequest>) {
        loop {
            let keep_alive_due = self.last_command_sent + self.timings.keep_alive_interval;
            tokio::select! {
                request = requests.recv() => match request {
                    Some(SessionRequest::Execute { command, reply }) => {
                        let result = self.execute(&command).await;
                        // A caller that stopped waiting leaves nobody to answer.
                        let _ = reply.send(result);
                    }
                    Some(SessionRequest::LogOut { reply }) => {
                        self.log_out().await;
                        let _ = reply.send(());
                        return;
                    }
                    None => {
                        self.log_out().await;
                        return;
                    }
                },
                received = self.socket.recv(&mut self.buffer), if self.logged_in => {
                    if let Ok(length) = received
                        && let Ok(packet) = decode_server_packet(&self.buffer[..length])
                    {
                        self.absorb(packet).await;
                    }
                }
                () = sleep_until(keep_alive_due), if self.logged_in => self.keep_alive().await,
            }
        }
    }

    async fn execute(&mut self, command: &str) -> Result<String, RconError> {
        if !self.logged_in {
            self.log_in().await?;
        }
        debug!(command, "sending RCON command");
        if let Some(response) = self.exchange(command.as_bytes()).await {
            return Ok(String::from_utf8_lossy(&response).into_owned());
        }
        self.logged_in = false;
        warn!(server = %self.server, "RCON command unanswered; logging in again to send it once more");
        self.log_in().await?;
        match self.exchange(command.as_bytes()).await {
            Some(response) => Ok(String::from_utf8_lossy(&response).into_owned()),
            None => {
                self.logged_in = false;
                Err(RconError::NoResponse)
            }
        }
    }

    async fn log_in(&mut self) -> Result<(), RconError> {
        let datagram = login_packet(self.password.expose().as_bytes());
        for attempt in 1..=self.timings.transmission_attempts {
            self.send(&datagram).await;
            let deadline = Instant::now() + self.timings.response_timeout;
            while let Some(packet) = self.receive_until(deadline).await {
                match packet {
                    ServerPacket::LoginResponse { accepted: true } => {
                        self.logged_in = true;
                        self.last_command_sent = Instant::now();
                        self.server_messages.clear();
                        info!(server = %self.server, attempt, "RCON login accepted");
                        return Ok(());
                    }
                    ServerPacket::LoginResponse { accepted: false } => {
                        warn!(server = %self.server, "RCON login refused");
                        return Err(RconError::LoginRejected);
                    }
                    other => self.absorb(other).await,
                }
            }
        }
        warn!(server = %self.server, attempts = self.timings.transmission_attempts, "RCON login unanswered");
        Err(RconError::LoginUnanswered(self.server))
    }

    /// Sends one command and waits for its complete response, retransmitting it under the same
    /// sequence number while it goes unanswered. `None` when every transmission went unanswered.
    async fn exchange(&mut self, command: &[u8]) -> Option<Vec<u8>> {
        let sequence = self.sequence.allocate();
        let datagram = command_packet(sequence, command);
        let mut assembly = ResponseAssembly::default();
        for attempt in 1..=self.timings.transmission_attempts {
            if attempt > 1 {
                debug!(
                    sequence,
                    attempt, "retransmitting an unanswered RCON command"
                );
            }
            self.send(&datagram).await;
            self.last_command_sent = Instant::now();
            let deadline = Instant::now() + self.timings.response_timeout;
            while let Some(packet) = self.receive_until(deadline).await {
                match packet {
                    ServerPacket::CommandResponse {
                        sequence: answered,
                        body,
                    } if answered == sequence => {
                        if let Some(response) = assembly.accept(body) {
                            return Some(response);
                        }
                    }
                    other => self.absorb(other).await,
                }
            }
        }
        None
    }

    async fn keep_alive(&mut self) {
        if self.exchange(&[]).await.is_none() {
            self.logged_in = false;
            warn!(server = %self.server, "RCON keep-alive unanswered; the next command logs in again");
        }
    }

    async fn log_out(&mut self) {
        if !self.logged_in {
            return;
        }
        let sequence = self.sequence.allocate();
        self.send(&command_packet(sequence, SESSION_LOGOUT_COMMAND.as_bytes()))
            .await;
        self.logged_in = false;
        info!(server = %self.server, "RCON session logged out");
    }

    /// The next valid packet before `deadline`. Invalid datagrams are dropped; a socket error
    /// (such as the ICMP answer of a closed port) ends the wait early.
    async fn receive_until(&mut self, deadline: Instant) -> Option<ServerPacket> {
        loop {
            match timeout_at(deadline, self.socket.recv(&mut self.buffer)).await {
                Err(_elapsed) => return None,
                Ok(Err(error)) => {
                    debug!(%error, "RCON receive failed");
                    return None;
                }
                Ok(Ok(length)) => match decode_server_packet(&self.buffer[..length]) {
                    Ok(packet) => return Some(packet),
                    Err(error) => debug!(%error, "dropped an invalid RCON datagram"),
                },
            }
        }
    }

    /// Handles a packet no request waits for: server messages are acknowledged, anything else
    /// (such as a late answer to an earlier command) is ignored.
    async fn absorb(&mut self, packet: ServerPacket) {
        match packet {
            ServerPacket::ServerMessage { sequence, message } => {
                self.send(&server_message_acknowledgement(sequence)).await;
                if self.server_messages.first_delivery(sequence) {
                    debug!(sequence, message = %String::from_utf8_lossy(&message), "RCON server message");
                }
            }
            other => trace!(?other, "ignored an RCON packet nothing waits for"),
        }
    }

    async fn send(&self, datagram: &[u8]) {
        if let Err(error) = self.socket.send(datagram).await {
            debug!(%error, "RCON datagram not sent");
        }
    }
}
