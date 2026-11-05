use std::{
    collections::HashMap,
    io::Write,
    sync::{Mutex, mpsc},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(super) type Reply = Result<Value, String>;

#[derive(Default)]
pub(super) struct Rpc {
    state: Mutex<State>,
}

impl Rpc {
    pub fn send(
        &self,
        method: &str,
        params: Option<Value>,
        writer: &mut impl Write,
    ) -> Result<mpsc::Receiver<Reply>, String> {
        let (reply, response) = mpsc::channel();
        let id = {
            let mut state = self.state.lock().unwrap();
            if let Some(error) = &state.closed {
                return Err(error.clone());
            }
            let id = state.next_id;
            state.next_id = state
                .next_id
                .checked_add(1)
                .ok_or("request IDs exhausted")?;
            state.pending.insert(
                Id::Number(id),
                PendingRequest {
                    method: method.into(),
                    reply,
                },
            );
            id
        };
        let envelope = Envelope::new(Message::Request {
            id: Id::Number(id),
            method: method.into(),
            params,
        });
        if let Err(error) = envelope.write(writer) {
            self.close(&error);
            return Err(error);
        }
        Ok(response)
    }

    pub fn receive(&self, line: &str) -> Result<(), String> {
        let envelope: Envelope = serde_json::from_str(line).map_err(|error| error.to_string())?;
        let (id, response) = match envelope.message {
            Message::Success { id, result } => (id, Ok(result)),
            Message::Failure { id, error } => {
                (id, Err(format!("{} ({})", error.message, error.code)))
            }
            _ => return Err("expected a JSON-RPC response".into()),
        };
        let pending = {
            let mut state = self.state.lock().unwrap();
            match state.pending.remove(&id) {
                Some(pending) => pending,
                None if state.closed.is_some() => return Ok(()),
                None => return Err(format!("response for unknown request ID: {id:?}")),
            }
        };
        let response = response.map_err(|error| format!("{}: {error}", pending.method));
        let _ = pending.reply.send(response);
        Ok(())
    }

    pub fn close(&self, error: &str) {
        let pending = {
            let mut state = self.state.lock().unwrap();
            if state.closed.is_some() {
                return;
            }
            state.closed = Some(error.into());
            std::mem::take(&mut state.pending)
        };
        for (_, pending) in pending {
            let _ = pending
                .reply
                .send(Err(format!("{}: {error}", pending.method)));
        }
    }
}

struct State {
    next_id: i64,
    pending: HashMap<Id, PendingRequest>,
    closed: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            next_id: 1,
            pending: HashMap::new(),
            closed: None,
        }
    }
}

struct PendingRequest {
    method: String,
    reply: mpsc::Sender<Reply>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Envelope {
    jsonrpc: Version,
    #[serde(flatten)]
    message: Message,
}

impl Envelope {
    fn new(message: Message) -> Self {
        Self {
            jsonrpc: Version::V2,
            message,
        }
    }

