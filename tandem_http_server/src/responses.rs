use rocket::{
    http::Status,
    response::{self, Responder},
    serde::{Deserialize, Serialize},
};
use std::io::Cursor;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(crate = "rocket::serde")]
#[serde(tag = "error", content = "args")]
pub enum Error {
    CircuitHashMismatch,
    UnexpectedWireFormat(String),
    MpcRequestRejected(String),
    DuplicateEngineId { engine_id: String },
    UnexpectedMessageId,
    NoSuchEngineId { engine_id: String },
    Internal { message: String },
    BincodeError,
    Engine,
}

impl<'r, 'o: 'r> Responder<'r, 'o> for Error {
    fn respond_to(self, _: &'r rocket::Request<'_>) -> response::Result<'o> {
        let string =
            serde_json::to_string(&self).map_err(|_| rocket::http::Status::InternalServerError)?;

        rocket::Response::build()
            .header(rocket::http::ContentType::JSON)
            .sized_body(string.len(), Cursor::new(string))
            .status(self.status())
            .ok()
    }
}

impl Error {
    fn status(&self) -> Status {
        match self {
            Error::CircuitHashMismatch => Status::BadRequest,
            Error::UnexpectedWireFormat(_) => Status::BadRequest,
            Error::MpcRequestRejected(_) => Status::BadRequest,
            Error::DuplicateEngineId { .. } => Status::BadRequest,
            Error::UnexpectedMessageId => Status::BadRequest,
            Error::BincodeError => Status::BadRequest,
            Error::NoSuchEngineId { .. } => Status::NotFound,
            Error::Internal { .. } => Status::InternalServerError,
            Error::Engine { .. } => Status::InternalServerError,
        }
    }
}

impl From<bincode::Error> for Error {
    fn from(_: bincode::Error) -> Self {
        Self::BincodeError
    }
}

impl From<tandem::Error> for Error {
    fn from(_: tandem::Error) -> Self {
        Error::Engine
    }
}
