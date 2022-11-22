//! HTTP client for the Tandem SMPC engine.
//!
//! This crate provides an HTTP client acting as the `evaluator` and running the Tandem Multi-Party
//! Computation engine. An HTTP server is expected to act as the `contributor`.
//!
//! This crate provides a CLI client, as well as functions targeting WebAssembly to provide an easy
//! integration of the Tandem engine with JavaScript.
//!
//! This crate additionally includes an interactive notebook (provided by `index.html`) to run and
//! test Garble programs during development.

#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
// otherwise wasm_bindgen causes a clippy warning, see
// https://github.com/rustwasm/wasm-bindgen/issues/2774
#![allow(clippy::unused_unit)]

use msg_queue::{MessageId, MsgQueue};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use reqwest::Response;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt};
use tandem::{states::Msg, Circuit, CircuitBlake3Hash};
use tandem_garble_interop::{
    check_program, compile_program, deserialize_output, parse_input, Role, TypedCircuit,
};
pub use tandem_garble_interop::{Literal, VariantLiteral};
use url::Url;

const MAX_RETRIES: usize = 10;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

use self::ValidationError::*;

mod msg_queue;

/// An MPC program that was type-checked and can be executed by the Tandem engine.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[derive(Debug, Clone)]
pub struct MpcProgram {
    source_code: String,
    function_name: String,
    ast: tandem_garble_interop::TypedProgram,
    circuit: tandem_garble_interop::TypedCircuit,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl MpcProgram {
    /// Type-checks the specified function, returning a compiled program.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(constructor))]
    pub fn new(source_code: String, function_name: String) -> Result<MpcProgram, Error> {
        let source_code = source_code.trim().to_string();
        let ast = check_program(&source_code).map_err(GarbleCompileTimeError)?;
        let circuit = compile_program(&ast, &function_name).map_err(GarbleCompileTimeError)?;

        if circuit.fn_def.params.len() != 2 {
            return Err(ValidationError::GarbleProgramIsNoTwoPartyFunction.into());
        }
        Ok(Self {
            source_code,
            function_name,
            ast,
            circuit,
        })
    }

    /// Returns the number of gates in the circuit as a formatted string.
    ///
    /// E.g. "79k gates (XOR: 44k, NOT: 13k, AND: 21k)"
    pub fn report_gates(&self) -> String {
        self.circuit.info_about_gates.to_string()
    }
}

/// Stores data (either inputs or output) in an Tandem-compatible format.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpcData {
    literal: Literal,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl MpcData {
    /// Parses and type-checks a Garble string literal as MpcData.
    /// ```
    /// // Garble program stored as a string.
    /// let source_code = "pub fn card_guess(house: Card, player: Card) -> bool {
    ///     house == player
    /// }
    ///
    /// struct Card {
    ///     suit: Suit,
    ///     value: Value,
    /// }
    //
    /// enum Suit {
    ///     Diamonds,
    ///     Clubs,
    ///     Hearts,
    ///     Spades,
    /// }
    ///
    /// enum Value {
    ///     Jack,
    ///     Queen,
    ///     King,
    /// }";
    ///
    /// let card_guess_program =
    ///     tandem_http_client::MpcProgram::new(source_code.to_string(), "card_guess".to_string()).unwrap();
    ///
    /// let player_card_string = "Card {suit: Suit::Diamonds, value: Value::Jack}";
    ///
    /// let player_card =
    ///     tandem_http_client::MpcData::from_string(&card_guess_program, player_card_string.to_string())
    ///         .unwrap();
    ///
    /// assert_eq!(
    ///     player_card.to_literal_string(),
    ///     "Card {suit: Suit::Diamonds, value: Value::Jack}"
    /// );
    /// ```
    pub fn from_string(program: &MpcProgram, input: String) -> Result<MpcData, Error> {
        let literal = parse_input(
            Role::Evaluator,
            &program.ast,
            &program.circuit.fn_def,
            &input,
        )
        .map_err(GarbleCompileTimeError)?;
        Ok(MpcData { literal })
    }

    /// Type-checks a Garble literal, returning it as MpcData.
    /// ```
    ///
    /// use tandem_http_client::{Literal, VariantLiteral};
    ///
    /// let source_code = "pub fn card_guess(house: Card, player: Card) -> bool {
    ///     house == player
    /// }
    ///
    /// pub struct Card {
    ///     suit: Suit,
    ///     value: Value,
    /// }
    ///
    /// enum Suit {
    ///     Diamonds,
    ///     Clubs,
    ///     Hearts,
    ///     Spades,
    /// }
    ///
    /// enum Value {
    ///     Jack,
    ///     Queen,
    ///     King,
    /// }";
    ///
    /// let player_card_literal = Literal::Struct(
    ///     "Card".to_string(),
    ///     vec![
    ///         (
    ///             "suit".to_string(),
    ///             Literal::Enum(
    ///                 "Suit".to_string(),
    ///                 "Diamonds".to_string(),
    ///                 VariantLiteral::Unit,
    ///             ),
    ///         ),
    ///         (
    ///             "value".to_string(),
    ///             Literal::Enum(
    ///                 "Value".to_string(),
    ///                 "Jack".to_string(),
    ///                 VariantLiteral::Unit,
    ///             ),
    ///         ),
    ///     ],
    /// );
    ///
    /// let card_guess_program =
    ///     tandem_http_client::MpcProgram::new(source_code.to_string(), "card_guess".to_string()).unwrap();
    ///
    /// let player_card =
    ///     tandem_http_client::MpcData::from_literal(&card_guess_program, player_card_literal)
    ///         .unwrap();
    ///
    /// assert_eq!(
    ///     player_card.to_literal_string(),
    ///     "Card {suit: Suit::Diamonds, value: Value::Jack}"
    /// );
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_literal(program: &MpcProgram, literal: Literal) -> Result<MpcData, Error> {
        let expected_type =
            tandem_garble_interop::input_type(Role::Evaluator, &program.circuit.fn_def);
        if !literal.is_of_type(&program.ast, expected_type) {
            return Err(Error::ValidationError(
                ValidationError::GarbleCompileTimeError(format!(
                    "Input literal is not of the type {expected_type}"
                )),
            ));
        }
        Ok(MpcData { literal })
    }

    /// Parses and type-checks a Garble literal in its JSON representation as MpcData.
    /// ```
    /// // Garble program stored as a string.
    /// let source_code = "pub fn card_guess(house: Card, player: Card) -> bool {
    ///     house == player
    /// }
    ///
    /// struct Card {
    ///     suit: Suit,
    ///     value: Value,
    /// }
    ///
    /// enum Suit {
    ///     Diamonds,
    ///     Clubs,
    ///     Hearts,
    ///     Spades,
    /// }
    ///
    /// enum Value {
    ///     Jack,
    ///     Queen,
    ///     King,
    /// }";
    ///
    /// let card_game_program =
    ///     tandem_http_client::MpcProgram::new(source_code.to_string(), "card_game".to_string()).unwrap();
    ///
    /// let json_string = "{
    ///     \"Struct\": [
    ///         \"Card\",
    ///         [
    ///             [
    ///                 \"suit\",
    ///                 {
    ///                     \"Enum\": [
    ///                         \"Suit\",
    ///                         \"Diamonds\",
    ///                         \"Unit\"
    ///                     ]
    ///                 }
    ///             ],
    ///             [
    ///                 \"value\",
    ///                 {
    ///                     \"Enum\": [
    ///                         \"Value\",
    ///                         \"Jack\",
    ///                         \"Unit\"
    ///                     ]
    ///                 }
    ///             ]
    ///         ]
    ///     ]
    /// }";
    ///
    /// let js_value_literal = serde_json::from_str(json_string);
    ///
    /// let player_card = tandem_http_client::MpcData::from_object(&card_guess_program, js_value_literal);
    ///
    /// assert_eq!(
    ///     player_card.to_literal_string(),
    ///     "Card {suit: Suit::Diamonds, value: Value::Jack}"
    /// );
    /// ```
    ///
    #[cfg(target_arch = "wasm32")]
    pub fn from_object(program: &MpcProgram, literal: JsValue) -> Result<MpcData, Error> {
        let literal: Literal =
            serde_wasm_bindgen::from_value(literal).map_err(|e| Error::JsonError(e.to_string()))?;
        let expected_type =
            tandem_garble_interop::input_type(Role::Evaluator, &program.circuit.fn_def);
        if !literal.is_of_type(&program.ast, &expected_type) {
            return Err(Error::ValidationError(
                ValidationError::GarbleCompileTimeError(format!(
                    "Input literal is not of the type {expected_type}"
                )),
            ));
        }
        Ok(MpcData { literal })
    }

    /// Returns MpcData as a Garble literal string.
    ///
    /// See [`MpcData::from_string`] for the format of the literal string returned here.
    pub fn to_literal_string(&self) -> String {
        format!("{}", self.literal)
    }

    /// Returns MpcData as a Garble literal in its JSON representation.
    ///
    /// See [`MpcData::from_object`] for the format of the JsValue returned here.
    #[cfg(target_arch = "wasm32")]
    pub fn to_literal(&self) -> Result<JsValue, serde_wasm_bindgen::Error> {
        serde_wasm_bindgen::to_value(&self.literal)
    }
}

/// Computes the specified program using Multi-Party Computation, keeping the input private.
///
/// A Tandem server must be running at the specified url, to provide the contributor's input.
///
/// The client can send plaintext metadata to the server, to influence the server's choice of the
/// input.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub async fn compute(
    url: String,
    plaintext_metadata: String,
    program: MpcProgram,
    input: MpcData,
) -> Result<MpcData, Error> {
    let url = Url::parse(&url)?;

    let my_input = input.literal.as_bits(&program.ast);

    let expected_input_len = program
        .circuit
        .gates
        .gates()
        .iter()
        .filter(|&gate| gate == &tandem::Gate::InEval)
        .count();

    if expected_input_len != my_input.len() {
        return Err(ValidationError::InvalidInput.into());
    }

    let client = TandemClient::new(&url);
    let TypedCircuit { gates, fn_def, .. } = program.circuit;
    let session = client
        .new_session(
            &gates,
            program.source_code.clone(),
            program.function_name.clone(),
            plaintext_metadata,
        )
        .await?;
    let result = session.evaluate(gates, my_input).await?;
    let literal =
        deserialize_output(&program.ast, &fn_def, &result).map_err(GarbleCompileTimeError)?;
    Ok(MpcData { literal })
}

