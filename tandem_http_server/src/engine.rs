#![allow(clippy::let_unit_value)]

use crate::{
    msg_queue::MessageId,
    requests::NewSession,
    responses::Error,
    state::{EngineRef, EngineRegistry},
    types::{EngineCreationResult, HandleMpcRequestFn},
};
use rand::Rng;
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use rocket::{
    data::ToByteUnit,
    fairing::{AdHoc, Fairing, Info, Kind},
    http::Header,
    response::{status::Created, stream::ByteStream},
    serde::{json::Json, Deserialize},
    Data, Request, Response, State,
};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};
use url::{Host, Url};

#[options("/")]
pub(crate) fn preflight_response_create_session() {}

#[post("/", format = "application/json", data = "<request>")]
pub(crate) fn create_session(
    r: &State<EngineRegistry>,
    request: Json<NewSession>,
) -> Result<Created<Json<EngineCreationResult>>, Error> {
    let server_version = env!("CARGO_PKG_VERSION").to_string();
    if request.client_version != server_version {
        return Err(Error::IncompatibleVersions {
            client_version: request.client_version.clone(),
            server_version,
        });
    }
    let invocation = crate::types::MpcRequest {
        plaintext_metadata: request.plaintext_metadata.clone(),
        program: request.program.clone(),
        function: request.function.clone(),
    };
    let handled = r
        .handle_input(invocation)
        .map_err(Error::MpcRequestRejected)?;
    let circuit_hash = handled.circuit.blake3_hash();
    if circuit_hash != request.circuit_hash {
        return Err(Error::CircuitHashMismatch);
    }

    let mut rng = ChaCha20Rng::from_entropy();
    let engine_id = uuid::Builder::from_random_bytes(rng.gen()).into_uuid();
    let engine_id = engine_id.to_string();
    let er = Arc::new(Mutex::new(EngineRef::new(
        rng,
        handled.circuit,
        handled.input_from_server,
    )?));
    let inserted = r.insert_engine(engine_id.clone(), er);

    if !inserted {
        return Err(Error::DuplicateEngineId { engine_id });
    }

    let body = EngineCreationResult {
        engine_id: engine_id.clone(),
        request_headers: handled.request_headers,
        server_version,
    };

    // Otherwise clippy complains that the uri! macro is using an unnecessary redefinition of engine_id.
    #[allow(clippy::redundant_locals)]
    let c = Created::new(uri!(dialog(engine_id)).to_string()).body(Json(body));
    Ok(c)
}

#[options("/<_engine_id>")]
pub(crate) fn preflight_response_delete_session(_engine_id: String) {}

#[delete("/<engine_id>")]
pub(crate) fn delete_session(engine_id: String, r: &State<EngineRegistry>) -> Result<(), Error> {
    let removed = r.drop_engine(&engine_id);
    if removed {
        Ok(())
    } else {
        Err(Error::NoSuchEngineId { engine_id })
    }
}

#[post("/<engine_id>", data = "<messages>")]
pub(crate) async fn dialog(
    engine_id: String,
    messages: Data<'_>,
    registry: &State<EngineRegistry>,
) -> Result<ByteStream![Vec<u8>], Error> {
    let stream = messages.open(20.mebibytes());
    let (last_durably_received_offset, messages): (Option<u32>, Vec<(Vec<u8>, MessageId)>) =
        bincode::deserialize(&stream.into_bytes().await.unwrap())?;

    let engine = registry.lookup(&engine_id)?;
    let mut engine = engine.lock().unwrap();

    if let Some(offset) = last_durably_received_offset {
        engine.flush_queue(offset);
    }
    for (msg, offset) in messages {
        engine.process_message(&msg, offset)?;
    }

    let result = (
        engine.dump_messages(),
        engine.last_durably_received_client_event_offset(),
    );

    if engine.is_done() {
        registry.drop_engine(&engine_id);
    }

    let (msgs, message_id) = result;
    let serialized = bincode::serialize(&(msgs, message_id))?;
    Ok(ByteStream! { yield serialized; })
}

pub fn stage(handle_input: HandleMpcRequestFn) -> AdHoc {
    AdHoc::on_ignite("Engine Context", |rocket| async {
        rocket
            .mount(
                "/",
                routes![
                    preflight_response_create_session,
                    preflight_response_delete_session,
                    create_session,
                    delete_session,
                    dialog
                ],
            )
            .manage(EngineRegistry::new(handle_input))
    })
}

pub(crate) struct Cors;

#[rocket::async_trait]
impl Fairing for Cors {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        #[derive(Debug, Deserialize)]
        #[serde(crate = "rocket::serde")]
        struct CorsConfig {
            origins: HashSet<String>,
        }

        let config = request.rocket().figment().extract::<CorsConfig>();
        if let Ok(config) = config {
            let request_origin = request.headers().get_one("origin");

            if let Some(origin) = request_origin {
                if let Ok(url) = Url::parse(origin) {
                    if config.origins.contains(url.as_str())
                        || url.host() == Some(Host::Domain("127.0.0.1"))
                        || url.host() == Some(Host::Domain("localhost"))
                    {
                        response.set_header(Header::new("Access-Control-Allow-Origin", origin));
                    }
                    // Access should be denied if the request's origin is not included in CorsConfig
                    // nor is a localhost. In that case, no header is set (automatically blocking
                    // the access).
                }
            }
        } else {
            response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        }

        response.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "POST, GET, PATCH, OPTIONS",
        ));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}