    fn write(&self, writer: &mut impl Write) -> Result<(), String> {
        let mut bytes = serde_json::to_vec(self).map_err(|error| error.to_string())?;
        bytes.push(b'\n');
        writer
            .write_all(&bytes)
            .map_err(|error| error.to_string())?;
        writer.flush().map_err(|error| error.to_string())
    }
}

// JSON-RPC 2.0 §5: successful responses include `result`, even when it is null.
// Failed responses include `error` instead; the two fields cannot appear together.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
enum Message {
    Request {
        id: Id,
        method: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        params: Option<Value>,
    },
    Notification {
        method: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        params: Option<Value>,
    },
    Success {
        id: Id,
        result: Value,
    },
    Failure {
        id: Id,
        error: RpcError,
    },
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
enum Id {
    Number(i64),
    String(String),
    Null,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct RpcError {
    code: i64,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum Version {
    #[serde(rename = "2.0")]
    V2,
}

#[cfg(test)]
mod tests {
    use std::{
        io::{self, BufWriter, Write},
        sync::{Arc, mpsc::TryRecvError},
        thread,
    };

    use serde_json::{Value, json};

    use super::Rpc;

    #[test]
    fn requests_have_unique_ids_and_are_written_as_flushed_json_lines() {
        let rpc = Rpc::default();
        let mut writer = BufWriter::new(Vec::new());
        let _first = rpc.send("ping", None, &mut writer).unwrap();
        let _second = rpc
            .send(
                "echo",
                Some(json!({"message": "first\nsecond"})),
                &mut writer,
            )
            .unwrap();

        let output = std::str::from_utf8(writer.get_ref()).unwrap();
        assert!(output.ends_with('\n'));
        let lines: Vec<_> = output.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#);
        assert_eq!(
            serde_json::from_str::<Value>(lines[1]).unwrap(),
            json!({
                "jsonrpc": "2.0", "id": 2, "method": "echo",
                "params": {"message": "first\nsecond"}
            })
        );
    }

    #[test]
    fn partial_writes_still_produce_a_complete_request() {
        let rpc = Rpc::default();
        let mut writer = ShortWriter(Vec::new());

        let _response = rpc.send("ping", None, &mut writer).unwrap();

        assert_eq!(
            writer.0,
            b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n"
        );
    }

    #[test]
    fn a_response_can_arrive_during_the_write_without_a_bookkeeping_lock() {
        let rpc = Rpc::default();
        let mut writer = ReplyingWriter(&rpc);

        let response = rpc.send("ping", None, &mut writer).unwrap();

        assert_eq!(response.try_recv().unwrap().unwrap(), json!("pong"));
    }

    #[test]
    fn responses_to_the_same_method_are_correlated_in_reverse_order() {
        let rpc = Rpc::default();
        let mut writer = Vec::new();
        let first = rpc
            .send("lookup", Some(json!({"key": "first"})), &mut writer)
            .unwrap();
        let second = rpc
            .send("lookup", Some(json!({"key": "second"})), &mut writer)
            .unwrap();

        rpc.receive(r#"{"jsonrpc":"2.0","id":2,"result":"second result"}"#)
            .unwrap();

        assert_eq!(second.try_recv().unwrap().unwrap(), json!("second result"));
        assert_eq!(first.try_recv().unwrap_err(), TryRecvError::Empty);

        rpc.receive(r#"{"jsonrpc":"2.0","id":1,"result":"first result"}"#)
            .unwrap();

        assert_eq!(first.try_recv().unwrap().unwrap(), json!("first result"));
    }

    #[test]
    fn explicit_null_is_a_successful_result() {
        let rpc = Rpc::default();
        let response = rpc.send("nothing", None, &mut Vec::new()).unwrap();

        rpc.receive(r#"{"jsonrpc":"2.0","id":1,"result":null}"#)
            .unwrap();

        assert_eq!(response.try_recv().unwrap().unwrap(), Value::Null);
    }

    #[test]
    fn a_remote_error_identifies_its_method_and_does_not_close_the_rpc() {
        let rpc = Rpc::default();
        let first = rpc.send("rejected", None, &mut Vec::new()).unwrap();

        rpc.receive(r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"denied"}}"#)
            .unwrap();

        assert_eq!(
            first.try_recv().unwrap().unwrap_err(),
            "rejected: denied (-32000)"
        );

        let second = rpc.send("ping", None, &mut Vec::new()).unwrap();
        rpc.receive(r#"{"jsonrpc":"2.0","id":2,"result":"pong"}"#)
            .unwrap();

        assert_eq!(second.try_recv().unwrap().unwrap(), json!("pong"));
    }

    #[test]
    fn malformed_or_unsupported_messages_do_not_complete_the_pending_request() {
        let rpc = Rpc::default();
        let response = rpc.send("ping", None, &mut Vec::new()).unwrap();

        for line in [
            "not json",
            "",
            "{}",
            r#"{"jsonrpc":"1.0","id":1,"result":null}"#,
            r#"{"id":1,"result":null}"#,
            r#"{"jsonrpc":"2.0","id":1}"#,
            r#"{"jsonrpc":"2.0","result":null}"#,
            r#"{"jsonrpc":"2.0","id":1,"result":null,"error":{"code":-32603,"message":"failure"}}"#,
            r#"{"jsonrpc":"2.0","id":1,"error":{"message":"missing code"}}"#,
            r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#,
            r#"{"jsonrpc":"2.0","method":"progress"}"#,
        ] {
            assert!(rpc.receive(line).is_err(), "accepted: {line}");
            assert_eq!(response.try_recv().unwrap_err(), TryRecvError::Empty);
        }

        rpc.receive(r#"{"jsonrpc":"2.0","id":1,"result":"pong"}"#)
            .unwrap();
        assert_eq!(response.try_recv().unwrap().unwrap(), json!("pong"));
    }

    #[test]
    fn unknown_ids_and_different_id_types_cannot_complete_a_pending_request() {
        let rpc = Rpc::default();
        let response = rpc.send("ping", None, &mut Vec::new()).unwrap();

        for id in [json!(99), json!("1"), Value::Null] {
            let line = json!({"jsonrpc": "2.0", "id": id, "result": "wrong"}).to_string();
            assert!(
                rpc.receive(&line)
                    .unwrap_err()
                    .contains("unknown request ID")
            );
            assert_eq!(response.try_recv().unwrap_err(), TryRecvError::Empty);
        }

        rpc.receive(r#"{"jsonrpc":"2.0","id":1,"result":"pong"}"#)
            .unwrap();
        assert_eq!(response.try_recv().unwrap().unwrap(), json!("pong"));

        assert!(
            rpc.receive(r#"{"jsonrpc":"2.0","id":1,"result":"duplicate"}"#)
                .unwrap_err()
                .contains("unknown request ID")
        );
        assert_eq!(response.try_recv().unwrap_err(), TryRecvError::Disconnected);
    }

    #[test]
    fn closing_fails_pending_requests_and_rejects_new_sends_without_writing() {
        let rpc = Rpc::default();
        let first = rpc.send("first", None, &mut Vec::new()).unwrap();
        let second = rpc.send("second", None, &mut Vec::new()).unwrap();

        rpc.close("stdout closed");
        rpc.close("another reason");

        assert_eq!(
            first.try_recv().unwrap().unwrap_err(),
            "first: stdout closed"
        );
        assert_eq!(
            second.try_recv().unwrap().unwrap_err(),
            "second: stdout closed"
        );
        let mut writer = b"unchanged".to_vec();
        assert_eq!(
            rpc.send("third", None, &mut writer).unwrap_err(),
            "stdout closed"
        );
        assert_eq!(writer, b"unchanged");

        rpc.receive(r#"{"jsonrpc":"2.0","id":1,"result":"late"}"#)
            .unwrap();
        assert_eq!(first.try_recv().unwrap_err(), TryRecvError::Disconnected);
    }

    #[test]
    fn write_and_flush_failures_close_the_rpc_and_fail_other_pending_requests() {
        for fail_on_flush in [false, true] {
            let rpc = Rpc::default();
            let first = rpc.send("first", None, &mut Vec::new()).unwrap();
            let error = if fail_on_flush {
                "flush failed"
            } else {
                "write failed"
            };
            let mut writer = FailingWriter { fail_on_flush };

            assert_eq!(rpc.send("second", None, &mut writer).unwrap_err(), error);

            assert_eq!(
                first.try_recv().unwrap().unwrap_err(),
                format!("first: {error}")
            );
            assert_eq!(rpc.send("third", None, &mut Vec::new()).unwrap_err(), error);
        }
    }

    #[test]
    fn dropping_a_waiter_does_not_prevent_later_requests_from_completing() {
        let rpc = Rpc::default();
        let abandoned = rpc.send("abandoned", None, &mut Vec::new()).unwrap();
        drop(abandoned);

        rpc.receive(r#"{"jsonrpc":"2.0","id":1,"result":null}"#)
            .unwrap();
        let response = rpc.send("ping", None, &mut Vec::new()).unwrap();
        rpc.receive(r#"{"jsonrpc":"2.0","id":2,"result":"pong"}"#)
            .unwrap();

        assert_eq!(response.try_recv().unwrap().unwrap(), json!("pong"));
    }

    #[test]
    fn a_separate_reader_thread_can_complete_a_request() {
        let rpc = Arc::new(Rpc::default());
        let response = rpc.send("ping", None, &mut Vec::new()).unwrap();
        let reader = Arc::clone(&rpc);

        thread::spawn(move || reader.receive(r#"{"jsonrpc":"2.0","id":1,"result":"pong"}"#))
            .join()
            .unwrap()
            .unwrap();

        assert_eq!(response.try_recv().unwrap().unwrap(), json!("pong"));
    }

    #[test]
    fn a_zero_length_write_fails_pending_requests_instead_of_leaving_them_waiting() {
        let rpc = Rpc::default();
        let first = rpc.send("first", None, &mut Vec::new()).unwrap();
        let mut writer: &mut [u8] = &mut [];

        let error = rpc.send("second", None, &mut writer).unwrap_err();

        assert_eq!(
            first.try_recv().unwrap().unwrap_err(),
            format!("first: {error}")
        );
        assert_eq!(rpc.send("third", None, &mut Vec::new()).unwrap_err(), error);
    }

    #[test]
    fn exhausted_ids_do_not_write_or_disrupt_existing_requests() {
        let rpc = Rpc::default();
        let response = rpc.send("ping", None, &mut Vec::new()).unwrap();
        rpc.state.lock().unwrap().next_id = i64::MAX;
        let mut writer = Vec::new();

        assert_eq!(
            rpc.send("another", None, &mut writer).unwrap_err(),
            "request IDs exhausted"
        );
        assert!(writer.is_empty());

        rpc.receive(r#"{"jsonrpc":"2.0","id":1,"result":"pong"}"#)
            .unwrap();
        assert_eq!(response.try_recv().unwrap().unwrap(), json!("pong"));
    }

    struct ShortWriter(Vec<u8>);

    impl Write for ShortWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            let count = bytes.len().min(3);
            self.0.extend_from_slice(&bytes[..count]);
            Ok(count)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct ReplyingWriter<'a>(&'a Rpc);

    impl Write for ReplyingWriter<'_> {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            assert!(self.0.state.try_lock().is_ok(), "RPC lock held during IO");
            self.0
                .receive(r#"{"jsonrpc":"2.0","id":1,"result":"pong"}"#)
                .unwrap();
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct FailingWriter {
        fail_on_flush: bool,
    }

    impl Write for FailingWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.fail_on_flush {
                Ok(bytes.len())
            } else {
                Err(io::Error::other("write failed"))
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("flush failed"))
        }
    }
}
