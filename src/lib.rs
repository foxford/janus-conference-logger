#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;

#[macro_use]
mod janus_logger;

use std::path::Path;
use std::thread;

use chrono::{DateTime, Local, NaiveDateTime, Utc};
use regex::Regex;
use serde_json::{json, Value as JsonValue};

////////////////////////////////////////////////////////////////////////////////

lazy_static! {
    static ref CORE_HANDLE_ID_REGEX: Regex =
        Regex::new(r"\A\[(\d+)\]").expect("Failed to compile regex");
    static ref CONFERENCE_REGEX: Regex =
        Regex::new(r"\A\[CONFERENCE (\{.+\})\] (.+)").expect("Failed to compile regex");
}

#[derive(Debug, Serialize)]
struct JsonMessage<'a> {
    ts: String,
    level: &'static str,
    #[serde(flatten)]
    source_with_tags: SourceWithTags,
    msg: &'a str,
}

#[derive(Debug, Serialize)]
#[serde(tag = "source", rename_all = "lowercase")]
enum SourceWithTags {
    Core(CoreTags),
    Conference(ConferenceTags),
}

#[derive(Debug, Default, Serialize)]
struct CoreTags {
    #[serde(skip_serializing_if = "Option::is_none")]
    handle_id: Option<usize>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct ConferenceTags {
    #[serde(skip_serializing_if = "Option::is_none")]
    handle_id: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rtc_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    transaction: Option<String>,
}

#[derive(Debug)]
struct Message {
    timestamp: i64,
    line: String,
}

impl Message {
    fn new(timestamp: i64, line: &str) -> Self {
        Self {
            timestamp,
            line: line.to_owned(),
        }
    }

    fn to_json_message<'a>(&'a self) -> JsonMessage<'a> {
        let ts = self.timestamp().with_timezone(&Local).to_rfc3339();
        let (level, rest) = self.extract_level();
        let (source_with_tags, rest) = Self::extract_source_with_tags(rest);

        JsonMessage {
            ts,
            level,
            source_with_tags,
            msg: rest.trim(),
        }
    }

    fn timestamp(&self) -> DateTime<Utc> {
        let secs = self.timestamp / 1000000;
        let nsecs = self.timestamp % 1000000 * 1000;
        DateTime::from_utc(NaiveDateTime::from_timestamp(secs, nsecs as u32), Utc)
    }

    fn extract_level(&self) -> (&'static str, &str) {
        if let Some(rest) = self.line.strip_prefix("[ERR] ") {
            return ("ERRO", rest);
        }

        if let Some(rest) = self.line.strip_prefix("[WARN] ") {
            return ("WARN", rest);
        }

        // More verbose levels than WARN don't have a prefix so consider them all as INFO.
        ("INFO", &self.line)
    }

    fn extract_source_with_tags(line: &str) -> (SourceWithTags, &str) {
        if let Some(captures) = CONFERENCE_REGEX.captures(line) {
            let tags = captures.get(1).map(|c| c.as_str()).unwrap_or("{}");

            let parsed_tags = serde_json::from_str::<ConferenceTags>(tags)
                .unwrap_or_else(|_err| ConferenceTags::default());

            let rest = captures.get(2).map(|c| c.as_str()).unwrap_or("");
            (SourceWithTags::Conference(parsed_tags), rest)
        } else {
            let (tags, rest) = Self::extract_core_tags(line);
            (SourceWithTags::Core(tags), rest)
        }
    }

    fn extract_core_tags(line: &str) -> (CoreTags, &str) {
        let mut tags = CoreTags::default();
        let mut rest = line;

        if let Some(captures) = CORE_HANDLE_ID_REGEX.captures(rest) {
            if let Some(capture) = captures.get(1) {
                if let Ok(handle_id) = capture.as_str().parse::<usize>() {
                    tags.handle_id = Some(handle_id);
                    let prefix = format!("[{}] ", handle_id);
                    rest = rest.strip_prefix(&prefix).unwrap_or(rest);
                }
            }
        }

        (tags, rest)
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub(crate) struct JanusConferenceLogger {
    tx: crossbeam_channel::Sender<Message>,
}

impl janus_logger::JanusLogger for JanusConferenceLogger {
    fn new(_server_name: &str, _config_path: &Path) -> Self {
        let (tx, rx) = crossbeam_channel::unbounded::<Message>();

        thread::spawn(move || {
            while let Ok(message) = rx.recv() {
                let json_message = message.to_json_message();

                if let Ok(dumped_message) = serde_json::to_string(&json_message) {
                    println!("{}", dumped_message);
                }
            }
        });

        Self { tx }
    }

    fn incoming_logline(&self, timestamp: i64, line: &str) {
        let _result = self.tx.send(Message::new(timestamp, line));
    }

    fn handle_request(&self, _request: &JsonValue) -> JsonValue {
        json!({"error": "not implemented"})
    }
}

define_logger!(JanusConferenceLogger);
