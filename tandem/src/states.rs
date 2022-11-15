//! The different states in the MPC protocol.
//!
//! At each step in the protocol, each party ([`Contributor`] and [`Evaluator`]) always expects a
//! single message from the other party. Based on the message each party either computes the final
//! output or transitions into a new state, returning a message for the other party.
//!
//! The parties are deliberately communication-agnostic and expect the sending and receiving of
//! messages to be handled by the user of this crate. As a result, the crate works both in sync and
//! async environments.

use std::borrow::Borrow;

use crate::{
    hash::{garbling_hash, hash, hash_key, hash_keys},
    leakyand::{compute_leaky_and_hashes, derive_and_shares},
    leakydelta_ot::{
        message::{OtInitReply, SerializedOtInit},
        LeakyOtReceiver, LeakyOtSender, ReceiverInitializer, SenderInitializer, BLOCK_SIZE,
    },
    protocol::{
        self,
        cointossing::{CoinResult, CoinShare},
    },
    types::{
        AndTableShare, BitShare, Delta, InputMaskShare, KeyType, MacType, PartialBitShare,
        TableShare, WireLabel, WireMask, WireState, K,
    },
    Circuit,
    Error::{self, *},
    Gate, GateIndex,
};
use bincode::{deserialize, serialize};
use rand::Rng;
use rand_chacha::ChaCha20Rng;

/// The type of messages exchanged between [`Contributor`] and [`Evaluator`].
pub type Msg = Vec<u8>;

const TRIPLES: usize = BLOCK_SIZE * 3;

/// The party that contributes its input to the MPC protocol.
pub struct Contributor<C: Borrow<Circuit>, I: Borrow<[bool]>> {
    state: Box<ContribState>,
    circuit: C,
    input: I,
}

/// The party that evaluates the circuit and the output.
///
/// Upon successful circuit evaluation, the evaluator can access the plain text output.
pub struct Evaluator<C: Borrow<Circuit>, I: Borrow<[bool]>> {
    state: Box<EvalState>,
    circuit: C,
    input: I,
}

impl<C: Borrow<Circuit>, I: Borrow<[bool]>> Contributor<C, I> {
    /// Initializes the contributor, returning a state and an initial message for the [`Evaluator`].
    pub fn new(circuit: C, input: I, rng: ChaCha20Rng) -> Result<(Self, Msg), Error> {
        let (state, msg) = ContribStep1::init(circuit.borrow(), input.borrow(), rng)?;
        let contrib = Self {
            state: Box::new(ContribState::Step1(state)),
            circuit,
            input,
        };
        Ok((contrib, msg))
    }

    /// Returns the number of messages that need to be exchanged before the protocol is completed.
    ///
    /// When the end state is reached, the contributor's last message will enable the [`Evaluator`]
    /// to compute the final output.
    pub fn steps(&self) -> u32 {
        7
    }

    /// Executes a single step in the protocol, based on the message received from the [`Evaluator`].
    pub fn run(self, msg: &[u8]) -> Result<(Contributor<C, I>, Msg), Error> {
        use ContribState::*;

        let (state, msg) = match *self.state {
            Step1(s) => {
                let (state, msg) = s.run(msg)?;
                (Box::new(Step1a(state)), msg)
            }
            Step1a(s) => {
                let (state, msg) = s.run(msg, self.circuit.borrow())?;
                (Box::new(Step2(state)), msg)
            }
            Step2(s) => {
                let (state, msg) = s.run(msg)?;
                (Box::new(Step3(state)), msg)
            }
            Step3(s) => {
                let (state, msg) = s.run(msg)?;
                (Box::new(Step4(state)), msg)
            }
            Step4(s) => {
                let (state, msg) = s.run(msg, self.circuit.borrow())?;
                (Box::new(Step5(ContribBucketingStep(state))), msg)
            }
            Step5(s) => {
                let (state, msg) = s.run(msg, self.circuit.borrow(), self.input.borrow())?;
                (Box::new(Step6(state)), msg)
            }
            Step6(s) => {
                let ((), msg) = s.run(msg, self.circuit.borrow(), self.input.borrow())?;
                (Box::new(Done), msg)
            }
            Done => return Err(Error::ProtocolEnded),
        };
        let next_state = Contributor {
            state,
            circuit: self.circuit,
            input: self.input,
        };
        Ok((next_state, msg))
    }
}

impl<C: Borrow<Circuit>, I: Borrow<[bool]>> Evaluator<C, I> {
    /// Initializes the evaluator, returning its initial state.
    pub fn new(circuit: C, input: I, rng: ChaCha20Rng) -> Result<Self, Error> {
        let state = EvalStep1::init(circuit.borrow(), input.borrow(), rng)?;
        Ok(Self {
            state: Box::new(EvalState::Step1(state)),
            circuit,
            input,
        })
    }

    /// Returns the number of messages that need to be exchanged before reaching the end state.
    ///
    /// After the end state is reached, the evaluator expects one last message from the
    /// [`Contributor`] to compute the final output.
    pub fn steps(&self) -> u32 {
        7
    }

    /// Executes a single step in the protocol, based on the message received from the [`Contributor`].
    pub fn run(self, msg: &[u8]) -> Result<(Evaluator<C, I>, Msg), Error> {
        use EvalState::*;

        let (state, msg) = match *self.state {
            Step1(s) => {
                let (state, msg) = s.run(msg, self.circuit.borrow())?;
                (Box::new(Step2(state)), msg)
            }
            Step2(s) => {
                let (state, msg) = s.run(msg)?;
                (Box::new(Step2a(state)), msg)
            }
            Step2a(s) => {
                let (state, msg) = s.run(msg, self.circuit.borrow())?;
                (Box::new(Step3(state)), msg)
            }
            Step3(s) => {
                let (state, msg) = s.run(msg)?;
                (Box::new(Step4(state)), msg)
            }
            Step4(s) => {
                let (state, msg) = s.run(msg)?;
                (Box::new(Step5(state)), msg)
            }
            Step5(s) => {
                let (state, msg) = s.run(msg, self.circuit.borrow())?;
                (Box::new(Step6(state)), msg)
            }
            Step6(s) => {
                let (state, msg) = s.run(msg, self.circuit.borrow(), self.input.borrow())?;
                (Box::new(Step8(state)), msg)
            }
            Step8(s) => {
                let (output, _) = s.run(msg, self.circuit.borrow())?;
                (Box::new(Done(output)), vec![])
            }
            Done(_) => return Err(Error::ProtocolEnded),
        };
        let next_state = Evaluator {
            state,
            circuit: self.circuit,
            input: self.input,
        };
        Ok((next_state, msg))
    }

    /// Returns the output of the computation or `None` if the protocol has not ended.
    pub fn output(self, msg: &[u8]) -> Result<Vec<bool>, Error> {
        match *self.state {
            EvalState::Step8(s) => {
                let (output, _) = s.run(msg, self.circuit.borrow())?;
                Ok(output)
            }
            _ => Err(Error::ProtocolStillInProgress),
        }
    }
}

type TandemResult<S> = Result<(S, Msg), Error>;

enum ContribState {
    Step1(ContribStep1),
    Step1a(ContribStep1a),
    Step2(ContribStep2),
    Step3(ContribStep3),
    Step4(ContribStep4),
    Step5(ContribBucketingStep),
    Step6(InputProcContrib),
    Done,
}

enum EvalState {
    Step1(EvalStep1),
    Step2(EvalStep2),
    Step2a(EvalStep2a),
    Step3(EvalStep3),
    Step4(EvalStep4),
    Step5(EvalStep5),
    Step6(EvalStep6),
    Step8(InputProcEval),
    Done(Vec<bool>),
}

#[derive(Clone)]
struct EvalStep1(OtPreInitState);

#[derive(Clone)]
struct ContribStep1(OtInitState1);

#[derive(Clone)]
struct ContribStep1a(OtInitState3);

#[derive(Clone)]
struct EvalStep2(OtInitState2);

struct EvalStep2a(OtInitState4);

#[derive(Clone)]
struct ContribStep2(OtAndsState1);

