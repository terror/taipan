use {
  hmac::{Hmac, KeyInit, Mac},
  serde::{Deserialize, Serialize, de},
  serde_json::{Map, Value},
  sha1::Sha1,
  sha2::{Sha224, Sha256, Sha384, Sha512},
  std::{fmt, str::FromStr},
  thiserror::Error,
};

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
  Stdin,
}

impl fmt::Display for Channel {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str(match self {
      Self::Control => "control",
      Self::Heartbeat => "heartbeat",
      Self::Iopub => "iopub",
      Self::Shell => "shell",
      Self::Stdin => "stdin",
    })
  }
}

impl FromStr for Channel {
  type Err = WireError;

  fn from_str(value: &str) -> Result<Self, Self::Err> {
    match value {
      "control" => Ok(Self::Control),
      "heartbeat" => Ok(Self::Heartbeat),
      "iopub" => Ok(Self::Iopub),
      "shell" => Ok(Self::Shell),
      "stdin" => Ok(Self::Stdin),
      _ => Err(WireError::UnknownChannel(value.into())),
    }
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

impl fmt::Display for JsonFrame {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
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

impl fmt::Display for MessageType {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignatureScheme {
  HmacSha1,
  HmacSha224,
  HmacSha256,
  HmacSha384,
  HmacSha512,
}

impl fmt::Display for SignatureScheme {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str(match self {
      Self::HmacSha1 => "hmac-sha1",
      Self::HmacSha224 => "hmac-sha224",
      Self::HmacSha256 => "hmac-sha256",
      Self::HmacSha384 => "hmac-sha384",
      Self::HmacSha512 => "hmac-sha512",
    })
  }
}

impl FromStr for SignatureScheme {
  type Err = WireError;

  fn from_str(value: &str) -> Result<Self, Self::Err> {
    match value {
      "hmac-sha1" => Ok(Self::HmacSha1),
      "hmac-sha224" => Ok(Self::HmacSha224),
      "hmac-sha256" => Ok(Self::HmacSha256),
      "hmac-sha384" => Ok(Self::HmacSha384),
      "hmac-sha512" => Ok(Self::HmacSha512),
      _ => Err(WireError::UnsupportedScheme(value.into())),
    }
  }
}

#[derive(Debug, Error)]
pub enum WireError {
  #[error("message signature did not verify")]
  BadSignature,
  #[error("invalid JSON in {frame} frame: {source}")]
  InvalidJson {
    frame: JsonFrame,
    source: serde_json::Error,
  },
  #[error("authentication key has an invalid length")]
  InvalidKeyLength,
  #[error("message signature is not a canonical hexadecimal digest")]
  InvalidSignatureEncoding,
  #[error("message does not contain the <IDS|MSG> delimiter")]
  MissingDelimiter,
  #[error("failed to serialize {frame} frame: {source}")]
  SerializeJson {
    frame: JsonFrame,
    source: serde_json::Error,
  },
  #[error(
    "message has {actual} frame(s) after the delimiter, expected at least 5"
  )]
  TooFewFrames { actual: usize },
  #[error("unknown Jupyter channel `{0}`")]
  UnknownChannel(String),
  #[error("unsupported signature scheme `{0}`")]
  UnsupportedScheme(String),
}

pub struct WireProtocol {
  key: Vec<u8>,
  scheme: SignatureScheme,
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

  pub fn new(
    key: impl Into<Vec<u8>>,
    signature_scheme: &str,
  ) -> Result<Self, WireError> {
    Ok(Self {
      key: key.into(),
      scheme: signature_scheme.parse()?,
    })
  }

  fn sign(&self, frames: &[&[u8]]) -> Result<Frame, WireError> {
    if self.key.is_empty() {
      return Ok(Frame::new());
    }

    macro_rules! sign {
      ($digest:ty) => {{
        let mut mac = Hmac::<$digest>::new_from_slice(&self.key)
          .map_err(|_| WireError::InvalidKeyLength)?;

        for frame in frames {
          mac.update(frame);
        }

        hex::encode(mac.finalize().into_bytes()).into_bytes()
      }};
    }

    Ok(match self.scheme {
      SignatureScheme::HmacSha1 => sign!(Sha1),
      SignatureScheme::HmacSha224 => sign!(Sha224),
      SignatureScheme::HmacSha256 => sign!(Sha256),
      SignatureScheme::HmacSha384 => sign!(Sha384),
      SignatureScheme::HmacSha512 => sign!(Sha512),
    })
  }

  #[must_use]
  pub fn signature_scheme(&self) -> SignatureScheme {
    self.scheme
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

    macro_rules! verify {
      ($digest:ty) => {{
        let mut mac = Hmac::<$digest>::new_from_slice(&self.key)
          .map_err(|_| WireError::InvalidKeyLength)?;

        for frame in frames {
          mac.update(frame);
        }

        mac
          .verify_slice(&decoded)
          .map_err(|_| WireError::BadSignature)
      }};
    }

    match self.scheme {
      SignatureScheme::HmacSha1 => verify!(Sha1),
      SignatureScheme::HmacSha224 => verify!(Sha224),
      SignatureScheme::HmacSha256 => verify!(Sha256),
      SignatureScheme::HmacSha384 => verify!(Sha384),
      SignatureScheme::HmacSha512 => verify!(Sha512),
    }
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
    scheme: String,
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
      WireProtocol::new(self.key.as_bytes(), &self.scheme).unwrap()
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
  fn channel_names() {
    #[track_caller]
    fn case(name: &str, expected: Channel) {
      let channel = name.parse::<Channel>().unwrap();
      assert_eq!(channel, expected);
      assert_eq!(channel.to_string(), name);
    }

    case("control", Channel::Control);
    case("heartbeat", Channel::Heartbeat);
    case("iopub", Channel::Iopub);
    case("shell", Channel::Shell);
    case("stdin", Channel::Stdin);

    assert!(matches!(
      "foo".parse::<Channel>(),
      Err(WireError::UnknownChannel(channel)) if channel == "foo"
    ));
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
  fn signature_schemes() {
    #[track_caller]
    fn case(scheme: &str, signature_length: usize) {
      let fixture = Fixture::load(UNSIGNED);
      let envelope = fixture.protocol().decode(&fixture.frames()).unwrap();
      let protocol = WireProtocol::new(b"foo", scheme).unwrap();
      let frames = protocol.encode(&envelope).unwrap();
      let delimiter =
        frames.iter().position(|frame| frame == DELIMITER).unwrap();

      assert_eq!(frames[delimiter + 1].len(), signature_length);
      assert_eq!(protocol.decode(&frames).unwrap(), envelope);
    }

    case("hmac-sha1", 40);
    case("hmac-sha224", 56);
    case("hmac-sha256", 64);
    case("hmac-sha384", 96);
    case("hmac-sha512", 128);

    assert!(matches!(
      WireProtocol::new(b"foo", "hmac-foo"),
      Err(WireError::UnsupportedScheme(scheme)) if scheme == "hmac-foo"
    ));
  }
}
