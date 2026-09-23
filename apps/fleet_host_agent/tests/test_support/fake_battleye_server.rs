//! An in-process BattlEye RCon server for the transport tests. It is written from the protocol
//! specification independently of the agent's codec, answers like an Arma Reforger server
//! (`#players` gets a player listing, any other command an acknowledgement naming it, the empty
//! keep-alive an empty answer), ignores commands from clients that are not logged in or whose
//! login lapsed, and loses, corrupts, duplicates, reorders and fragments packets on request.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use tokio::net::UdpSocket;
use tokio::task::JoinHandle;

/// The server's answer to `#players`. The names are not ASCII, so a split into one-byte parts
/// cuts through characters.
pub const PLAYERS_RESPONSE: &str = "Players on server:\n\
    [Player#] ; [Player UID] ; [Player Name]\n\
    ------------------------------------------\n\
    0 ; 3d8f0e52-6c5a-4e0b-9a55-0c2b1d7e4f10 ; Rhodes\n\
    1 ; b6955d91-4749-4cdb-9a51-e69f630ec435 ; Jérôme Lefèvre";

const LOGIN: u8 = 0x00;
const COMMAND: u8 = 0x01;
const SERVER_MESSAGE: u8 = 0x02;

/// Faults applied to the next commands. Keep-alives (empty commands) never consume a fault.
#[derive(Debug, Clone, Default)]
pub struct Faults {
    /// Command packets discarded on arrival, unexecuted.
    pub discard_commands: usize,
    /// Commands executed whose responses are then discarded.
    pub discard_responses: usize,
    /// Responses sent with a damaged checksum.
    pub corrupt_responses: usize,
    /// Responses split into parts of this many bytes.
    pub fragment_bytes: Option<usize>,
    /// Parts sent last to first.
    pub reverse_fragments: bool,
    /// Every part sent twice.
    pub duplicate_fragments: bool,
}

/// One command packet from a logged-in client, retransmissions and keep-alives included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceivedCommand {
    pub sequence: u8,
    pub text: String,
}

struct ServerState {
    password: String,
    idle_timeout: Duration,
    /// Logged-in clients and when each last sent a command packet.
    sessions: HashMap<SocketAddr, Instant>,
    faults: Faults,
    login_attempts: usize,
    received_commands: Vec<ReceivedCommand>,
    executed_commands: Vec<String>,
    acknowledgements: Vec<u8>,
}

pub struct FakeBattlEyeServer {
    socket: Arc<UdpSocket>,
    state: Arc<Mutex<ServerState>>,
    task: JoinHandle<()>,
}

impl FakeBattlEyeServer {
    /// A server on a free loopback port that drops a login after `idle_timeout` without a
    /// command packet, as Arma Reforger does after 45 s.
    pub async fn start(password: &str, idle_timeout: Duration) -> Self {
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.expect("a free port"));
        let state = Arc::new(Mutex::new(ServerState {
            password: password.to_owned(),
            idle_timeout,
            sessions: HashMap::new(),
            faults: Faults::default(),
            login_attempts: 0,
            received_commands: Vec::new(),
            executed_commands: Vec::new(),
            acknowledgements: Vec::new(),
        }));
        let task = tokio::spawn(serve(Arc::clone(&socket), Arc::clone(&state)));
        Self {
            socket,
            state,
            task,
        }
    }

    pub fn address(&self) -> SocketAddr {
        self.socket.local_addr().expect("a bound socket")
    }

    pub fn set_faults(&self, faults: Faults) {
        self.state().faults = faults;
    }

    /// Forgets every login, as a restarted game server does.
    pub fn restart(&self) {
        self.state().sessions.clear();
    }

    pub fn login_attempts(&self) -> usize {
        self.state().login_attempts
    }

    pub fn received_commands(&self) -> Vec<ReceivedCommand> {
        self.state().received_commands.clone()
    }

    /// The sequence numbers of every transmission of `text`.
    pub fn transmissions_of(&self, text: &str) -> Vec<u8> {
        self.state()
            .received_commands
            .iter()
            .filter(|command| command.text == text)
            .map(|command| command.sequence)
            .collect()
    }

    pub fn executed_commands(&self) -> Vec<String> {
        self.state().executed_commands.clone()
    }

    pub fn acknowledgements(&self) -> Vec<u8> {
        self.state().acknowledgements.clone()
    }

    /// Sends a server message to every logged-in client.
    pub async fn send_server_message(&self, sequence: u8, text: &str) {
        let clients: Vec<SocketAddr> = self.state().sessions.keys().copied().collect();
        let mut payload = vec![sequence];
        payload.extend_from_slice(text.as_bytes());
        let datagram = server_datagram(SERVER_MESSAGE, &payload);
        for client in clients {
            self.socket
                .send_to(&datagram, client)
                .await
                .expect("a loopback send");
        }
    }

    fn state(&self) -> MutexGuard<'_, ServerState> {
        self.state.lock().expect("an unpoisoned state")
    }
}