#[derive(Clone)]
struct EvalStep3(OtAndsState2);

#[derive(Clone)]
struct ContribStep3(OtAndsState2);

#[derive(Clone)]
struct EvalStep4(OtAndsState3);

#[derive(Clone)]
struct ContribStep4(OtAndsState4);

#[derive(Clone)]
struct ContribBucketingStep(AndsBucketingState);

#[derive(Clone)]
struct ContribStep5(OtAndsState6);

#[derive(Clone)]
struct EvalStep5(OtAndsState5);

#[derive(Clone)]
struct EvalStep6(OtAndsState6);

#[derive(Clone)]
struct OtPreInitState {
    rng: ChaCha20Rng,
}

#[derive(Clone)]
struct OtInitState1 {
    rng: ChaCha20Rng,
    delta: Delta,
    r_init: ReceiverInitializer,
    coin_share: CoinShare,
    blocks: usize,
}

#[derive(Clone)]
struct OtInitState2 {
    rng: ChaCha20Rng,
    delta: Delta,
    r_init: ReceiverInitializer,
    s: SenderInitializer,
    coin_share: CoinShare,
    coin_commitment: Vec<u8>, //< upstream coin commitment message
    blocks: usize,
}

#[derive(Clone)]
struct OtInitState3 {
    rng: ChaCha20Rng,
    delta: Delta,
    s: SenderInitializer,
    r: LeakyOtReceiver,
    coin: CoinResult,
    blocks: usize,
}

#[derive(Clone)]
struct OtInitState4 {
    rng: ChaCha20Rng,
    delta: Delta,
    s: LeakyOtSender,
    coin: CoinResult,
    blocks: usize,
    abits: Vec<BitShare>,
}

#[derive(Clone)]
struct OtAndsState1 {
    rng: ChaCha20Rng,
    delta: Delta,
    coin: CoinResult,
    random_bits: Vec<MacType>,
    and_triples: Vec<BitShare>,
    wire_abits: Vec<BitShare>,
    and_shares: Vec<MacType>,
    r_and_rand_key: Vec<(MacType, KeyType)>,
    r_and_rand_hash: Vec<MacType>,
    r_prime: Vec<MacType>,
}

#[derive(Clone)]
struct OtAndsState2 {
    rng: ChaCha20Rng,
    delta: Delta,
    coin: CoinResult,
    and_triples: Vec<BitShare>,
    wire_abits: Vec<BitShare>,
    and_shares: Vec<MacType>,
    r_and_rand_key: Vec<(MacType, KeyType)>,
    r_and_rand_hash: Vec<MacType>,
    r_prime: Vec<MacType>,
}

#[derive(Clone)]
struct OtAndsState3 {
    rng: ChaCha20Rng,
    delta: Delta,
    coin: CoinResult,
    and_triples: Vec<BitShare>,
    wire_abits: Vec<BitShare>,
    r_and_rand_key: Vec<(MacType, KeyType)>,
    r_and_rand_hash: Vec<MacType>,
    r_prime: Vec<MacType>,
}

#[derive(Clone)]
struct OtAndsState4 {
    rng: ChaCha20Rng,
    delta: Delta,
    coin: CoinResult,
    and_triples: Vec<BitShare>,
    wire_abits: Vec<BitShare>,
    r_and_rand_key: Vec<(MacType, KeyType)>,
    r_and_rand_hash: Vec<MacType>,
    r_prime: Vec<MacType>,
}

#[derive(Clone)]
struct OtAndsState5 {
    rng: ChaCha20Rng,
    delta: Delta,
    coin: CoinResult,
    and_triples: Vec<BitShare>,
    wire_abits: Vec<BitShare>,
    r_and_rand_key: Vec<(MacType, KeyType)>,
    r_and_rand_hash: Vec<MacType>,
    r_prime: Vec<MacType>,
}

#[derive(Clone)]
struct AndsBucketingState {
    rng: ChaCha20Rng,
    delta: Delta,
    bucketing_bits: Vec<bool>,
    wire_abits: Vec<BitShare>,
    and_triples: Vec<BitShare>,
    permutation: Vec<u32>,
    length: usize, // number of resulting and triples
    bucket_size: usize,
}

#[derive(Clone)]
struct OtAndsState6 {
    delta: Delta,
    and_triples: Vec<BitShare>,
    masks: Vec<WireMask>,
    lhs_and_bits: Vec<bool>,
    rhs_and_bits: Vec<bool>,
}

/// WRK17 "input processing phase".
#[derive(Clone)]
struct InputProcContrib {
    delta: Delta,
    pending_from_b: usize,
    mac_checks_success: bool,
    masks: Vec<WireMask>,
}

/// WRK17 "input processing phase" / "circuit evaluation phase".
struct InputProcEval {
    delta: Delta,
    pending_input: usize,
    masks: Vec<WireMask>,
    wires: Vec<WireState>,
}

impl EvalStep1 {
    pub(crate) fn init(circuit: &Circuit, input: &[bool], rng: ChaCha20Rng) -> Result<Self, Error> {
        circuit.validate_evaluator_input(input)?;
        let state = OtPreInitState { rng };
        Ok(Self(state))
    }
}

impl ContribStep1 {
    pub(crate) fn init(
        circuit: &Circuit,
        input: &[bool],
        mut rng: ChaCha20Rng,
    ) -> Result<(Self, Msg), Error> {
        circuit.validate_contributor_input(input)?;
        let (state, msg) = init_ot1(Delta::gen_random(&mut rng), rng, circuit)?;
        Ok((Self(state), msg))
    }
}

impl EvalStep1 {
    fn run(mut self, msg: &[u8], circuit: &Circuit) -> TandemResult<EvalStep2> {
        let (state, reply1) = init_ot1(Delta::gen_random(&mut self.0.rng), self.0.rng, circuit)?;
        let (state, reply2) = init_ot2(state, msg)?;
        let reply = serialize(&(reply1, reply2))?;
        Ok((EvalStep2(state), reply))
    }
}

impl ContribStep1 {
    fn run(self, msg: &[u8]) -> TandemResult<ContribStep1a> {
        let (msg1, msg2): (Msg, Msg) = deserialize(msg)?;
        let (state, reply1) = init_ot2(self.0, &msg1)?;
        let (state, reply2) = init_ot3(state, &msg2)?;
        let reply = serialize(&(reply1, reply2))?;
        Ok((ContribStep1a(state), reply))
    }
}

impl EvalStep2 {
    fn run(self, msg: &[u8]) -> TandemResult<EvalStep2a> {
        let (msg1, msg2): (Msg, Msg) = deserialize(msg)?;
        let (state, reply1) = init_ot3(self.0, &msg1)?;
        let (state, reply2) = init_ot4(state, msg2)?;
        let reply = serialize(&(reply1, reply2))?;
        Ok((EvalStep2a(state), reply))
    }
}

impl ContribStep1a {
    fn run(self, msg: &[u8], circuit: &Circuit) -> TandemResult<ContribStep2> {
        let (msg1, msg2): (Msg, Msg) = deserialize(msg)?;
        let (state, reply1) = init_ot4(self.0, msg1)?;
        let (state, reply2) = ot_ands1(state, &msg2, circuit)?;
        let reply = serialize(&(reply1, reply2))?;
        Ok((ContribStep2(state), reply))
    }
}

impl EvalStep2a {
    fn run(self, msg: &[u8], circuit: &Circuit) -> TandemResult<EvalStep3> {
        let (msg1, msg2): (Msg, Msg) = deserialize(msg)?;
        let (state, reply) = ot_ands1(self.0, &msg1, circuit)?;

        // Step 2 of `Π_{LaAND}`
        let and_hashes: Vec<[MacType; 2]> = deserialize(&msg2)?;
        let and_shares = state.compute_and_shares(&and_hashes, Role::Evaluator)?;
        let state = OtAndsState2 {
            rng: state.rng,
            delta: state.delta,
            coin: state.coin,
            and_triples: state.and_triples,
            wire_abits: state.wire_abits,
            r_and_rand_key: state.r_and_rand_key,
            r_and_rand_hash: state.r_and_rand_hash,
            r_prime: state.r_prime,
            and_shares,
        };
        Ok((EvalStep3(state), reply))
    }
}

