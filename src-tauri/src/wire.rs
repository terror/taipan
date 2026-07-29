use super::*;

pub const DELIMITER: &[u8] = b"<IDS|MSG>";

pub type Frame = Vec<u8>;

pub type JsonObject = Map<String, Value>;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
  Control,
  Heartbeat,
  Iopub,
  Shell,
}

impl Display for Channel {
  fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
    formatter.write_str(match self {
      Self::Control => "control",
      Self::Heartbeat => "heartbeat",
      Self::Iopub => "iopub",
      Self::Shell => "shell",
    })
  }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Envelope {
  pub buffers: Vec<Frame>,
  pub content: JsonObject,
  pub header: Header,
  pub identities: Vec<Frame>,
  pub metadata: JsonObject,
  pub parent_header: ParentHeader,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JsonFrame {
  Content,
  Header,
  Metadata,
  ParentHeader,
}

impl Display for JsonFrame {
  fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
    formatter.write_str(match self {
      Self::Content => "content",
      Self::Header => "header",
      Self::Metadata => "metadata",
      Self::ParentHeader => "parent header",
    })
  }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Header {
  pub date: String,
  #[serde(flatten)]
  pub extra: JsonObject,
  pub msg_id: String,
  pub msg_type: MessageType,
  pub session: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub subshell_id: Option<String>,
  pub username: String,
  pub version: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MessageType(pub String);

impl Display for MessageType {
  fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
    formatter.write_str(&self.0)
  }
}

impl From<&str> for MessageType {
  fn from(value: &str) -> Self {
    Self(value.into())
  }
}

impl From<String> for MessageType {
  fn from(value: String) -> Self {
    Self(value)
  }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum ParentHeader {
  #[default]
  Empty,
  Header(Header),
}

impl<'de> Deserialize<'de> for ParentHeader {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let value = JsonObject::deserialize(deserializer)?;

    if value.is_empty() {
      Ok(Self::Empty)
    } else {
      serde_json::from_value(Value::Object(value))
        .map(Self::Header)
        .map_err(de::Error::custom)
    }
  }
}

impl Serialize for ParentHeader {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    match self {
      Self::Empty => JsonObject::new().serialize(serializer),
      Self::Header(header) => header.serialize(serializer),
    }
  }
}

pub struct WireProtocol {
  key: Vec<u8>,
}

#[allow(clippy::missing_errors_doc)]
impl WireProtocol {
  pub fn decode(&self, frames: &[Frame]) -> Result<Envelope, WireError> {
    let delimiter = frames
      .iter()
      .position(|frame| frame == DELIMITER)
      .ok_or(WireError::MissingDelimiter)?;

    let message = &frames[delimiter + 1..];

    if message.len() < 5 {
      return Err(WireError::TooFewFrames {
        actual: message.len(),
      });
    }

    let signed = [&message[1][..], &message[2], &message[3], &message[4]];
    self.verify(&message[0], &signed)?;

    Ok(Envelope {
      buffers: message[5..].to_vec(),
      content: deserialize(&message[4], JsonFrame::Content)?,
      header: deserialize(&message[1], JsonFrame::Header)?,
      identities: frames[..delimiter].to_vec(),
      metadata: deserialize(&message[3], JsonFrame::Metadata)?,
      parent_header: deserialize(&message[2], JsonFrame::ParentHeader)?,
    })
  }

  pub fn encode(&self, envelope: &Envelope) -> Result<Vec<Frame>, WireError> {
    let header = serialize(&envelope.header, JsonFrame::Header)?;
    let parent_header =
      serialize(&envelope.parent_header, JsonFrame::ParentHeader)?;
    let metadata = serialize(&envelope.metadata, JsonFrame::Metadata)?;
    let content = serialize(&envelope.content, JsonFrame::Content)?;
    let signed = [&header[..], &parent_header, &metadata, &content];

    let mut frames = Vec::with_capacity(
      envelope.identities.len() + envelope.buffers.len() + 6,
    );

    frames.extend_from_slice(&envelope.identities);
    frames.push(DELIMITER.to_vec());
    frames.push(self.sign(&signed)?);
    frames.extend([header, parent_header, metadata, content]);
    frames.extend_from_slice(&envelope.buffers);

    Ok(frames)
  }

  pub fn new(key: impl Into<Vec<u8>>) -> Self {
    Self { key: key.into() }
  }

  fn sign(&self, frames: &[&[u8]]) -> Result<Frame, WireError> {
    if self.key.is_empty() {
      return Ok(Frame::new());
    }

    let mut mac = Hmac::<Sha256>::new_from_slice(&self.key)
      .map_err(|_| WireError::InvalidKeyLength)?;

    for frame in frames {
      mac.update(frame);
    }

    Ok(hex::encode(mac.finalize().into_bytes()).into_bytes())
  }

  fn verify(
    &self,
    signature: &[u8],
    frames: &[&[u8]],
  ) -> Result<(), WireError> {
    if self.key.is_empty() {
      return Ok(());
    }

    let decoded = hex::decode(signature)
      .map_err(|_| WireError::InvalidSignatureEncoding)?;

    if hex::encode(&decoded).as_bytes() != signature {
      return Err(WireError::InvalidSignatureEncoding);
    }

    let mut mac = Hmac::<Sha256>::new_from_slice(&self.key)
      .map_err(|_| WireError::InvalidKeyLength)?;

    for frame in frames {
      mac.update(frame);
    }

    mac
      .verify_slice(&decoded)
      .map_err(|_| WireError::BadSignature)
  }
}