type MessageLog = Vec<(Msg, MessageId)>;

#[derive(Debug)]
struct TandemClient {
    url: Url,
}

struct TandemSession {
    url: Url,
    request_headers: HashMap<String, String>,
}

#[derive(Serialize, Debug)]
struct NewSession {
    plaintext_metadata: String,
    program: String,
    function: String,
    circuit_hash: CircuitBlake3Hash,
}

#[derive(Deserialize, Debug, PartialEq, Eq)]
struct EngineCreationResult {
    engine_id: String,
    request_headers: HashMap<String, String>,
}

impl TandemClient {
    fn new(url: &Url) -> Self {
        Self { url: url.clone() }
    }

    async fn new_session<'a, 'b>(
        &'a self,
        circuit: &Circuit,
        source_code: String,
        function: String,
        plaintext_metadata: String,
    ) -> Result<TandemSession, Error> {
        let req = NewSession {
            plaintext_metadata,
            program: source_code,
            function,
            circuit_hash: circuit.blake3_hash(),
        };
        let EngineCreationResult {
            engine_id,
            request_headers,
        } = send_new_session(self.url.clone(), &req).await?;
        let url = self.url.join(&engine_id)?;

        Ok(TandemSession {
            url,
            request_headers,
        })
    }
}

impl TandemSession {
    async fn evaluate(self, circuit: Circuit, input: Vec<bool>) -> Result<Vec<bool>, Error> {
        let mut context = MsgQueue::new();
        let mut evaluator =
            tandem::states::Evaluator::new(circuit, input, ChaCha20Rng::from_entropy())?;

        let mut last_durably_received_offset: Option<MessageId> = None;
        let mut steps_remaining = evaluator.steps();
        loop {
            let messages: Vec<(&Msg, MessageId)> = context.msgs_iter().collect();
            let (upstream_msgs, server_commited_offset) =
                self.dialog(last_durably_received_offset, &messages).await?;
            if messages.last().map(|v| v.1) != server_commited_offset {
                return Err(Error::MessageOffsetMismatch);
            }

            if let Some(last_durably_received_offset) = server_commited_offset {
                context.flush_queue(last_durably_received_offset);
            }

            for (msg, server_offset) in &upstream_msgs {
                if *server_offset != last_durably_received_offset.map(|o| o + 1).unwrap_or(0) {
                    return Err(Error::MessageOffsetMismatch);
                }

                if steps_remaining > 0 {
                    let (next_state, msg) = evaluator.run(msg)?;
                    evaluator = next_state;
                    steps_remaining -= 1;
                    context.send(msg);
                } else {
                    return Ok(evaluator.output(msg)?);
                }
                last_durably_received_offset = Some(*server_offset);
            }
        }
    }