impl ContribStep2 {
    // Implements Step 2 of `Π_{LaAND}` of WRK17a
    fn run(self, msg: &[u8]) -> TandemResult<ContribStep3> {
        let and_hashes: Vec<[MacType; 2]> = deserialize(msg)?;
        let state = self.0;
        let and_shares = state.compute_and_shares(&and_hashes, Role::Contributor)?;
        let reply = serialize(&and_shares)?;
        let state = OtAndsState2 {
            rng: state.rng,
            delta: state.delta,
            coin: state.coin,
            and_triples: state.and_triples,
            wire_abits: state.wire_abits,
            r_and_rand_key: state.r_and_rand_key,
            r_and_rand_hash: state.r_and_rand_hash,
            r_prime: state.r_prime,
            and_shares: state.and_shares,
        };
        Ok((ContribStep3(state), reply))
    }
}

/// Receives its message from [`ContribStep2`] which is a (large) vector of `AND` shares.
impl EvalStep3 {
    fn run(self, msg: &[u8]) -> TandemResult<EvalStep4> {
        let (state, replies) = ot_ands3_update_z2_eval(self.0, msg)?;
        let reply = serialize(&replies)?;
        Ok((EvalStep4(state), reply))
    }
}

impl ContribStep3 {
    fn run(self, msg: &[u8]) -> TandemResult<ContribStep4> {
        let (msg1, msg2): (Msg, Msg) = deserialize(msg)?;
        let (state, reply1) = ot_ands3_update_z2_contrib(self.0, &msg1)?;
        let (state, reply2) = ot_ands4(state, &msg2)?;
        let reply = serialize(&(reply1, reply2))?;
        Ok((ContribStep4(state), reply))
    }
}

impl EvalStep4 {
    fn run(self, msg: &[u8]) -> TandemResult<EvalStep5> {
        let (msg1, msg2): (Msg, Msg) = deserialize(msg)?;
        let (state, reply1) = ot_ands4(self.0, &msg1)?;
        let (state, reply2) = ot_ands5(state, &msg2)?;
        let reply = serialize(&(reply1, reply2))?;
        Ok((EvalStep5(state), reply))
    }
}

impl ContribStep4 {
    fn run(self, msg: &[u8], circuit: &Circuit) -> TandemResult<AndsBucketingState> {
        let (msg1, msg2): (Msg, Msg) = deserialize(msg)?;
        let (state, reply1) = ot_ands5(self.0, &msg1)?;
        let (state, reply2) = ot_ands6(state, &msg2, circuit)?;
        let reply = serialize(&(reply1, reply2))?;
        Ok((state, reply))
    }
}

impl EvalStep5 {
    fn run(self, msg: &[u8], circuit: &Circuit) -> TandemResult<EvalStep6> {
        let (msg1, msg2): (Msg, Msg) = deserialize(msg)?;
        let (state, reply1) = ot_ands6(self.0, &msg1, circuit)?;
        let (state, reply2) = state.finish(&msg2, circuit)?;

        let msg = serialize(&(reply1, reply2))?;
        Ok((EvalStep6(state), msg))
    }
}

impl ContribBucketingStep {
    fn run(self, msg: &[u8], circuit: &Circuit, input: &[bool]) -> TandemResult<InputProcContrib> {
        let (msg1, msg2): (Msg, Msg) = deserialize(msg)?;
        let (state, reply1) = self.0.finish(&msg1, circuit)?;
        let (state, reply2) = ot_ands8_contrib(state, &msg2, circuit, input)?;

        let msg = serialize(&(reply1, reply2))?;
        Ok((state, msg))
    }
}

impl EvalStep6 {
    fn run(self, msg: &[u8], circuit: &Circuit, input: &[bool]) -> TandemResult<InputProcEval> {
        let (msg1, msg2): (Msg, Msg) = deserialize(msg)?;
        let (state, reply) = ot_ands8_eval(self.0, &msg1, &msg2, circuit, input)?;
        Ok((state, reply))
    }
}

type StateResult<S> = Result<(S, Msg), Error>;

/// Calculates the bucket size according to WRK17a, Table 4 for statistical security ρ = 40 (rho).
fn bucket_size(circuit: &Circuit) -> usize {
    match circuit.and_gates() {
        n if n >= 280_000 => 3,
        n if n >= 3_100 => 4,
        _ => 5,
    }
}

enum Role {
    Contributor,
    Evaluator,
}

fn init_ot1(delta: Delta, mut rng: ChaCha20Rng, p: &Circuit) -> StateResult<OtInitState1> {
    p.validate()?;

    // the number of authenticated bits we need for wires
    let wire_abits = p.and_gates() + p.eval_inputs() + p.contrib_inputs();

    // the number of authenticated bits need for AND triples
    let triples_bits = p.and_gates() * 3 * bucket_size(p);
    let triples_bits_aligned = (triples_bits + TRIPLES - 1) / TRIPLES * TRIPLES;
    let total_abits = wire_abits + triples_bits_aligned;
    let num_abits_aligned = (total_abits + BLOCK_SIZE - 1) / BLOCK_SIZE * BLOCK_SIZE;
    let (r_init, ot_msg) = ReceiverInitializer::init(&mut rng);
    let (coin_share, coin_msg) = {
        let mut coin = [0u8; protocol::cointossing::COIN_LEN];
        rng.fill(&mut coin);
        protocol::cointossing::init(coin)?
    };

    let msg = serialize(&(&ot_msg.serialize(), &coin_msg))?;
    let state = OtInitState1 {
        rng,
        delta,
        r_init,
        coin_share,
        blocks: num_abits_aligned / BLOCK_SIZE,
    };
    Ok((state, msg))
}

fn init_ot2(mut state: OtInitState1, msg: &[u8]) -> StateResult<OtInitState2> {
    let (serialized_ot_init, coin_commitment): (SerializedOtInit, Vec<u8>) = deserialize(msg)?;
    let ot_init = serialized_ot_init.deserialize()?;
    let sender = SenderInitializer::init(&mut state.rng, state.delta.clone(), &ot_init);
    let coin_msg = protocol::cointossing::serialize(&state.coin_share)?;
    let msg = serialize(&(sender.1.serialize(), coin_msg))?;
    let state = OtInitState2 {
        rng: state.rng,
        delta: state.delta,
        r_init: state.r_init,
        s: sender.0,
        coin_share: state.coin_share,
        coin_commitment,
        blocks: state.blocks,
    };
    Ok((state, msg))
}

fn init_ot3(state: OtInitState2, msg: &[u8]) -> StateResult<OtInitState3> {
    let (serialized_ot_init, upstream_coin): (SerializedOtInit, Vec<u8>) = deserialize(msg)?;
    let coin =
        protocol::cointossing::finish(state.coin_share, state.coin_commitment, upstream_coin)?;
    let ot_init = serialized_ot_init.deserialize()?;
    let (r, reply) = state.r_init.recv(&ot_init);
    let reply = reply.serialize();
    let state = OtInitState3 {
        rng: state.rng,
        delta: state.delta,
        s: state.s,
        r,
        coin,
        blocks: state.blocks,
    };
    Ok((state, reply))
}

fn init_ot4(mut state: OtInitState3, msg: Vec<u8>) -> StateResult<OtInitState4> {
    let init_msg = OtInitReply::deserialize(msg)?;
    let s = state.s.recv(&init_msg);

    let mut r = state.r;
    let mut blocks = Vec::new();
    let mut abits = vec![BitShare::default(); state.blocks * BLOCK_SIZE];
    for block_id in 0..state.blocks {
        let mut macs_out = [MacType(0); BLOCK_SIZE];
        let mut ot_out = Box::new([MacType(0); BLOCK_SIZE]);
        let bits: u128 = state.rng.gen();
        r.new_batch(bits, &mut macs_out, &mut ot_out[0..]);

        let abits = &mut abits[block_id * BLOCK_SIZE..];
        for i in 0..BLOCK_SIZE {
            abits[i].mac = MacType(macs_out[i].0);
            abits[i].bit = bits & (1 << i) != 0;
        }
        blocks.push(ot_out.to_vec());
    }
    let reply = serialize(&blocks)?;

    let state = OtInitState4 {
        rng: state.rng,
        delta: state.delta,
        blocks: state.blocks,
        coin: state.coin,
        abits,
        s,
    };
    Ok((state, reply))
}