fn deserialize<'a, T>(frame: &'a [u8], kind: JsonFrame) -> Result<T, WireError>
where
  T: Deserialize<'a>,
{
  serde_json::from_slice(frame).map_err(|source| WireError::InvalidJson {
    frame: kind,
    source,
  })
}

fn serialize<T>(value: &T, frame: JsonFrame) -> Result<Frame, WireError>
where
  T: Serialize,
{
  serde_json::to_vec(value)
    .map_err(|source| WireError::SerializeJson { frame, source })
}

#[cfg(test)]
mod tests {
  use super::*;

  const BINARY: &str = include_str!("../tests/fixtures/wire-binary.json");
  const MALFORMED: &str = include_str!("../tests/fixtures/wire-malformed.json");
  const TAMPERED: &str = include_str!("../tests/fixtures/wire-tampered.json");
  const UNSIGNED: &str = include_str!("../tests/fixtures/wire-unsigned.json");
  const VALID: &str = include_str!("../tests/fixtures/wire-valid.json");

  #[derive(Deserialize)]
  struct Fixture {
    frames: Vec<String>,
    key: String,
  }

  impl Fixture {
    fn frames(&self) -> Vec<Frame> {
      self
        .frames
        .iter()
        .map(|frame| hex::decode(frame).unwrap())
        .collect()
    }

    fn load(recording: &str) -> Self {
      serde_json::from_str(recording).unwrap()
    }

    fn protocol(&self) -> WireProtocol {
      WireProtocol::new(self.key.as_bytes())
    }
  }

  #[test]
  fn accepts_recorded_message_with_extensions() {
    let fixture = Fixture::load(VALID);
    let envelope = fixture.protocol().decode(&fixture.frames()).unwrap();

    assert_eq!(
      envelope.identities,
      [b"client".to_vec(), b"\0\xff\x7f".to_vec()]
    );
    assert_eq!(
      envelope.header.msg_type,
      MessageType::from("future_request")
    );
    assert_eq!(envelope.header.extra["extension"], true);
    assert_eq!(envelope.metadata["foo"], "bar");
    assert_eq!(envelope.content["extra"]["foo"], true);
  }

  #[test]
  fn empty_key_disables_authentication() {
    let fixture = Fixture::load(UNSIGNED);
    let mut frames = fixture.frames();
    frames[7] = br#"{"tampered":true}"#.to_vec();

    let envelope = fixture.protocol().decode(&frames).unwrap();
    let encoded = fixture.protocol().encode(&envelope).unwrap();

    let delimiter =
      encoded.iter().position(|frame| frame == DELIMITER).unwrap();
    assert!(encoded[delimiter + 1].is_empty());
  }

  #[test]
  fn malformed_recordings_return_typed_errors() {
    let fixture = Fixture::load(MALFORMED);
    assert!(matches!(
      fixture.protocol().decode(&fixture.frames()),
      Err(WireError::MissingDelimiter)
    ));

    let fixture = Fixture::load(UNSIGNED);
    let mut frames = fixture.frames();
    frames.truncate(5);
    assert!(matches!(
      fixture.protocol().decode(&frames),
      Err(WireError::TooFewFrames { actual: 2 })
    ));

    let mut frames = fixture.frames();
    frames[4] = b"{".to_vec();
    assert!(matches!(
      fixture.protocol().decode(&frames),
      Err(WireError::InvalidJson {
        frame: JsonFrame::Header,
        ..
      })
    ));
  }

  #[test]
  fn preserves_routing_identities_and_binary_buffers() {
    let fixture = Fixture::load(BINARY);
    let mut envelope = fixture.protocol().decode(&fixture.frames()).unwrap();

    assert_eq!(
      envelope.identities,
      [b"client".to_vec(), b"\0\xff\x7f".to_vec()]
    );
    assert_eq!(
      envelope.buffers,
      [b"\0\xff\x80foo".to_vec(), b"\x01\x02".to_vec()]
    );

    envelope.parent_header = ParentHeader::Header(envelope.header.clone());

    let encoded = fixture.protocol().encode(&envelope).unwrap();
    assert_eq!(fixture.protocol().decode(&encoded).unwrap(), envelope);
  }

  #[test]
  fn rejects_noncanonical_signature() {
    let fixture = Fixture::load(VALID);
    let mut frames = fixture.frames();
    frames[3].make_ascii_uppercase();

    assert!(matches!(
      fixture.protocol().decode(&frames),
      Err(WireError::InvalidSignatureEncoding)
    ));
  }

  #[test]
  fn rejects_recorded_tampered_message_without_exposing_key() {
    let fixture = Fixture::load(TAMPERED);
    let error = fixture.protocol().decode(&fixture.frames()).unwrap_err();

    assert!(matches!(error, WireError::BadSignature));
    assert!(!error.to_string().contains(&fixture.key));
  }

  #[test]
  fn signs_with_sha256() {
    let fixture = Fixture::load(UNSIGNED);
    let envelope = fixture.protocol().decode(&fixture.frames()).unwrap();
    let protocol = WireProtocol::new(b"foo");
    let frames = protocol.encode(&envelope).unwrap();
    let delimiter = frames.iter().position(|frame| frame == DELIMITER).unwrap();

    assert_eq!(frames[delimiter + 1].len(), 64);
    assert_eq!(protocol.decode(&frames).unwrap(), envelope);
  }
}