    async fn dialog(
        &self,
        last_durably_received_offset: Option<u32>,
        messages: &[(&Msg, MessageId)],
    ) -> Result<(MessageLog, Option<MessageId>), Error> {
        let mut errors = vec![];
        for _ in 0..MAX_RETRIES {
            match send_msgs(
                self.url.clone(),
                &self.request_headers,
                last_durably_received_offset,
                messages,
            )
            .await
            {
                Ok(resp) => return Ok(resp),
                Err(e) => errors.push(e),
            }
        }
        Err(Error::MaxRetriesExceeded(errors))
    }
}

async fn send_new_session(url: Url, session: &NewSession) -> Result<EngineCreationResult, Error> {
    let client = reqwest::Client::new();
    let resp = client.post(url).json(session).send().await?;
    let resp = resp_or_err(resp).await?;
    Ok(resp.json::<EngineCreationResult>().await?)
}

async fn send_msgs(
    url: Url,
    request_headers: &HashMap<String, String>,
    last_durably_received_offset: Option<u32>,
    msgs: &[(&Msg, MessageId)],
) -> Result<(MessageLog, Option<MessageId>), Error> {
    let client = reqwest::Client::new();
    let body = bincode::serialize(&(last_durably_received_offset, msgs))?;
    let mut req = client.post(url).body(body);
    for (k, v) in request_headers.iter() {
        req = req.header(k, v);
    }
    let resp = req.send().await?;
    let resp = resp_or_err(resp).await?;
    Ok(bincode::deserialize(&resp.bytes().await?)?)
}