fn ot_ands1(mut state: OtInitState4, msg: &[u8], circuit: &Circuit) -> StateResult<OtAndsState1> {
    let blocks: Vec<Vec<MacType>> = deserialize(msg)?;
    for (block_id, block) in blocks.into_iter().enumerate() {
        let ot_rx: [MacType; BLOCK_SIZE] = block
            .try_into()
            .map_err(|_| Error::OtBlockDeserializationError)?;
        let ot_rx = Box::new(ot_rx);
        let mut keys_out = [MacType(0); BLOCK_SIZE];
        state.s.send(ot_rx.as_ref(), &mut keys_out);

        let abits = &mut state.abits[block_id * BLOCK_SIZE..];
        for i in 0..BLOCK_SIZE {
            abits[i].key = KeyType(keys_out[i].0);
        }
    }

    // the number of authenticated bits we need for wires
    let n_and_gates = circuit.and_gates();
    let n_wire_abits = circuit.and_gates() + circuit.eval_inputs() + circuit.contrib_inputs();
    let abits_blocks = (n_wire_abits + BLOCK_SIZE - 1) / BLOCK_SIZE;
    assert_eq!(0, (state.blocks - abits_blocks) % 3);

    // split state.abits into 2 parts:
    // 1. the first part is used for the wires which equals the number of input and AND gates
    //    aligned by BLOCK_SIZE
    // 2. the latter part is used for AND triples

    let wire_abits_start = (state.blocks - abits_blocks) * BLOCK_SIZE;
    let wire_abits = &state.abits[wire_abits_start..wire_abits_start + n_wire_abits];

    let num_blocks = (n_and_gates + BLOCK_SIZE - 1) / BLOCK_SIZE * BLOCK_SIZE;
    let random_bits = Vec::<MacType>::with_capacity(num_blocks / BLOCK_SIZE);
    let wire_abits = wire_abits.to_vec();

    let mut triples = state.abits;
    triples.truncate((state.blocks - abits_blocks) * BLOCK_SIZE);

    let mut state = OtAndsState1 {
        rng: state.rng,
        delta: state.delta,
        coin: state.coin,
        and_triples: triples,
        wire_abits: wire_abits.to_vec(),
        and_shares: Default::default(),
        random_bits,
        r_and_rand_key: vec![],
        r_and_rand_hash: vec![],
        r_prime: vec![],
    };

    // Step 1 of `Π_{LaAND}`
    let and_hashes = state.compute_and_ot_data();
    let msg = serialize(&and_hashes)?;

    Ok((state, msg))
}

impl OtAndsState1 {
    fn compute_and_ot_data(&mut self) -> Vec<[MacType; 2]> {
        let and_bits = &self.and_triples[0..];
        let num_blocks = and_bits.len() / BLOCK_SIZE / 3;

        self.random_bits.clear();

        let mut result: Vec<[MacType; 2]> = Vec::with_capacity(num_blocks * BLOCK_SIZE);
        result.resize(num_blocks * BLOCK_SIZE, [MacType(0), MacType(0)]);

        for i in 0..num_blocks {
            let r: u128 = self.rng.gen();
            self.random_bits.push(MacType(r));

            let bits = &and_bits[(i * BLOCK_SIZE * 3)..];
            let y = collect_y_bits(bits);
            compute_leaky_and_hashes(
                &mut result[(i * BLOCK_SIZE)..],
                &self.delta,
                r,
                y.0,
                &keys(bits),
            );
        }

        result
    }

    fn compute_and_shares(
        &self,
        and_hashes: &[[MacType; 2]],
        role: Role,
    ) -> Result<Vec<MacType>, Error> {
        let and_bits = &self.and_triples[0..];
        let num_blocks = and_bits.len() / BLOCK_SIZE / 3;

        if and_hashes.len() != num_blocks * BLOCK_SIZE {
            return Err(InsufficientAndShares);
        }

        let mut result: Vec<MacType> = Vec::with_capacity(num_blocks);
        for i in 0..num_blocks {
            let bits = &and_bits[(i * BLOCK_SIZE * 3)..];
            let (x, y, z) = collect_authenticated_bits(bits);
            let and_bits = derive_and_shares(
                self.random_bits[i].0,
                x.0,
                &macs(bits),
                &and_hashes[(i * BLOCK_SIZE)..],
            );

            let and_share = if let Role::Contributor = role {
                MacType(and_bits.0 ^ (x.0 & y.0) ^ z.0)
            } else {
                and_bits
            };
            result.push(and_share);
        }

        Ok(result)
    }
}

/// Implements sub-protoocol Π_{LaAND} steps 4a+4b (resp. 5a+5b) of WRK17a.
fn compute_u(delta: &Delta, and_bits: &[BitShare]) -> Vec<MacType> {
    let mut msgs = Vec::with_capacity(and_bits.len() / 3);
    for i in (0..and_bits.len()).step_by(3) {
        let BitShare {
            key: k_x1,
            bit: b_x2,
            ..
        } = and_bits[i];
        let BitShare {
            key: k_y1,
            bit: b_y2,
            ..
        } = and_bits[i + 1];
        let BitShare {
            key: k_z1,
            bit: b_z2,
            ..
        } = and_bits[i + 2];
        let t0 = hash_keys(k_x1, k_z1 ^ (if b_z2 { delta.0 } else { 0 }));
        let u0 = t0
            ^ hash_keys(
                k_x1 ^ delta.0,
                k_y1 ^ k_z1 ^ (if b_y2 ^ b_z2 { delta.0 } else { 0 }),
            );
        let t1 = hash_keys(k_x1, k_y1 ^ k_z1 ^ (if b_y2 ^ b_z2 { delta.0 } else { 0 }));
        let u1 = t1 ^ hash_keys(k_x1 ^ delta.0, k_z1 ^ (if b_z2 { delta.0 } else { 0 }));
        let u_for_other_party = if b_x2 { u1 } else { u0 };
        msgs.push(u_for_other_party);
    }
    msgs
}

fn ot_ands3_update_z2_contrib(mut state: OtAndsState2, msg: &[u8]) -> StateResult<OtAndsState3> {
    let and_shares: Vec<MacType> = deserialize(msg)?;
    // Step 3 of `Π_{LaAND}`
    let and_bits = &mut state.and_triples[0..];
    let num_blocks = and_bits.len() / BLOCK_SIZE / 3;

    if and_shares.len() != num_blocks {
        return Err(InsufficientAndShares);
    }

    for block in 0..num_blocks {
        let bits = &mut and_bits[(block * BLOCK_SIZE * 3)..];
        let z_r_bits = and_shares[block];

        for i in 0..K {
            if (z_r_bits.0 & (1 << i)) != 0 {
                bits[i * 3 + 2].key.0 ^= state.delta.0;
            }
        }
    }

    // Step 4/5 (a + b) of `Π_{LaAND}`
    let u = compute_u(&state.delta, &state.and_triples);
    let msg = serialize(&u)?;
    let state = OtAndsState3 {
        rng: state.rng,
        delta: state.delta,
        coin: state.coin,
        and_triples: state.and_triples,
        wire_abits: state.wire_abits,
        r_and_rand_key: state.r_and_rand_key,
        r_and_rand_hash: state.r_and_rand_hash,
        r_prime: state.r_prime,
    };
    Ok((state, msg))
}

