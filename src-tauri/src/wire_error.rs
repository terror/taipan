use super::*;

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
}