async fn resp_or_err(resp: Response) -> Result<Response, Error> {
    if resp.status().is_success() {
        Ok(resp)
    } else {
        let e = resp.text().await?;
        let e = match serde_json::from_str::<ErrorJson>(&e) {
            Ok(ErrorJson { error, args }) => format!("{error}: {args}"),
            Err(_) => e,
        };
        Err(Error::ServerError(e))
    }
}

#[derive(Deserialize)]
struct ErrorJson {
    error: String,
    args: String,
}

/// Errors occurring during the validation or the execution of the MPC protocol.
#[derive(Debug)]
pub enum Error {
    /// An error occurred on the server side.
    ServerError(String),
    /// An error occurred while trying to send a request to the server.
    ReqwestError(reqwest::Error),
    /// The provided JSON is not a valid Garble literal.
    JsonError(String),
    /// The provided URL is invalid.
    ParseError(url::ParseError),
    /// The MPC program or the input is invalid.
    ValidationError(ValidationError),
    /// An error occurred during the client's execution of the MPC protocol.
    TandemError(tandem::Error),
    /// A message could not be serialized/deserialized.
    BincodeError,
    /// The client's message id did not match the server's message id.
    MessageOffsetMismatch,
    /// The request failed after exceeding the maximum number of retries.
    MaxRetriesExceeded(Vec<Error>),
}

impl From<bincode::Error> for Error {
    fn from(_: bincode::Error) -> Self {
        Self::BincodeError
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Self::ReqwestError(e)
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Self {
        Self::ParseError(e)
    }
}

impl From<ValidationError> for Error {
    fn from(e: ValidationError) -> Self {
        Self::ValidationError(e)
    }
}

impl From<tandem::Error> for Error {
    fn from(e: tandem::Error) -> Self {
        Self::TandemError(e)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ValidationError(e) => write!(f, "The MPC program or the input is invalid: {e}"),
            Error::ServerError(e) => write!(f, "An error occurred on the server side: {e}"),
            Error::ReqwestError(e) => write!(
                f,
                "An error occurred while trying to send a request to the server: {e}"
            ),
            Error::JsonError(e) => {
                write!(f, "The provided JSON is not a valid Garble literal: {e}")
            }
            Error::ParseError(e) => write!(f, "The provided URL is invalid: {e}"),
            Error::TandemError(e) => write!(
                f,
                "An error occurred during the client's execution of the MPC protocol: {e}"
            ),
            Error::BincodeError => write!(f, "A message could not be serialized/deserialized."),
            Error::MessageOffsetMismatch => write!(
                f,
                "The client's message id did not match the server's message id."
            ),
            Error::MaxRetriesExceeded(errs) => {
                write!(f, "The request failed after {MAX_RETRIES} retries: ")?;
                for e in errs {
                    e.fmt(f)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for Error {}

#[cfg(target_arch = "wasm32")]
impl From<Error> for JsValue {
    fn from(e: Error) -> Self {
        JsValue::from_str(&format!("{e}"))
    }
}

/// An error that occurred during validation, before the MPC execution.
#[derive(Debug, PartialEq, Eq)]
pub enum ValidationError {
    /// The input does not match the circuit's expected input.
    InvalidInput,
    /// An error was found while scanning, parsing or type-checking the program.
    GarbleCompileTimeError(String),
    /// The Garble program has more or fewer than two parameters and thus is not a 2-Party program.
    GarbleProgramIsNoTwoPartyFunction,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InvalidInput => write!(f, "The input does not match the circuit's expected input."),
            GarbleCompileTimeError(e) => write!(f, "Garble compile time error: {e}"),
            GarbleProgramIsNoTwoPartyFunction => write!(
                f,
                "The Garble program has more or fewer than two parameters and thus is not a 2-Party program."
            ),
        }
    }
}
