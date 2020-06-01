use thiserror::Error;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("AEAD encryption for {0} failed")]
    AEADEncryption(String),
    #[error("AEAD decryption for {0} failed")]
    AEADDecryption(String),
    #[error("the following error occured when serializing '{0}': {1:?}")]
    Serialization(String, bincode::ErrorKind),
    #[error("the following error occured when deserializing '{0}': {1:?}")]
    Deserialization(String, bincode::ErrorKind),
    #[error("rejected message with too many skipped messages")]
    TooManySkippedMessages,
    #[error("received a DoubleRatchetMessage with Double Ratchet uninitialized")]
    UnreadableDoubleRatchetMessage,
}
