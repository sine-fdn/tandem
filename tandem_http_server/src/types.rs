use std::collections::HashMap;

use rocket::serde::{Deserialize, Serialize};
use tandem::{states::Msg, Circuit};

use crate::msg_queue::MessageId;

pub type EngineId = String;
pub type MessageLog = Vec<(Msg, MessageId)>;

/// Custom logic to choose a server's circuit and input.
pub type HandleMpcRequestFn = Box<dyn Fn(MpcRequest) -> Result<MpcSession, String> + Send + Sync>;

/// Session information used by the server to start executing the MPC protocol.
#[derive(Debug, Clone)]
pub struct MpcSession {
    /// The circuit to execute using MPC.
    pub circuit: Circuit,
    /// The server's input, kept hidden from the client.
    pub input_from_server: Vec<bool>,
    /// Optional headers for the client, which the client should set for every request.
    ///
    /// This can be used to set headers which allow the server(s) to re-identify clients after the
    /// initial request and to e.g. ensure that every request during the MPC execution is routed to
    /// the same server instance.
    pub request_headers: HashMap<String, String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct EngineMessages {
    log: MessageLog,
    last_durably_received_client_event_offset: MessageId,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(crate = "rocket::serde")]
pub struct EngineCreationResult {
    pub engine_id: String,
    pub request_headers: HashMap<String, String>,
}

/// A request by a client to start a Multi-Party Computation.
pub struct MpcRequest {
    /// Plaintext freely chosen by the client to influence the server's choice of its input.
    pub plaintext_metadata: String,
    /// The Garble program to execute.
    pub program: String,
    /// The name of the function in the Garble program to execute using MPC.
    pub function: String,
}