fn ot_ands3_update_z2_eval(
    mut state: OtAndsState2,
    msg: &[u8],
) -> Result<(OtAndsState3, (Msg, Msg)), Error> {
    let upstream_ands: Vec<MacType> = deserialize(msg)?;
    // Step 3 of `Π_{LaAND}`
    let and_bits = &mut state.and_triples[0..];
    let num_blocks = and_bits.len() / BLOCK_SIZE / 3;

    if and_bits.len() != num_blocks * BLOCK_SIZE * 3 {
        return Err(InsufficientAndShares);
    }

    let mut result: Vec<MacType> = Vec::with_capacity(num_blocks);
    for block in 0..num_blocks {
        let bits = &mut and_bits[(block * BLOCK_SIZE * 3)..];
        let (x, y, r) = collect_authenticated_bits(bits);
        let z_2 = state.and_shares[block].0 ^ (x.0 & y.0) ^ upstream_ands[block].0;
        let d = MacType(r.0 ^ z_2);

        result.push(d);

        // update z_2 according to new values
        for bit_idx in 0..BLOCK_SIZE {
            bits[bit_idx * 3 + 2].bit ^= (d.0 & (1 << bit_idx)) != 0
        }
    }
    let msg1 = serialize(&result)?;

    // Step 4/5 (a + b) of `Π_{LaAND}`
    let u = compute_u(&state.delta, &state.and_triples);
    let msg2 = serialize(&u)?;

    let state = OtAndsState3 {
        rng: state.rng,
        delta: state.delta,
        coin: state.coin,
        and_triples: state.and_triples,
        wire_abits: state.wire_abits,
        r_and_rand_key: state.r_and_rand_key,
        r_and_rand_hash: state.r_and_rand_hash,
        r_prime: state.r_prime,
    };
    Ok((state, (msg1, msg2)))
}

fn ot_ands4(mut state: OtAndsState3, msg: &[u8]) -> StateResult<OtAndsState4> {
    let u_from_other_party: Vec<MacType> = deserialize(msg)?;
    // Step 4/5 (c + d) of `Π_{LaAND}`
    // implementation of Protoocol Π_{LaAND} steps 4c+4d (resp. 5c+5d) of WRK17a
    let and_bits = &state.and_triples[0..];
    let mut r_and_rand = Vec::with_capacity(and_bits.len() / 3);
    let mut r_and_rand_hashed = Vec::with_capacity(and_bits.len() / 3);
    let mut w = Vec::with_capacity(and_bits.len() / 3);
    for i in (0..and_bits.len()).step_by(3) {
        let BitShare {
            key: k_x1,
            mac: m_x2,
            bit: b_x2,
        } = and_bits[i];
        let BitShare { mac: m_y2, .. } = and_bits[i + 1];
        let BitShare { mac: m_z2, .. } = and_bits[i + 2];

        let r: u128 = state.rng.gen();
        let u = u_from_other_party[i / 3];
        let v0 = hash_keys(m_x2.into(), m_z2.into());
        let v1 = hash_keys(m_x2.into(), KeyType(m_z2.0) ^ m_y2.0);
        let (w_x2_0, w_x2_1) = if b_x2 {
            let w_1_0 = hash_key(k_x1) ^ v1 ^ u.0 ^ r;
            let w_1_1 = hash_key(k_x1 ^ state.delta.0) ^ v0 ^ u.0 ^ r;
            (w_1_0, w_1_1)
        } else {
            let w_0_0 = hash_key(k_x1) ^ v0 ^ r;
            let w_0_1 = hash_key(k_x1 ^ state.delta.0) ^ v1 ^ r;
            (w_0_0, w_0_1)
        };
        // hash r + random key for 'commit & open' eq check
        let rand_for_eq_box_hash = KeyType(state.rng.gen());
        let hash_for_commit = hash_keys(KeyType(r), rand_for_eq_box_hash);
        r_and_rand.push((MacType(r), rand_for_eq_box_hash));
        r_and_rand_hashed.push(hash_for_commit);
        w.push((w_x2_0, w_x2_1));
    }
    let msg = serialize(&(r_and_rand_hashed, w))?;

    state.r_and_rand_key = r_and_rand;

    let state = OtAndsState4 {
        rng: state.rng,
        delta: state.delta,
        coin: state.coin,
        and_triples: state.and_triples,
        wire_abits: state.wire_abits,
        r_and_rand_key: state.r_and_rand_key,
        r_and_rand_hash: state.r_and_rand_hash,
        r_prime: state.r_prime,
    };
    Ok((state, msg))
}

fn ot_ands5(mut state: OtAndsState4, msg: &[u8]) -> StateResult<OtAndsState5> {
    let (r_and_rand_hashed, w_from_other_party): (Vec<MacType>, Vec<(MacType, MacType)>) =
        deserialize(msg)?;

    // Step 4/5 (e) of `Π_{LaAND}`
    let and_bits = &state.and_triples[0..];
    let mut r_prime = Vec::with_capacity(and_bits.len() / 3);
    for i in (0..and_bits.len()).step_by(3) {
        let BitShare {
            key: k_x1,
            mac: m_x2,
            bit: b_x2,
        } = and_bits[i];
        let BitShare {
            key: k_y1,
            bit: b_y2,
            ..
        } = and_bits[i + 1];
        let BitShare {
            key: k_z1,
            bit: b_z2,
            ..
        } = and_bits[i + 2];

        let t0 = hash_keys(k_x1, k_z1 ^ (if b_z2 { state.delta.0 } else { 0 }));
        let t1 = hash_keys(
            k_x1,
            k_y1 ^ k_z1 ^ (if b_y2 ^ b_z2 { state.delta.0 } else { 0 }),
        );
        let t_x2 = if b_x2 { t1 } else { t0 };
        let (w_x1_0, w_x1_1) = w_from_other_party[i / 3];
        let w_x1_x2 = if b_x2 { w_x1_1 } else { w_x1_0 };
        r_prime.push(hash(m_x2) ^ w_x1_x2 ^ t_x2);
    }

    // 'commit' step of F_EQ check
    state.r_and_rand_hash = r_and_rand_hashed;
    state.r_prime = r_prime.clone();

    let msg = serialize(&(r_prime, state.r_and_rand_key.clone()))?;

    let state = OtAndsState5 {
        rng: state.rng,
        delta: state.delta,
        coin: state.coin,
        and_triples: state.and_triples,
        wire_abits: state.wire_abits,
        r_and_rand_key: state.r_and_rand_key,
        r_and_rand_hash: state.r_and_rand_hash,
        r_prime: state.r_prime,
    };
    Ok((state, msg))
}

fn check_hash(
    state: &OtAndsState5,
    r_prime: &Vec<MacType>,
    r_and_rand: &Vec<(MacType, KeyType)>,
) -> Result<(), Error> {
    if r_prime.len() != r_and_rand.len() || r_prime.len() != state.r_and_rand_key.len() {
        return Err(UnexpectedMessageType);
    }

    let mut success = true;

    // 'open' step of F_EQ check
    for (i, (r, rand_key)) in r_and_rand.iter().enumerate() {
        let hashed = hash_keys(KeyType(r.0), KeyType(rand_key.0));
        // check that the hash received previously matches the r + rand received now:
        let hash_ok = state.r_and_rand_hash[i] == hashed;
        // check that the r received now matches own r':
        let r_equal = *r == state.r_prime[i];
        success &= hash_ok & r_equal;
    }
    for (i, r_prime) in r_prime.iter().enumerate() {
        // check that the r' received now from the other party matches own r:
        let r_prime_check = state.r_and_rand_key[i].0 == *r_prime;

        success &= r_prime_check;
    }

    if !success {
        Err(LeakyAndNotEqual)
    } else {
        Ok(())
    }
}

