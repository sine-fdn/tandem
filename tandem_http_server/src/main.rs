use std::{collections::HashMap, fs::read_to_string, path::Path};

use figment::{
    providers::{Env, Format, Json, Toml},
    Figment,
};
use serde::Deserialize;
use tandem_garble_interop::{check_program, compile_program, serialize_input, Role};
use tandem_http_server::{build, MpcRequest, MpcSession};

use std::{env, iter::zip};

#[macro_use]
extern crate rocket;

type ProgramFilePath = String;
type ProgramFnName = String;
type PlaintextMetadata = String;
type OwnInput = String;

#[derive(Debug, Clone, Deserialize)]
struct HandlerConfig {
    handlers: HashMap<ProgramFnName, HashMap<PlaintextMetadata, OwnInput>>,
}

#[launch]
fn rocket() -> _ {
    println!(
        "Starting server in {}...",
        env::current_dir().unwrap().display().to_string()
    );

    let default = HashMap::<ProgramFilePath, HashMap<PlaintextMetadata, OwnInput>>::new();
    let config: HandlerConfig = Figment::from(("handlers", default))
        .merge(Json::file("Tandem.json"))
        .merge(Toml::file("Tandem.toml"))
        .merge(Env::prefixed("TANDEM_"))
        .extract()
        .unwrap();

    let mut request_headers = HashMap::new();

    // fly.io specific logic to allow reconnecting to the same instance:
    set_fly_instance_id(&mut request_headers);

    if config.handlers.is_empty() {
        println!("No configured handlers, starting simple echo server instead...");
        let handler = move |r: MpcRequest| -> Result<MpcSession, String> {
            let prg = check_program(&r.program)?;
            let circuit = compile_program(&prg, &r.function)?;
            let input = serialize_input(
                Role::Contributor,
                &prg,
                &circuit.fn_def,
                &r.plaintext_metadata,
            )?;
            Ok(MpcSession {
                circuit: circuit.gates,
                input_from_server: input,
                request_headers: request_headers.clone(),
            })
        };
        build(Box::new(handler))
    } else {
        println!("Starting server based on configured handlers...");
        let path = Path::new("program.garble.rs");
        let source_code =
            read_to_string(&path).unwrap_or_else(|_| panic!("could not read file {path:?}"));
        let source_code = source_code.trim().to_string();
        let program = check_program(&source_code)
            .unwrap_or_else(|e| panic!("{path:?} is not a valid program:\n{e}"));
        let mut handlers_with_circuit = HashMap::with_capacity(config.handlers.capacity());
        for (fn_name, handlers) in config.handlers {
            let circuit = compile_program(&program, &fn_name)
                .unwrap_or_else(|e| panic!("{fn_name} in {path:?} cannot be compiled:\n{e}"));
            let mut inputs = HashMap::with_capacity(handlers.len());
            for (metadata, input) in handlers {
                let input = serialize_input(Role::Contributor, &program, &circuit.fn_def, &input)
                    .unwrap_or_else(|e| panic!("Could not parse literal of handler {path:?}, {fn_name}, \"{metadata}\":\n{e}"));
                inputs.insert(metadata, input);
            }
            handlers_with_circuit.insert(fn_name, (circuit.gates, inputs));
        }
        let handler = move |r: MpcRequest| -> Result<MpcSession, String> {
            let hash_of_source_code = blake3::hash(r.program.trim().as_bytes());
            let server_program = source_code.chars();
            let client_program = r.program.chars();
            let mut differences = zip(client_program, server_program);
            let mismatch_index = differences.position(|(a, b)| a != b);

            if let Some(mismatch_index) = mismatch_index {
                fn extract_snippet(code: &str, index: usize) -> String {
                    let snippet: String = code.chars().skip(index).take(10).collect();
                    let snippet = snippet.replace('\\', "\\\\").replace('\n', "\\n");
                    format!("'{snippet}...'")
                }

                let client = extract_snippet(&r.program, mismatch_index);
                let server = extract_snippet(&source_code, mismatch_index);

                return Err(format!(
                    "Programs differ at character {mismatch_index}: {client}, {server}"
                ));
            }

            if let Some((circuit, handlers)) = handlers_with_circuit.get(&r.function) {
                if let Some(input) = handlers.get(&r.plaintext_metadata) {
                    Ok(MpcSession {
                        circuit: circuit.clone(),
                        input_from_server: input.clone(),
                        request_headers: HashMap::new(),
                    })
                } else {
                    Err(format!(
                        "could not find a handler for metadata '{}' (for the function '{}' in the program with hash {}):\n{} ",
                        r.plaintext_metadata, r.function, hash_of_source_code, r.program
                        ))
                }
            } else {
                Err(format!(
                        "could not find a handler for the function '{}' (in the program with hash {}):\n{}",
                        r.function, hash_of_source_code, r.program
                    ))
            }
        };
        build(Box::new(handler))
    }
}

fn set_fly_instance_id(request_headers: &mut HashMap<String, String>) {
    if let Ok(fly_alloc_id) = env::var("FLY_ALLOC_ID") {
        let fly_instance_id = fly_alloc_id.split("-").collect::<Vec<_>>()[0].to_string();
        request_headers.insert("fly-force-instance-id".to_string(), fly_instance_id);
    }
}

#[test]

fn test_fly_instance_id() {
    env::set_var("FLY_ALLOC_ID", "b996131a-5bae-215b-d0f1-2d75d1a8812b");
    let mut headers = HashMap::new();
    set_fly_instance_id(&mut headers);
    assert_eq!(headers.get("fly-force-instance-id").unwrap(), "b996131a");
}
