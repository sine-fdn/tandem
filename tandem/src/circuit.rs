use blake3::Hasher;

use crate::Error;

/// The index of the gate in the circuit, representing its output wire.
pub type GateIndex = u32;

/// A circuit of AND, XOR and NOT gates that can be executed using MPC.
#[derive(Clone, Debug)]
pub struct Circuit {
    /// A collection of connected gates, each implicitly identified by its index in the vector.
    gates: Vec<Gate>,
    /// The output wires of the gates that are exposed as outputs of the whole circuit.
    output_gates: Vec<GateIndex>,

    /// total number of AND gantes in the circuit
    and_gates: usize,
    /// number of evaluator input bits
    eval_inputs: usize,
    /// number of contributor input bits
    contrib_inputs: usize,
}

/// A blake3 hash that can be used to compare circuits for equality.
pub type CircuitBlake3Hash = [u8; 32];

const MAX_GATES: usize = (u32::MAX >> 4) as usize;
const MAX_AND_GATES: usize = (u32::MAX >> 8) as usize;

impl Circuit {
    /// the gates of the circuit
    pub fn gates(&self) -> &Vec<Gate> {
        &self.gates
    }
    /// indexes of the gates that are exposed as outputs of the circuit
    pub fn output_gates(&self) -> &Vec<GateIndex> {
        &self.output_gates
    }
    /// number of and gates in the circuit
    pub fn and_gates(&self) -> usize {
        self.and_gates
    }
    /// number of input bits by the evaluator party
    pub fn eval_inputs(&self) -> usize {
        self.eval_inputs
    }
    /// number of input bits by the contributor party
    pub fn contrib_inputs(&self) -> usize {
        self.contrib_inputs
    }

    /// create new circuit from a collection of gates and a collection of output gate indexes
    pub fn new(gates: Vec<Gate>, output_gates: Vec<GateIndex>) -> Self {
        let mut and_gates = 0;
        let mut eval_inputs = 0;
        let mut contrib_inputs = 0;

        for gate in &gates {
            match gate {
                Gate::And(_, _) => and_gates += 1,
                Gate::InEval => eval_inputs += 1,
                Gate::InContrib => contrib_inputs += 1,
                _ => {}
            }
        }

        Self {
            gates,
            output_gates,
            and_gates,
            eval_inputs,
            contrib_inputs,
        }
    }

    /// Calculates the blake3 hash of the circuit.
    pub fn blake3_hash(&self) -> CircuitBlake3Hash {
        let mut hasher = blake3::Hasher::new();
        for gate in self.gates.iter() {
            gate.update_hash(&mut hasher);
        }
        for output_gate in self.output_gates.iter() {
            hasher.update(&output_gate.to_be_bytes());
        }
        *hasher.finalize().as_bytes()
    }

    /// Performs a syntax check of the circuit.
    ///
    /// A circuit is invalid if any of the following is true:
    ///   - it contains cycles (by referring to a wire larger than its own index)
    ///   - it does not contain any output gates
    ///   - the output gate indexes do not occur in the circuit
    ///   - the number of gates exceeds the maximum number supported
    ///   - the number of AND gates exceeds the maximum number supported
    pub fn validate(&self) -> Result<(), Error> {
        let mut num_and_gates = 0;
        for (i, g) in self.gates.iter().enumerate() {
            let i = i as u32;
            match g {
                Gate::InContrib | Gate::InEval => {}
                &Gate::Xor(x, y) => {
                    if x >= i || y >= i {
                        return Err(Error::InvalidCircuit);
                    }
                }
                &Gate::And(x, y) => {
                    if x >= i || y >= i {
                        return Err(Error::InvalidCircuit);
                    }
                    num_and_gates += 1;
                }
                &Gate::Not(x) => {
                    if x >= i {
                        return Err(Error::InvalidCircuit);
                    }
                }
            }
        }
        if self.output_gates.is_empty() {
            return Err(Error::InvalidCircuit);
        }
        for &o in self.output_gates.iter() {
            if o >= self.gates.len() as u32 {
                return Err(Error::InvalidCircuit);
            }
        }
        if num_and_gates > MAX_AND_GATES {
            return Err(Error::MaxCircuitSizeExceeded);
        }
        if self.gates.len() > MAX_GATES {
            return Err(Error::MaxCircuitSizeExceeded);
        }
        Ok(())
    }

    pub(crate) fn validate_contributor_input(&self, input: &[bool]) -> Result<(), Error> {
        if self
            .gates
            .iter()
            .filter(|g| matches!(g, Gate::InContrib))
            .count()
            == input.len()
        {
            Ok(())
        } else {
            Err(Error::InsufficientInput)
        }
    }

    pub(crate) fn validate_evaluator_input(&self, input: &[bool]) -> Result<(), Error> {
        if self
            .gates
            .iter()
            .filter(|g| matches!(g, Gate::InEval))
            .count()
            == input.len()
        {
            Ok(())
        } else {
            Err(Error::InsufficientInput)
        }
    }
}

/// A single gate in a larger [`Circuit`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Gate {
    /// A single input bit coming from the circuit contributor.
    InContrib,
    /// A single input bit coming from the circuit evaluator.
    InEval,
    /// A gate computing the XOR of the two specified gates.
    Xor(GateIndex, GateIndex),
    /// A gate computing the AND of the two specified gates.
    And(GateIndex, GateIndex),
    /// A gate computing the NOT of the specified gate.
    Not(GateIndex),
}

impl Gate {
    #[inline]
    pub(crate) fn is_and(&self) -> bool {
        matches!(self, Gate::And { .. })
    }

    pub(crate) fn update_hash(&self, hasher: &mut Hasher) {
        let type_byte = match self {
            Gate::InContrib => 0,
            Gate::InEval => 1,
            Gate::Xor(x, y) => {
                hasher.update(&x.to_be_bytes());
                hasher.update(&y.to_be_bytes());
                2
            }
            Gate::And(x, y) => {
                hasher.update(&x.to_be_bytes());
                hasher.update(&y.to_be_bytes());
                3
            }
            Gate::Not(x) => {
                hasher.update(&x.to_be_bytes());
                4
            }
        };
        hasher.update(&[type_byte]);
    }
}