fn compute_hashes_contrib(
    state: &OtAndsState6,
    gate_index: usize,
    output_mask: &WireMask,
    lhs: &WireMask,
    rhs: &WireMask,
    input_mask: &BitShare,
) -> AndTableShare {
    let mut h0 = output_mask.bit.xor(input_mask);
    let mut h1 = h0.xor(&lhs.bit);
    let mut h2 = h0.xor(&rhs.bit);
    let mut h3 = h1.xor(&rhs.bit);
    h3.key.0 ^= state.delta.0;

    let output_label = output_mask.label_0.0;
    h0.key = KeyType(mac(&state.delta, h0.key.0 ^ output_label, h0.bit));
    h1.key = KeyType(mac(&state.delta, h1.key.0 ^ output_label, h1.bit));
    h2.key = KeyType(mac(&state.delta, h2.key.0 ^ output_label, h2.bit));
    h3.key = KeyType(mac(&state.delta, h3.key.0 ^ output_label, h3.bit));

    let l0 = &lhs.label_0;
    let l1 = WireLabel(l0.0 ^ state.delta.0);
    let r0 = &rhs.label_0;
    let r1 = WireLabel(r0.0 ^ state.delta.0);

    let gi_u32 = gate_index as u32;
    [
        h0.xor(&garbling_hash::new(l0, r0, gi_u32, 0)),
        h1.xor(&garbling_hash::new(l0, &r1, gi_u32, 1)),
        h2.xor(&garbling_hash::new(&l1, r0, gi_u32, 2)),
        h3.xor(&garbling_hash::new(&l1, &r1, gi_u32, 3)),
    ]
}

fn mac(delta: &Delta, value: u128, bit: bool) -> u128 {
    value ^ (if bit { delta.0 } else { 0 })
}

/// Implements Step 2 + 3 + 4a of Π_{2pc}.
fn preprocessing_assign_masks(
    abits: Vec<BitShare>,
    rng: &mut ChaCha20Rng,
    delta: &Delta,
    circuit: &Circuit,
) -> Vec<WireMask> {
    let mut masks = vec![WireMask::default(); circuit.gates().len()];

    // assign output masks to each wire
    let mut abit_offset = 0;
    for (idx, gate) in circuit.gates().iter().enumerate() {
        match gate {
            Gate::InContrib | Gate::InEval | Gate::And { .. } => {
                // Step 2 `Π_{2pc}`
                masks[idx].bit = abits[abit_offset];
                abit_offset += 1;
                masks[idx].label_0 = (rng.gen::<u128>()).into();
            }
            _ => {}
        }
    }

    for (idx, gate) in circuit.gates().iter().enumerate() {
        match gate {
            Gate::Xor(input_lhs, input_rhs) => {
                // Step 3 `Π_{2pc}`
                let lhs = &masks[*input_lhs as usize];
                let rhs = &masks[*input_rhs as usize];
                masks[idx as usize] = lhs.xor(rhs);
            }
            Gate::Not(input) => {
                let lhs = &masks[*input as usize];
                masks[idx as usize] = lhs.not(delta);
            }
            _ => {}
        }
    }

    masks
}

/// Collects XOR of authenticated bits relating to input wires to AND gates.
///
/// Returns:
///   - Tuple #1: XOR of authenticated bits of left-hand side input
///   - Tuple #2: like #1 but for right-hand side
fn preprocessing_and_gate_bits(
    circuit: &Circuit,
    masks: &[WireMask],
    and_triples: &[BitShare],
) -> (Vec<bool>, Vec<bool>) {
    let mut lhs_bits = Vec::new();
    let mut rhs_bits = Vec::new();

    let mut ands = 0;
    for gate in circuit.gates() {
        if let Gate::And(input_lhs, input_rhs) = gate {
            lhs_bits.push(masks[*input_lhs as usize].bit.bit ^ and_triples[3 * ands].bit);
            rhs_bits.push(masks[*input_rhs as usize].bit.bit ^ and_triples[3 * ands + 1].bit);
            ands += 1;
        }
    }

    (lhs_bits, rhs_bits)
}

/// Implements the `Π_{aAND}` functionality.
///
/// The protocol "consumes" authenticated AND triples from previous steps and returns a new vector
/// with AND triples for further processing.
///
///   - Function `init`: Starts the process yielding bits (`d'` and `d''` in Π_{aAND} terms) and
///     their respective macs.
///   - Function `finish`: Upon receiving upstream bits, computes the final authenticated AND
///     triples.
impl AndsBucketingState {
    fn init(state: OtAndsState5, circuit: &Circuit) -> StateResult<AndsBucketingState> {
        fn new_permutation(mut rng: ChaCha20Rng, total_abits: usize) -> Vec<u32> {
            let mut permutation = vec![0; total_abits];
            for (i, item) in permutation.iter_mut().enumerate().take(total_abits) {
                *item = i as u32;
            }

            let mut idx: Vec<i32> = vec![0; total_abits];
            rng.fill(&mut idx[0..]);
            let idx = idx;

            for i in (0..total_abits as i32).rev() {
                let idx = idx[i as usize] % (i + 1);
                let idx = if idx < 0 { -idx } else { idx };

                permutation.swap(i as usize, idx as usize);
            }

            permutation
        }

        let bucket_size = bucket_size(circuit);
        let length = circuit.and_gates();

        assert!(state.and_triples.len() >= length * bucket_size);

        let mut bits = vec![false; length * bucket_size];
        let mut macs = vec![MacType::default(); length * bucket_size];

        let permutation = {
            let rng = <ChaCha20Rng as rand::SeedableRng>::from_seed(state.coin);
            new_permutation(rng, length * bucket_size)
        };

        for i in 0..length {
            let lhs = permutation[i * bucket_size] as usize;

            for j in 1..bucket_size {
                let rhs = permutation[i * bucket_size + j] as usize;

                let d = state.and_triples[lhs * 3 + 1].xor(&state.and_triples[rhs * 3 + 1]);
                bits[i * bucket_size + j] = d.bit;
                macs[i * bucket_size + j] = d.mac;
            }
        }

        let msg = serialize(&(&bits, macs))?;

        let state = AndsBucketingState {
            rng: state.rng,
            delta: state.delta,
            bucketing_bits: bits,
            wire_abits: state.wire_abits,
            permutation,
            and_triples: state.and_triples,
            length,
            bucket_size,
        };

        Ok((state, msg))
    }

    fn finish(self, msg: &[u8], circuit: &Circuit) -> StateResult<OtAndsState6> {
        let mut state = self.update_triples(msg)?;

        let wire_abits = state.wire_abits;
        let masks = preprocessing_assign_masks(wire_abits, &mut state.rng, &state.delta, circuit);
        let (lhs_and_bits, rhs_and_bits) =
            preprocessing_and_gate_bits(circuit, &masks, &state.and_triples);
        let msg = serialize(&(&lhs_and_bits, &rhs_and_bits))?;

        let state = OtAndsState6 {
            delta: state.delta,
            and_triples: state.and_triples,
            masks,
            lhs_and_bits,
            rhs_and_bits,
        };

        Ok((state, msg))
    }

    /// Implements sub-protocol `Π_{aAND}` Step 3.a (checking step), and 3.b.
    fn update_triples(self, msg: &[u8]) -> Result<AndsBucketingState, Error> {
        assert!(self.bucketing_bits.len() == self.length * self.bucket_size);

        let (upstream_bits, upstream_macs): (Vec<bool>, Vec<MacType>) = deserialize(msg)?;
        if upstream_bits.len() != self.bucketing_bits.len()
            || upstream_macs.len() != self.bucketing_bits.len()
        {
            return Err(Error::InsufficientInput);
        }

        let perm = &self.permutation;

        // checking step of `Π_{aAND}` Step 3.a
        {
            let mut checks_succeeded = 0;
            for i in 0..self.length {
                let lhs = perm[i * self.bucket_size] as usize;

                for j in 1..self.bucket_size {
                    let rhs = perm[i * self.bucket_size + j] as usize;
                    let d = self.and_triples[lhs * 3 + 1].xor(&self.and_triples[rhs * 3 + 1]);
                    let upstream_bs = PartialBitShare {
                        bit: upstream_bits[i * self.bucket_size + j],
                        mac: upstream_macs[i * self.bucket_size + j],
                    };

                    checks_succeeded += upstream_bs.verify(&d.key, &self.delta) as usize;
                }
            }
            if checks_succeeded != self.length * (self.bucket_size - 1) {
                return Err(Error::MacError);
            }
        }

        // checking step of `Π_{aAND}` Step 3.b.
        // Difference between this implementation of `Π_{aAND}` of WRK17a: in the paper,
        // the authors perform pairwise XOR'ing of authenticated bit shares with the MAC check.
        // here we perform this in a looped manner, such that there is only a single communication
        // and MAC check step performed. This is also analogous to the emp toolkit's implementation.
        let mut bucketing_bits = self.bucketing_bits;
        for i in 0..bucketing_bits.len() {
            bucketing_bits[i] ^= upstream_bits[i];
        }
        let bucketing_bits = bucketing_bits;

        let mut and_triples = vec![BitShare::default(); self.length * 3];
        for i in 0..self.length {
            for j in 0..3 {
                let idx = perm[i * self.bucket_size] as usize;
                and_triples[i * 3 + j] = self.and_triples[idx * 3 + j];
            }

            for j in 1..self.bucket_size {
                let idx = perm[i * self.bucket_size + j] as usize;
                and_triples[i * 3] = and_triples[i * 3].xor(&self.and_triples[idx * 3]);
                and_triples[i * 3 + 2] = and_triples[i * 3 + 2].xor(&self.and_triples[idx * 3 + 2]);

                if bucketing_bits[i * self.bucket_size + j] {
                    and_triples[i * 3 + 2] = and_triples[i * 3 + 2].xor(&self.and_triples[idx * 3])
                }
            }
        }

        Ok(AndsBucketingState {
            and_triples,
            bucketing_bits: Default::default(),
            ..self
        })
    }
}

