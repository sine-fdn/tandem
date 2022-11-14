# Tandem

Tandem is an SMPC engine implementing the WRK17[^1] protocol as a non-blocking FSM, with an OT extension and optimizations based on ALSZ13[^2] and a base OT implementation based on ABKLX21[^3].

For the time being, Tandem only supports __two-party__ computation.

__Please note:__ 

The present is still an __alpha__ release. Caution is therefore recommended when using it. Although the engine is secure, no highly-sensitive data should yet depend on it.

If you find any bugs, encounter unexpected behavior or have suggestions on how to improve Tandem, please let us know by opening an issue.

## High-Level Description of the Engine

The following is a very high-level description of how the Tandem engine works. For a more detailed approach, please refer to the code-level documentation, starting [here](./src/lib.rs).

The protocol implemented by the Tandem engine rests on the exchange of encrypted messages between two parties: the [`Contributor` and the `Evaluator`](./src/states.rs). The `Contributor` provides an input to the MPC protocol and always sends the first message. The `Evaluator` evaluates the circuit (together with its input) and decrypts the output.

Both `Contributor` and `Evaluator` need three arguments to be initialized: a [Garbled Circuit](./src/circuit.rs), input gates (an array of `bool`s) and an RNG (in this case `ChaCha20Rng`). When the `Contributor` is initialized, its original state is determined and an encrypted message is generated. The `Evaluator` is initialized with an original state but no message. Rather, it awaits a message from the `Contributor`.

The protocol starts when the `Contributor` sends its initial message to the `Evaluator`. Based on the received message, the `Evaluator` sends another encrypted message to the `Contributor` and transitions into a new state. Receiving the message from the `Evaluator`, the `Contributor` sends a new message and transitions into a new state. This back-and-forth communication takes place a total of six times. When the final message is received by the `Evaluator`, the output is decrypted and the protocol ends.



[^1]: [Wang, Ranellucci, and Katz (2017)](https://acmccs.github.io/papers/p21-wangA.pdf).
[^2]: [Asharov, Lindell, Schneider, and Zohner (2013)](https://eprint.iacr.org/2013/552.pdf)
[^3]: [Abdalla, Barbosa, Katz, Loss, and Xu (2021)](https://eprint.iacr.org/2021/1218.pdf)
