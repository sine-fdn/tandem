use std::collections::{vec_deque, VecDeque};

pub(crate) type MessageId = u32;

#[derive(Clone)]
pub(crate) struct MsgQueue {
    send_q: VecDeque<Vec<u8>>,
    msg_counter: usize,
}

impl MsgQueue {
    pub(crate) fn new() -> Self {
        Self {
            send_q: VecDeque::with_capacity(100),
            msg_counter: 0,
        }
    }

    // flushes the queue until excluding {last_durably_received_offset}.
    //
    // after this operation, the logical message id of each queued message will be **strictly bigger than** the given offset
    pub(crate) fn flush_queue(&mut self, last_durably_received_offset: MessageId) -> usize {
        let last_durably_received_offset = last_durably_received_offset as usize;

        let first_offset = self.msg_counter - self.send_q.len();
        let mut offset = first_offset;

        while offset <= last_durably_received_offset && !self.send_q.is_empty() {
            self.send_q.pop_front();
            offset += 1;
        }

        // return how many elements were removed
        offset - first_offset
    }

    pub(crate) fn send(&mut self, msg: Vec<u8>) {
        self.msg_counter += 1;
        self.send_q.push_back(msg);
    }
}

pub struct MsgIter<'a>(vec_deque::Iter<'a, Vec<u8>>, MessageId);

impl<'a> Iterator for MsgIter<'a> {
    type Item = (&'a Vec<u8>, MessageId);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|msg| {
            self.1 += 1;
            (msg, self.1 - 1)
        })
    }
}

impl MsgQueue {
    pub fn msgs_iter(&self) -> MsgIter<'_> {
        let message_id = self.msg_counter - self.send_q.len();
        MsgIter(self.send_q.iter(), message_id as MessageId)
    }
}

#[test]
fn test_flush_queue() {
    let c = MsgQueue {
        send_q: Default::default(),
        msg_counter: Default::default(),
    };

    {
        assert_eq!(0, c.clone().flush_queue(0));
        assert_eq!(0, c.clone().flush_queue(1));
        assert_eq!(0, c.clone().flush_queue(10));
    }
    {
        let mut c = c;

        c.send(bincode::serialize(&vec![(0, false)]).unwrap());
        assert_eq!(1, c.clone().flush_queue(0));

        c.flush_queue(0);
        assert_eq!(None, c.send_q.pop_front());

        c.send(bincode::serialize(&vec![(1, false)]).unwrap());
        assert_eq!(0, c.clone().flush_queue(0));
        assert_eq!(1, c.clone().flush_queue(1));
        assert_eq!(
            Some(bincode::serialize(&vec![(1, false)]).unwrap()),
            c.send_q.pop_front()
        );
    }
}