impl Drop for FakeBattlEyeServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn serve(socket: Arc<UdpSocket>, state: Arc<Mutex<ServerState>>) {
    let mut buffer = vec![0; 65_536];
    loop {
        let Ok((length, client)) = socket.recv_from(&mut buffer).await else {
            continue;
        };
        let replies = state
            .lock()
            .expect("an unpoisoned state")
            .answer(&buffer[..length], client);
        for reply in replies {
            socket
                .send_to(&reply, client)
                .await
                .expect("a loopback send");
        }
    }
}

impl ServerState {
    fn answer(&mut self, datagram: &[u8], client: SocketAddr) -> Vec<Vec<u8>> {
        let Some((packet_type, payload)) = client_packet(datagram) else {
            return Vec::new();
        };
        match packet_type {
            LOGIN => {
                self.login_attempts += 1;
                let accepted = payload == self.password.as_bytes();
                if accepted {
                    self.sessions.insert(client, Instant::now());
                }
                vec![server_datagram(LOGIN, &[u8::from(accepted)])]
            }
            COMMAND => self.answer_command(payload, client),
            SERVER_MESSAGE => {
                if let [sequence] = payload {
                    self.acknowledgements.push(*sequence);
                }
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    fn answer_command(&mut self, payload: &[u8], client: SocketAddr) -> Vec<Vec<u8>> {
        let Some((&sequence, command)) = payload.split_first() else {
            return Vec::new();
        };
        let now = Instant::now();
        let logged_in = self
            .sessions
            .get(&client)
            .is_some_and(|last| now.duration_since(*last) <= self.idle_timeout);
        if !logged_in {
            self.sessions.remove(&client);
            return Vec::new();
        }
        self.sessions.insert(client, now);
        let text = String::from_utf8_lossy(command).into_owned();
        self.received_commands.push(ReceivedCommand {
            sequence,
            text: text.clone(),
        });
        if text.is_empty() {
            return self.response_datagrams(sequence, b"");
        }
        if take(&mut self.faults.discard_commands) {
            return Vec::new();
        }
        self.executed_commands.push(text.clone());
        if take(&mut self.faults.discard_responses) {
            return Vec::new();
        }
        let response = match text.as_str() {
            "#players" => PLAYERS_RESPONSE.to_owned(),
            other => format!("executed {other}"),
        };
        let mut datagrams = self.response_datagrams(sequence, response.as_bytes());
        if take(&mut self.faults.corrupt_responses) {
            for datagram in &mut datagrams {
                let last = datagram.len() - 1;
                datagram[last] ^= 0x5A;
            }
        }
        datagrams
    }

    fn response_datagrams(&self, sequence: u8, response: &[u8]) -> Vec<Vec<u8>> {
        let Some(part_bytes) = self
            .faults
            .fragment_bytes
            .filter(|part_bytes| response.len() > *part_bytes)
        else {
            let mut payload = vec![sequence];
            payload.extend_from_slice(response);
            return vec![server_datagram(COMMAND, &payload)];
        };
        let parts: Vec<&[u8]> = response.chunks(part_bytes).collect();
        let total = u8::try_from(parts.len()).expect("at most 255 parts");
        let mut datagrams: Vec<Vec<u8>> = parts
            .iter()
            .zip(0u8..)
            .map(|(part, index)| {
                let mut payload = vec![sequence, 0x00, total, index];
                payload.extend_from_slice(part);
                server_datagram(COMMAND, &payload)
            })
            .collect();
        if self.faults.reverse_fragments {
            datagrams.reverse();
        }
        if self.faults.duplicate_fragments {
            datagrams = datagrams
                .into_iter()
                .flat_map(|datagram| [datagram.clone(), datagram])
                .collect();
        }
        datagrams
    }
}

/// Consumes one unit of a fault counter.
fn take(counter: &mut usize) -> bool {
    let pending = *counter > 0;
    if pending {
        *counter -= 1;
    }
    pending
}

/// `'B' 'E' | CRC32 little-endian | 0xFF | type | payload`, the CRC32 covering 0xFF onward.
fn server_datagram(packet_type: u8, payload: &[u8]) -> Vec<u8> {
    let mut checked = vec![0xFF, packet_type];
    checked.extend_from_slice(payload);
    let mut datagram = b"BE".to_vec();
    datagram.extend_from_slice(&crc32fast::hash(&checked).to_le_bytes());
    datagram.extend_from_slice(&checked);
    datagram
}

/// The type and payload of a client datagram whose header and checksum are valid.
fn client_packet(datagram: &[u8]) -> Option<(u8, &[u8])> {
    let (header, checked) = datagram.split_at_checked(6)?;
    if &header[..2] != b"BE" || checked.first() != Some(&0xFF) {
        return None;
    }
    let declared = u32::from_le_bytes(header[2..6].try_into().ok()?);
    if crc32fast::hash(checked) != declared {
        return None;
    }
    let (&packet_type, payload) = checked[1..].split_first()?;
    Some((packet_type, payload))
}
