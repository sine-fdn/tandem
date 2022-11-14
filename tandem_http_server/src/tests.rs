#![allow(dead_code)]

use crate::{
    build,
    msg_queue::{MessageId, MsgQueue},
    requests::NewSession,
    types::{EngineCreationResult, MessageLog, MpcSession},
    MpcRequest,
};
use std::collections::HashMap;

use crate::engine;

use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use rocket::{
    http::Status,
    local::blocking::{Client, LocalResponse},
};
use tandem::{
    states::{Evaluator, Msg},
    Circuit,
};
use tandem_garble_interop::{
    check_program, compile_program, deserialize_output, serialize_input, Role, TypedCircuit,
};

#[launch]
pub fn _rocket() -> _ {
    let handler = |r: MpcRequest| -> Result<MpcSession, String> {
        let prg = check_program(&r.program)?;
        let circuit = compile_program(&prg, &r.function)?;
        let headers = HashMap::new();
        let input = serialize_input(
            Role::Contributor,
            &prg,
            &circuit.fn_def,
            &r.plaintext_metadata,
        )?;
        Ok(MpcSession {
            circuit: circuit.gates,
            input_from_server: input,
            request_headers: headers,
        })
    };
    build(Box::new(handler))
}

#[test]
fn test_multiple_engines() {
    let client = &Client::tracked(_rocket()).unwrap();

    let r1 = new_session(client, xor_and_program(), "false".to_string());
    assert_eq!(r1.status(), Status::Created);

    let r2 = new_session(client, xor_and_program(), "false".to_string());
    assert_eq!(r2.status(), Status::Created);

    assert_ne!(
        r1.into_json::<EngineCreationResult>(),
        r2.into_json::<EngineCreationResult>(),
    );
}

#[test]
fn test_delete_session() {
    let client = &Client::tracked(_rocket()).unwrap();

    let r1 = new_session(client, xor_and_program(), "false".to_string());
    assert_eq!(r1.status(), Status::Created);

    let EngineCreationResult { engine_id, .. } = r1.into_json().unwrap();
    let r3 = delete_session(client, &engine_id);
    assert_eq!(r3.status(), Status::Ok);

    let r4 = new_session(client, xor_and_program(), "false".to_string());
    assert_eq!(r4.status(), Status::Created);
}

#[test]
fn test_protocol_xor_and() {
    let client = &Client::tracked(_rocket()).unwrap();
    let program = xor_and_program();

    for input_party_a in [false, true] {
        for input_party_b in [false, true] {
            let r1 = new_session(client, program.clone(), input_party_a.to_string());
            assert_eq!(r1.status(), Status::Created);

            let EngineCreationResult { engine_id, .. } = r1.into_json().unwrap();
            let prg = check_program(&program).unwrap();
            let TypedCircuit { gates, fn_def, .. } = compile_program(&prg, "main").unwrap();
            let result = tandem_http_protocol(client, &engine_id, gates, vec![input_party_b]);
            let result = deserialize_output(&prg, &fn_def, &result)
                .unwrap()
                .as_bits(&prg);
            println!("{input_party_a}, {input_party_b} -> {result:?}");
            assert_eq!(
                result,
                vec![input_party_a ^ input_party_b, input_party_a & input_party_b]
            );
        }
    }

    // create engine session
}

/// runs protocol with upstream
///
/// assumes upstream session was already created
fn tandem_http_protocol(
    client: &Client,
    engine_id: &String,
    program: Circuit,
    input: Vec<bool>,
) -> Vec<bool> {
    let mut context = MsgQueue::new();
    let mut evaluator = Evaluator::new(program, input, ChaCha20Rng::from_entropy()).unwrap();

    let mut last_durably_received_offset: Option<MessageId> = None;
    let mut steps_remaining = evaluator.steps();
    loop {
        let messages: Vec<(&Msg, MessageId)> = context.msgs_iter().collect();
        let (upstream_msgs, server_commited_offset) =
            dialog(client, engine_id, last_durably_received_offset, &messages);
        assert_eq!(messages.last().map(|v| v.1), server_commited_offset);

        if let Some(last_durably_received_offset) = server_commited_offset {
            context.flush_queue(last_durably_received_offset);
        }

        for (msg, server_offset) in &upstream_msgs {
            assert_eq!(
                *server_offset,
                last_durably_received_offset.map(|o| o + 1).unwrap_or(0)
            );

            if steps_remaining > 0 {
                let (next_state, msg) = evaluator.run(msg).unwrap();
                evaluator = next_state;
                steps_remaining -= 1;
                context.send(msg);
            } else {
                return evaluator.output(msg).unwrap();
            }
            last_durably_received_offset = Some(*server_offset);
        }
    }
}

fn dialog<'a>(
    client: &'a Client,
    engine_id: &String,
    last_durably_received_offset: Option<u32>,
    messages: &Vec<(&Msg, MessageId)>,
) -> (MessageLog, Option<MessageId>) {
    let dialog_uri = uri!(engine::dialog(engine_id, last_durably_received_offset));
    let messages = bincode::serialize(messages).unwrap();
    let res = client.post(dialog_uri).body(messages).dispatch();
    assert_eq!(res.status(), Status::Ok);

    bincode::deserialize(&res.into_bytes().unwrap()).unwrap()
}

fn new_session<'a>(client: &'a Client, program: String, input: String) -> LocalResponse<'a> {
    let prg = check_program(&program).unwrap();
    let circuit = compile_program(&prg, "main").unwrap();
    let create_sess_uri = uri!(engine::create_session());
    let session = NewSession {
        plaintext_metadata: input,
        program,
        function: "main".to_string(),
        circuit_hash: circuit.gates.blake3_hash(),
    };
    client.post(create_sess_uri).json(&session).dispatch()
}

fn xor_and_program() -> String {
    "pub fn main(a: bool, b: bool) -> (bool, bool) { (a ^ b, a & b) }".to_string()
}

fn delete_session<'a>(client: &'a Client, engine_id: &String) -> LocalResponse<'a> {
    let delete_sess_uri = uri!(engine::delete_session(engine_id));
    client.delete(delete_sess_uri).dispatch()
}
