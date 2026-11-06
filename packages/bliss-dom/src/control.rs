//! Loopback control surface for agents — the host side of `BLISS-AINATIVE-1`.
//!
//! A **dev-only** channel that lets an external agent read, mutate and re-resolve
//! a document: connect to a localhost socket, authenticate with a token, then
//! exchange one command per line. It is gated behind the `control` feature and is
//! never enabled in a shipped app; run it only on `127.0.0.1`.
//!
//! Protocol (one request per line; the response is one line):
//!
//! ```text
//! <token>                        (first line; "ok bliss-control" on success)
//! dump                           -> the tree JSON (document_tree_json)
//! resolve                        -> ok
//! set_attr  <node> <name> <value>   \
//! set_style <node> <prop> <value>    |  applied (then resolved)
//! set_text  <node> <text…>           |
//! remove    <node>                  /
//! ```
//!
//! The server runs on the document's thread (a `BaseDocument` is not `Send`), so
//! a headless dev server is a single-threaded accept loop; wiring it behind a
//! running window is a follow-up (marshal the command to the UI thread).
//!
//! ```rust,no_run
//! # #[cfg(feature = "control")] fn main() {
//! use std::net::TcpListener;
//! let mut doc = bliss_dom::BaseDocument::new(Default::default());
//! let listener = TcpListener::bind("127.0.0.1:9222").unwrap();
//! let _ = bliss_dom::control::serve(&mut doc, listener, "some-dev-token");
//! # }
//! # #[cfg(not(feature = "control"))] fn main() {}
//! ```

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};

use crate::{BaseDocument, Mutation};

/// A control command parsed from one request line.
#[derive(Clone, Debug, PartialEq)]
pub enum ControlCommand {
    /// Return the resolved tree as JSON.
    Dump,
    /// Restyle + relayout.
    Resolve,
    /// Apply mutations, then restyle + relayout.
    Mutations(Vec<Mutation>),
}

/// Parse one request line into a [`ControlCommand`].
pub fn parse_command(line: &str) -> Result<ControlCommand, String> {
    let line = line.trim();
    let mut parts = line.splitn(2, char::is_whitespace);
    let cmd = parts.next().unwrap_or("");
    let rest = parts.next().unwrap_or("").trim();

    match cmd {
        "dump" => Ok(ControlCommand::Dump),
        "resolve" => Ok(ControlCommand::Resolve),
        "set_attr" | "set_style" => {
            let mut it = rest.splitn(3, char::is_whitespace);
            let node = parse_node(it.next())?;
            let name = it.next().ok_or("set_attr/set_style needs <node> <name> <value>")?.to_string();
            let value = it.next().ok_or("set_attr/set_style needs <value>")?.to_string();
            let mutation = if cmd == "set_attr" {
                Mutation::SetAttribute { node, name, value }
            } else {
                Mutation::SetStyle { node, property: name, value }
            };
            Ok(ControlCommand::Mutations(vec![mutation]))
        }
        "set_text" => {
            let mut it = rest.splitn(2, char::is_whitespace);
            let node = parse_node(it.next())?;
            let text = it.next().unwrap_or("").to_string();
            Ok(ControlCommand::Mutations(vec![Mutation::SetText { node, text }]))
        }
        "remove" => Ok(ControlCommand::Mutations(vec![Mutation::Remove { node: parse_node(Some(rest))? }])),
        "" => Err("empty command".to_string()),
        other => Err(format!("unknown command: {other}")),
    }
}

fn parse_node(field: Option<&str>) -> Result<usize, String> {
    field
        .ok_or_else(|| "missing <node>".to_string())?
        .parse::<usize>()
        .map_err(|_| "invalid <node>".to_string())
}

