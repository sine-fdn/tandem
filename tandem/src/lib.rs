//! Secure Multi-Party Computation (SMPC) using Garbled Circuits for 2 parties.
//!
//! This crate can be used to securely compute functions specified as boolean circuits (using AND,
//! XOR and NOT gates) without revealing the inputs of either party to the other party.
//!
//! The core protocol implemented in this crate is
//! [WRK17](https://acmccs.github.io/papers/p21-wangA.pdf), with an OT extension and optimizations
//! based on [ALSZ13](https://eprint.iacr.org/2013/552.pdf) and a base OT implementation based on
//! [ABKLX21](https://eprint.iacr.org/2021/1218.pdf).
//!
//! Communication channels are deliberately _not_ part of this crate. The sending and receiving of
//! messages needs to be handled by the user of this crate, which allows the MPC protocol to be used
//! both in sync and async environments.
//!
//! # Examples
//!
//! ```
//! use tandem::{
//!     states::{Contributor, Evaluator, Msg},
//!     Circuit, Error, Gate,
//! };
//! use rand::SeedableRng;
//! use rand_chacha::ChaCha20Rng;
//! use std::{
//!     sync::{mpsc::channel, Arc},
//!     thread,
//! };
//!
//! fn main() -> Result<(), Error> {
//!     // Create a simple circuit with 2 input bits, calculate their AND and output it:
//!     let simple_circuit = Circuit::new(
//!         vec![Gate::InContrib, Gate::InEval, Gate::And(0, 1)],
//!         vec![2],
//!     );
//!
//!     let (to_eval, from_contrib) = channel::<Msg>();
//!     let (to_contrib, from_eval) = channel::<Msg>();
//!
//!     // Spawn a contributor as a new thread, with the evaluator remaining on the main thread:
//!     let circuit_for_contrib_thread = simple_circuit.clone();
//!     thread::spawn(move || -> Result<(), Error> {
//!         let contributor_input = vec![true];
//!
//!         // Create a contributor and the initial message for the evaluator:
//!         let (mut contributor, msg) = Contributor::new(
//!             &circuit_for_contrib_thread,
//!             contributor_input,
//!             ChaCha20Rng::from_entropy(),
//!         )?;
//!
//!         // Send initial message to start MPC protocol:
//!         to_eval.send(msg).unwrap();
//!
//!         for _ in 0..contributor.steps() {
//!             let msg = from_eval.recv().expect("failed to get msg");
//!
//!             // Based on the message from the evaluator, the contributor transitions to a new state:
//!             let (next_state, reply) = contributor.run(&msg)?;
//!             to_eval.send(reply).expect("failed to send reply");
//!             contributor = next_state;
//!         }
//!         Ok(())
//!     });
//!
//!     let evaluator_input = vec![false];
//!
//!     let mut evaluator = Evaluator::new(
//!         &simple_circuit,
//!         evaluator_input,
//!         ChaCha20Rng::from_entropy(),
//!     )?;
//!
//!     for _ in 0..evaluator.steps() {
//!         let msg = from_contrib.recv().expect("failed to get msg");
//!
//!         // Based on the message from the contributor, the evaluator transitions to a new state:
//!         let (next_state, reply) = evaluator.run(&msg)?;
//!         to_contrib.send(reply).expect("failed to send reply");
//!         evaluator = next_state
//!     }
//!
//!     // The final message from the contributor allows the evaluator to decrypt the output:
//!     let final_msg = from_contrib.recv().expect("failed to get final msg");
//!     let output = evaluator.output(&final_msg)?;
//!     assert_eq!(output, vec![false]);
//!
//!     Ok(())
//! }
//! ```

#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

mod circuit;
mod hash;
mod leakyand;
mod leakydelta_ot;
mod ot_base;
mod protocol;
mod simulator;
pub mod states;
mod types;

pub use circuit::*;
pub use simulator::*;

/// Errors occurring during the validation or the execution of the MPC protocol.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// A different message was expected from the other party at this point in the protocol.
    UnexpectedMessageType,
    /// The AND shares received did not match the number of gates.
    InsufficientAndShares,
    /// The garbled table share does not belong to an AND gate.
    UnexpectedGarbledTableShare,
    /// Not enough input bits were provided as user input.
    InsufficientInput,
    /// A MAC checking error occured, due to an accidental or deliberate data corruption.
    MacError,
    /// The Leaky Authenticated AND Triples did not pass the equality check.
    LeakyAndNotEqual,
    /// The provided circuit contains invalid gate connections.
    InvalidCircuit,
    /// The provided circuit has too many gates to be processed.
    MaxCircuitSizeExceeded,
    /// The provided byte buffer could not be deserialized into an OT init message.
    OtInitDeserializationError,
    /// The provided byte buffer could not be deserialized into an OT block message.
    OtBlockDeserializationError,
    /// The provided byte buffer could not be deserialized into the expected type.
    BincodeError,
    /// The protocol has already ended, no further messages can be processed.
    ProtocolEnded,
    /// The protocol is still in progress and does not yet have any output.
    ProtocolStillInProgress,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::UnexpectedMessageType => f.write_str("Unexpected message kind"),
            Error::InsufficientAndShares => {
                f.write_str("Insufficient number of AND shares received from upstream")
            }
            Error::UnexpectedGarbledTableShare => {
                f.write_str("Received a table share for an unsupported gate")
            }
            Error::InsufficientInput => f.write_str("Not enough or too many input bits provided"),
            Error::MacError => f.write_str("At least 1 MAC check failed"),
            Error::LeakyAndNotEqual => {
                f.write_str("The equality check of the leaky AND step failed")
            }
            Error::InvalidCircuit => {
                f.write_str("The provided circuit is invalid and cannot be executed")
            }
            Error::MaxCircuitSizeExceeded => f.write_str(
                "The number of gates in the circuit exceed the maximum that can be processed",
            ),
            Error::OtInitDeserializationError => f.write_str(
                "The message buffer could not be deserialized into a proper OT init message",
            ),
            Error::OtBlockDeserializationError => f.write_str(
                "The message buffer could not be deserialized into a proper OT block message",
            ),
            Error::BincodeError => {
                f.write_str("The message could not be serialized to / deserialized from bincode")
            }
            Error::ProtocolEnded => {
                f.write_str("The protocol has already ended, no further messages can be processed.")
            }
            Error::ProtocolStillInProgress => {
                f.write_str("The protocol is still in progress and does not yet have any output.")
            }
        }
    }
}

impl From<bincode::Error> for Error {
    fn from(_: bincode::Error) -> Self {
        Self::BincodeError
    }
}