fn ot_ands6(state: OtAndsState5, msg: &[u8], circuit: &Circuit) -> StateResult<AndsBucketingState> {
    // 2nd part of Step 4e/5e of `Π_{LaAND}`
    let (r_prime, r_and_rand): (Vec<MacType>, Vec<(MacType, KeyType)>) = deserialize(msg)?;
    check_hash(&state, &r_prime, &r_and_rand)?;

    AndsBucketingState::init(state, circuit)
}

fn ot_ands8_contrib(
    mut state: OtAndsState6,
    msg1: &[u8],
    circuit: &Circuit,
    input: &[bool],
) -> StateResult<InputProcContrib> {
    let (x2, y2): (Vec<bool>, Vec<bool>) = deserialize(msg1)?;
    if state.lhs_and_bits.len() != x2.len()
        || state.rhs_and_bits.len() != y2.len()
        || state.lhs_and_bits.len() != state.rhs_and_bits.len()
    {
        return Err(Error::InsufficientAndShares);
    }

    for i in 0..state.lhs_and_bits.len() {
        state.lhs_and_bits[i] ^= x2[i];
        state.rhs_and_bits[i] ^= y2[i];
    }

    let masks = &state.masks;
    let mut ands = 0_usize;
    let mut garbled_table_shares = Vec::new();

    for (index, gate) in circuit.gates().iter().enumerate() {
        if let Gate::And(input_lhs, input_rhs) = gate {
            let input_mask = &state.sigma_mac(ands, Role::Contributor);
            ands += 1;

            let values = compute_hashes_contrib(
                &state,
                index,
                &masks[index],
                &masks[*input_lhs as usize],
                &masks[*input_rhs as usize],
                input_mask,
            );
            garbled_table_shares.push((index as u32, values));
        }
    }

    let pending_from_a = circuit.contrib_inputs();
    let pending_from_b = circuit.eval_inputs();

    if pending_from_a > input.len() {
        return Err(InsufficientInput);
    }
    if pending_from_a + pending_from_b == 0 {
        return Err(InvalidCircuit);
    }

    // generate message for each input bit and continue
    let mut input_mask_shares = Vec::with_capacity(pending_from_b);
    for (index, gate) in circuit.gates().iter().enumerate() {
        if gate == &Gate::InEval {
            input_mask_shares.push((
                index as GateIndex,
                PartialBitShare {
                    bit: masks[index].bit.bit,
                    mac: masks[index].bit.mac,
                },
            ))
        }
    }
    let msg = serialize(&(garbled_table_shares, input_mask_shares))?;

    let state = InputProcContrib {
        delta: state.delta,
        pending_from_b,
        mac_checks_success: true,
        masks: state.masks,
    };
    Ok((state, msg))
}

impl OtAndsState6 {
    fn sigma_mac(&self, ands: usize, role: Role) -> BitShare {
        let mut res = self.and_triples[3 * ands + 2];
        if self.lhs_and_bits[ands] {
            res = res.xor(&self.and_triples[3 * ands + 1]);
        }
        if self.rhs_and_bits[ands] {
            res = res.xor(&self.and_triples[3 * ands]);
        }

        if self.lhs_and_bits[ands] && self.rhs_and_bits[ands] {
            if let Role::Contributor = role {
                res.key.0 ^= self.delta.0; // TODO: is this correct?
            } else {
                res.bit = !res.bit;
            }
        }

        res
    }
}

fn ot_ands8_eval(
    mut state: OtAndsState6,
    msg1: &[u8],
    msg2: &[u8],
    circuit: &Circuit,
    input: &[bool],
) -> StateResult<InputProcEval> {
    let (upstream_lhs_bits, upstream_rhs_bits): (Vec<bool>, Vec<bool>) = deserialize(msg1)?;

    for i in 0..state.lhs_and_bits.len() {
        state.lhs_and_bits[i] ^= upstream_lhs_bits[i];
        state.rhs_and_bits[i] ^= upstream_rhs_bits[i];
    }

    let mut ands = 0_usize;
    let mut wires = vec![WireState::default(); circuit.gates().len()];
    for (index, gate) in circuit.gates().iter().enumerate() {
        if let Gate::And(input_lhs, input_rhs) = gate {
            let input_mask = &state.sigma_mac(ands, Role::Evaluator);
            ands += 1;

            wires[index].my_and_table = compute_hashes(
                &state.masks[index],
                &state.masks[*input_lhs as usize],
                &state.masks[*input_rhs as usize],
                input_mask,
            );
        }
    }

    // input processing:
    let (garbled_table_shares, input_mask_shares): (Vec<TableShare>, Vec<InputMaskShare>) =
        deserialize(msg2)?;
    if ands != garbled_table_shares.len() {
        return Err(UnexpectedGarbledTableShare);
    }
    for (gate, and_share) in garbled_table_shares {
        if !circuit.gates()[gate as usize].is_and() {
            return Err(UnexpectedGarbledTableShare);
        }
        wires[gate as usize].other_and_table = and_share;
    }

    if circuit.eval_inputs() > input.len() {
        return Err(InsufficientInput);
    }

    // generate message for each input bit and continue
    let mut mask_shares = Vec::new();
    for (index, gate) in circuit.gates().iter().enumerate() {
        if gate == &Gate::InContrib {
            mask_shares.push((
                index as GateIndex,
                PartialBitShare {
                    bit: state.masks[index].bit.bit,
                    mac: state.masks[index].bit.mac,
                },
            ))
        }
    }

    let mut masked_inputs = Vec::with_capacity(input_mask_shares.len());
    for ((index, bit_share), input) in input_mask_shares.iter().zip(input.iter()) {
        if circuit.gates()[*index as usize] != Gate::InEval {
            return Err(UnexpectedMessageType);
        }

        let mask = &state.masks[*index as usize];
        assert!(bit_share.verify(&mask.bit.key, &state.delta));

        let masked_input = mask.bit.bit ^ bit_share.bit ^ input;
        masked_inputs.push((*index, masked_input));
    }

    let (garbled_table_shares, input_mask_shares): (Vec<TableShare>, Vec<InputMaskShare>) =
        deserialize(msg2)?;
    if ands != garbled_table_shares.len() {
        return Err(UnexpectedGarbledTableShare);
    }
    for (gate, and_share) in garbled_table_shares {
        if !circuit.gates()[gate as usize].is_and() {
            return Err(UnexpectedGarbledTableShare);
        }
        wires[gate as usize].other_and_table = and_share;
    }

    let input_gates = circuit
        .gates()
        .iter()
        .filter(|g| *g == &Gate::InEval)
        .count();
    if input_gates > input.len() {
        return Err(InsufficientInput);
    }

    // generate message for each input bit and continue
    let mut mask_shares = Vec::new();
    for (index, gate) in circuit.gates().iter().enumerate() {
        if gate == &Gate::InContrib {
            mask_shares.push((
                index as GateIndex,
                PartialBitShare {
                    bit: state.masks[index].bit.bit,
                    mac: state.masks[index].bit.mac,
                },
            ))
        }
    }

    let mut masked_inputs = Vec::with_capacity(input_mask_shares.len());
    for ((index, bit_share), input) in input_mask_shares.iter().zip(input.iter()) {
        if circuit.gates()[*index as usize] != Gate::InEval {
            return Err(UnexpectedMessageType);
        }

        let mask = &state.masks[*index as usize];
        assert!(bit_share.verify(&mask.bit.key, &state.delta));

        let masked_input = mask.bit.bit ^ bit_share.bit ^ input;
        masked_inputs.push((*index, masked_input));
    }
    let reply = serialize(&(mask_shares, masked_inputs))?;
    let state = InputProcEval {
        delta: state.delta,
        pending_input: circuit.eval_inputs() + circuit.contrib_inputs(),
        masks: state.masks,
        wires,
    };

    Ok((state, reply))
}