/// Execute a command against `doc`, returning the single-line response.
pub fn execute(doc: &mut BaseDocument, command: ControlCommand) -> String {
    match command {
        ControlCommand::Dump => doc.document_tree_json(),
        ControlCommand::Resolve => {
            let now = doc.now_ms() / 1000.0;
            doc.resolve(now);
            "ok".to_string()
        }
        ControlCommand::Mutations(mutations) => {
            doc.apply_mutations(&mutations);
            let now = doc.now_ms() / 1000.0;
            doc.resolve(now);
            format!("ok {}", mutations.len())
        }
    }
}

/// Serve `doc` on `listener` until it is dropped. Blocking; one connection at a
/// time. Use only on `127.0.0.1`, with a token, in a dev build.
pub fn serve(doc: &mut BaseDocument, listener: TcpListener, token: &str) -> std::io::Result<()> {
    for stream in listener.incoming().flatten() {
        let _ = serve_connection(doc, stream, token);
    }
    Ok(())
}

/// Handle one control connection: authenticate, then execute commands until the
/// client closes.
pub fn serve_connection(
    doc: &mut BaseDocument,
    stream: TcpStream,
    token: &str,
) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut writer = stream;

    // First line is the token.
    let mut auth = String::new();
    if reader.read_line(&mut auth)? == 0 || auth.trim() != token {
        writer.write_all(b"error unauthorized\n")?;
        return Ok(());
    }
    writer.write_all(b"ok bliss-control\n")?;

    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        let response = match parse_command(&line) {
            Ok(command) => execute(doc, command),
            Err(error) => format!("error {error}"),
        };
        writer.write_all(response.as_bytes())?;
        writer.write_all(b"\n")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DocumentConfig, qual_name};
    use std::net::TcpStream;

    fn doc_with_div() -> (BaseDocument, usize) {
        let mut doc = BaseDocument::new(DocumentConfig::default());
        let mut mutator = doc.mutate();
        let node = mutator.create_element(qual_name!("div"), vec![]);
        mutator.append_children(0, &[node]);
        drop(mutator);
        (doc, node)
    }

    #[test]
    fn parses_and_executes() {
        let (mut doc, node) = doc_with_div();
        assert_eq!(parse_command("dump").unwrap(), ControlCommand::Dump);
        assert_eq!(parse_command("  resolve ").unwrap(), ControlCommand::Resolve);
        assert!(parse_command("nonsense").is_err());

        let cmd = parse_command(&format!("set_attr {node} title hello world")).unwrap();
        assert_eq!(execute(&mut doc, cmd), "ok 1");
        assert!(execute(&mut doc, ControlCommand::Dump).contains("\"title\":\"hello world\""));
    }

    #[test]
    fn round_trips_over_tcp() {
        let (mut doc, node) = doc_with_div();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let client = std::thread::spawn(move || {
            let mut stream = TcpStream::connect(addr).unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();

            writeln!(stream, "dev-token").unwrap();
            reader.read_line(&mut line).unwrap();
            assert_eq!(line.trim(), "ok bliss-control");

            writeln!(stream, "set_attr {node} title hi").unwrap();
            line.clear();
            reader.read_line(&mut line).unwrap();
            let ack = line.trim().to_string();

            writeln!(stream, "dump").unwrap();
            line.clear();
            reader.read_line(&mut line).unwrap();
            (ack, line.trim().to_string())
        });

        let (stream, _) = listener.accept().unwrap();
        serve_connection(&mut doc, stream, "dev-token").unwrap();

        let (ack, tree) = client.join().unwrap();
        assert_eq!(ack, "ok 1");
        assert!(tree.contains("\"title\":\"hi\""), "{tree}");
    }

    #[test]
    fn rejects_a_bad_token() {
        let (mut doc, _node) = doc_with_div();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let client = std::thread::spawn(move || {
            let mut stream = TcpStream::connect(addr).unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            writeln!(stream, "wrong").unwrap();
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            line.trim().to_string()
        });
        let (stream, _) = listener.accept().unwrap();
        serve_connection(&mut doc, stream, "dev-token").unwrap();
        assert_eq!(client.join().unwrap(), "error unauthorized");
    }
}
