use std::{
    collections::{hash_map::Entry, HashMap},
    sync::{Arc, Mutex, RwLock},
};

use rand_chacha::ChaCha20Rng;
use tandem::{
    states::{Contributor, Msg},
    Circuit,
};

use crate::{
    msg_queue::{MessageId, MsgQueue},
    responses::Error,
    types::{EngineId, HandleMpcRequestFn, MpcRequest, MpcSession},
};

/// reference to a (running) Engine
pub(crate) struct EngineRef {
    last_durably_received_client_event_offset: Option<MessageId>,
    tandem: Option<Contributor<Circuit, Vec<bool>>>,
    steps_remaining: u32,
    context: MsgQueue,
}

impl EngineRef {
    pub fn new(rng: ChaCha20Rng, program: Circuit, input: Vec<bool>) -> Result<Self, Error> {
        let mut context = MsgQueue::new();
        let (contrib, initial_msg) = Contributor::new(program, input, rng)?;
        let steps_remaining = contrib.steps();
        context.send(initial_msg);

        Ok(Self {
            context,
            tandem: Some(contrib),
            steps_remaining,
            last_durably_received_client_event_offset: None,
        })
    }

    pub fn process_message(&mut self, msg: &Msg, offset: MessageId) -> Result<(), Error> {
        if (self.last_durably_received_client_event_offset.is_none() && offset == 0)
            || self.last_durably_received_client_event_offset == Some(offset - 1)
        {
            self.last_durably_received_client_event_offset = Some(offset);
            if let Some(contrib) = self.tandem.take() {
                let (next_state, reply) = contrib.run(msg)?;
                self.tandem = Some(next_state);
                self.context.send(reply);
            }
            Ok(())
        } else {
            Err(Error::UnexpectedMessageId)
        }
    }

    pub fn last_durably_received_client_event_offset(&self) -> Option<MessageId> {
        self.last_durably_received_client_event_offset
    }

    pub fn flush_queue(&mut self, last_durably_received_offset: MessageId) {
        self.context.flush_queue(last_durably_received_offset);
    }

    pub fn dump_messages(&self) -> Vec<(&Msg, MessageId)> {
        self.context.msgs_iter().map(|m| (m.0, m.1)).collect()
    }

    pub fn is_done(&self) -> bool {
        self.steps_remaining == 0
    }
}

pub(crate) struct EngineRegistry {
    registry: RwLock<HashMap<EngineId, Arc<Mutex<EngineRef>>>>,
    handler: HandleMpcRequestFn,
}

impl EngineRegistry {
    pub(crate) fn new(handler: HandleMpcRequestFn) -> Self {
        Self {
            registry: RwLock::new(HashMap::new()),
            handler,
        }
    }

    pub(crate) fn insert_engine(&self, engine_id: EngineId, engine: Arc<Mutex<EngineRef>>) -> bool {
        let mut r = self.registry.write().unwrap();
        if let Entry::Vacant(e) = r.entry(engine_id) {
            e.insert(engine);
            true
        } else {
            false
        }
    }

    pub(crate) fn drop_engine(&self, engine_id: &EngineId) -> bool {
        let mut r = self.registry.write().unwrap();
        r.remove(engine_id).is_some()
    }

    pub(crate) fn lookup(&self, engine_id: &EngineId) -> Result<Arc<Mutex<EngineRef>>, Error> {
        let r = self.registry.read().unwrap();
        match r.get(engine_id).map(Arc::clone) {
            Some(e) => Ok(e),
            None => Err(Error::NoSuchEngineId {
                engine_id: engine_id.clone(),
            }),
        }
    }

    pub(crate) fn handle_input(&self, invocation: MpcRequest) -> Result<MpcSession, String> {
        self.handler.as_ref()(invocation)
    }
}