#[allow(clippy::identity_op)]
fn collect_authenticated_bits(bits: &[BitShare]) -> (MacType, MacType, MacType) {
    use crate::types::SecurityBits;

    assert!(!bits.len() >= BLOCK_SIZE * 3);

    let mut x = 0;
    let mut y = 0;
    let mut z = 0;

    for i in 0..BLOCK_SIZE {
        x |= SecurityBits::from(bits[i * 3 + 0].bit) << i;
        y |= SecurityBits::from(bits[i * 3 + 1].bit) << i;
        z |= SecurityBits::from(bits[i * 3 + 2].bit) << i;
    }

    (MacType(x), MacType(y), MacType(z))
}

fn collect_y_bits(bits: &[BitShare]) -> MacType {
    use crate::types::SecurityBits;

    assert!(!bits.len() >= BLOCK_SIZE * 3);

    let mut result = 0;

    for i in 0..BLOCK_SIZE {
        result |= SecurityBits::from(bits[i * 3 + 1].bit) << i;
    }

    MacType(result)
}

#[inline]
fn keys(bits: &[BitShare]) -> [KeyType; BLOCK_SIZE] {
    assert!(bits.len() >= BLOCK_SIZE * 3);

    let mut r = [KeyType(0); BLOCK_SIZE];
    for i in 0..BLOCK_SIZE {
        r[i] = bits[3 * i].key;
    }

    r
}

#[inline]
fn macs(bits: &[BitShare]) -> [MacType; BLOCK_SIZE] {
    assert!(bits.len() >= BLOCK_SIZE * 3);

    let mut r = [MacType(0); BLOCK_SIZE];
    for i in 0..BLOCK_SIZE {
        r[i] = bits[3 * i].mac;
    }

    r
}

fn compute_hashes(
    output_mask: &WireMask,
    lhs: &WireMask,
    rhs: &WireMask,
    input_mask: &BitShare,
) -> [BitShare; 4] {
    let h0 = output_mask.bit.xor(input_mask);
    let h1 = h0.xor(&lhs.bit);
    let h2 = h0.xor(&rhs.bit);
    let mut h3 = h1.xor(&rhs.bit);
    h3.bit ^= true;
    [h0, h1, h2, h3]
}

impl InputProcContrib {
    fn run(mut self, msg: &[u8], circuit: &Circuit, input: &[bool]) -> TandemResult<()> {
        // P_B sends its mask to P_A which then returns masked input plus label to P_B for final
        // circuit evaluation
        let (shares, inputs): (Vec<InputMaskShare>, Vec<(u32, bool)>) = deserialize(msg)?;
        let mut evaluation_inputs = Vec::with_capacity(shares.len());
        for ((index, bit_share), input) in shares.iter().zip(input.iter()) {
            if circuit.gates()[*index as usize] != Gate::InContrib {
                return Err(UnexpectedMessageType);
            }
            let mask = &self.masks[*index as usize];

            self.mac_checks_success &= bit_share.verify(&mask.bit.key, &self.delta);
            let my_input_masked = input ^ bit_share.bit ^ mask.bit.bit;
            let label = mask.label(my_input_masked, &self.delta);
            evaluation_inputs.push((*index, label, my_input_masked));
        }

        // P_B sends masked bit to P_A so P_A can return its label
        for (index, bit) in inputs {
            if circuit.gates()[index as usize] != Gate::InEval {
                return Err(UnexpectedMessageType);
            }
            if self.pending_from_b == 0 {
                return Err(UnexpectedMessageType);
            }

            let mask = &self.masks[index as usize];
            let label = mask.label(bit, &self.delta);
            evaluation_inputs.push((index, label, bit));
        }

        if self.mac_checks_success {
            // disclose masks of output gates to other party
            let mut mask_shares = Vec::new();
            for index in circuit.output_gates() {
                mask_shares.push((
                    *index,
                    PartialBitShare {
                        mac: self.masks[*index as usize].bit.mac,
                        bit: self.masks[*index as usize].bit.bit,
                    },
                ));
            }
            let reply = serialize(&(evaluation_inputs, mask_shares))?;
            Ok(((), reply))
        } else {
            Err(MacError)
        }
    }
}

impl InputProcEval {
    fn run(mut self, msg: &[u8], circuit: &Circuit) -> TandemResult<Vec<bool>> {
        let (inputs, shares): (Vec<(u32, WireLabel, bool)>, Vec<InputMaskShare>) =
            deserialize(msg)?;
        for (index, label, masked_value) in inputs {
            if circuit.gates()[index as usize] != Gate::InEval
                && circuit.gates()[index as usize] != Gate::InContrib
            {
                return Err(UnexpectedMessageType);
            }
            if self.pending_input == 0 {
                return Err(UnexpectedMessageType);
            }

            self.wires[index as usize].label = label.clone();
            self.wires[index as usize].masked_value = masked_value;

            self.pending_input -= 1;
        }

        assert_eq!(self.pending_input, 0);
        let mut wires = self.wires;
        let mut mac_checks_success = true;
        for (index, gate) in circuit.gates().iter().enumerate() {
            if let Gate::Xor(input_lhs, input_rhs) = gate {
                wires[index].masked_value = wires[*input_lhs as usize].masked_value
                    ^ wires[*input_rhs as usize].masked_value;
                wires[index].label = wires[*input_lhs as usize]
                    .label
                    .xor(&wires[*input_rhs as usize].label);
            } else if let Gate::Not(input) = gate {
                wires[index].masked_value = !wires[*input as usize].masked_value;
                wires[index].label = wires[*input as usize].label.clone();
            } else if let Gate::And(input_lhs, input_rhs) = gate {
                let lhs = &wires[*input_lhs as usize];
                let rhs = &wires[*input_rhs as usize];

                let row: u8 = 2 * u8::from(lhs.masked_value) + u8::from(rhs.masked_value);
                let result = wires[index].other_and_table[row as usize].xor(&garbling_hash::new(
                    &lhs.label,
                    &rhs.label,
                    index as u32,
                    row,
                ));

                mac_checks_success &= PartialBitShare::from(&result)
                    .verify(&wires[index].my_and_table[row as usize].key, &self.delta);

                wires[index].masked_value =
                    wires[index].my_and_table[row as usize].bit ^ result.bit;
                wires[index].label =
                    WireLabel(result.key.0 ^ wires[index].my_and_table[row as usize].mac.0);
            }
        }
        if !mac_checks_success {
            return Err(MacError);
        }

        let mut output = Vec::with_capacity(circuit.output_gates().len());
        if circuit.output_gates().len() != shares.len() {
            return Err(UnexpectedMessageType);
        }
        for (index, bit_share) in shares {
            mac_checks_success &=
                bit_share.verify(&self.masks[index as usize].bit.key, &self.delta);

            let result = wires[index as usize].masked_value
                ^ bit_share.bit
                ^ self.masks[index as usize].bit.bit;

            output.push(result);
        }
        if mac_checks_success {
            let empty_reply = vec![];
            Ok((output, empty_reply))
        } else {
            Err(MacError)
        }
    }
}
