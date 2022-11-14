//! SMPC engine simulation environment under ideal functionality

use crate::{
    states::{Contributor, Evaluator},
    Circuit, Error,
};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

/// Simulates the local execution of the circuit using a 2 Party MPC protocol.
///
/// The Multi-Party Computation is performed using the full cryptographic protocol exposed by the
/// [`Contributor`] and [`Evaluator`]. The messages between contributor and evaluator are exchanged
/// using local message queues. This function thus simulates an MPC execution on a local machine
/// under ideal network conditions, without any latency or bandwidth restrictions.
pub fn simulate(
    circuit: &Circuit,
    input_contributor: &[bool],
    input_evaluator: &[bool],
) -> Result<Vec<bool>, Error> {
    let mut eval = Evaluator::new(
        circuit.clone(),
        input_evaluator,
        ChaCha20Rng::from_entropy(),
    )?;
    let (mut contrib, mut msg_for_eval) =
        Contributor::new(circuit, input_contributor, ChaCha20Rng::from_entropy())?;

    assert_eq!(contrib.steps(), eval.steps());

    for _ in 0..eval.steps() {
        let (next_state, msg_for_contrib) = eval.run(&msg_for_eval)?;
        eval = next_state;

        let (next_state, reply) = contrib.run(&msg_for_contrib)?;
        contrib = next_state;

        msg_for_eval = reply;
    }
    eval.output(&msg_for_eval)
}
