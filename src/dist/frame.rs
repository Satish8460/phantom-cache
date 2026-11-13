use std::fmt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Magic bytes for the PhantomMesh binary framing protocol: "PHAN" (0x50 0x48 0x41 0x4E).
pub const MAGIC: [u8; 4] = [0x50, 0x48, 0x41, 0x4E];

/// Protocol version 1.
pub const VERSION_1: u8 = 1;

/// Fixed header size in bytes.
pub const HEADER_LEN: usize = 28;

/// Maximum allowable payload size (1 MB = 1,048,576 bytes).
pub const MAX_FRAME_PAYLOAD_BYTES: u32 = 1_048_576;

/// Conservative maximum key size for cache operations (4 KB = 4,096 bytes).
pub const MAX_KEY_LENGTH_BYTES: usize = 4_096;

/// Message types supported by the PhantomMesh inter-node protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    GetRequest = 0x01,
    GetResponse = 0x02,
    InvalidateRequest = 0x03,
    InvalidateAck = 0x04,
    HeartbeatRequest = 0x05,
    HeartbeatResponse = 0x06,
    AntiEntropySyncRequest = 0x07,
    AntiEntropySyncResponse = 0x08,
    PeerReadRequest = 0x09,
    PeerReadResponse = 0x0A,
}

impl MessageType {
    pub fn from_u8(val: u8) -> Result<Self, FrameError> {
        match val {
            0x01 => Ok(Self::GetRequest),
            0x02 => Ok(Self::GetResponse),
            0x03 => Ok(Self::InvalidateRequest),
            0x04 => Ok(Self::InvalidateAck),
            0x05 => Ok(Self::HeartbeatRequest),
            0x06 => Ok(Self::HeartbeatResponse),
            0x07 => Ok(Self::AntiEntropySyncRequest),
            0x08 => Ok(Self::AntiEntropySyncResponse),
            0x09 => Ok(Self::PeerReadRequest),
            0x0A => Ok(Self::PeerReadResponse),
            other => Err(FrameError::UnknownMessageType(other)),
        }
    }

    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

/// Invalidation scopes supported by the PhantomMesh invalidation protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum InvalidationScope {
    ExactKey = 0x00,
    Tenant = 0x01,
    FullCache = 0x02,
}

impl InvalidationScope {
    pub fn from_u8(val: u8) -> Result<Self, FrameError> {
        match val {
            0x00 => Ok(Self::ExactKey),
            0x01 => Ok(Self::Tenant),
            0x02 => Ok(Self::FullCache),
            other => Err(FrameError::InvalidScope(other)),
        }
    }

    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

/// Invalidation execution status returned in an InvalidateAck frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum InvalidationStatus {
    SuccessEvicted = 0x00,
    SuccessNotPresent = 0x01,
    IgnoredObsolete = 0x02,
    RejectedRateLimit = 0x03,
    InternalError = 0x04,
}

impl InvalidationStatus {
    pub fn from_u8(val: u8) -> Result<Self, FrameError> {
        match val {
            0x00 => Ok(Self::SuccessEvicted),
            0x01 => Ok(Self::SuccessNotPresent),
            0x02 => Ok(Self::IgnoredObsolete),
            0x03 => Ok(Self::RejectedRateLimit),
            0x04 => Ok(Self::InternalError),
            other => Err(FrameError::InvalidStatus(other)),
        }
    }

    pub fn to_u8(self) -> u8 {
        self as u8
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Self::SuccessEvicted | Self::SuccessNotPresent)
    }
}

/// Status of anti-entropy sequence synchronization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AntiEntropyStatus {
    InSync = 0x00,
    ReplayBatch = 0x01,
    HistoryOverwritten = 0x02,
}

impl AntiEntropyStatus {
    pub fn from_u8(val: u8) -> Result<Self, FrameError> {
        match val {
            0x00 => Ok(Self::InSync),
            0x01 => Ok(Self::ReplayBatch),
            0x02 => Ok(Self::HistoryOverwritten),
            other => Err(FrameError::InvalidAntiEntropyStatus(other)),
        }
    }

    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

/// Represents an individual cache invalidation record recorded in the ring buffer
/// and exchanged during anti-entropy replay batches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidateRecord {
    pub stream_seq: u64,
    pub scope: InvalidationScope,
    pub key: String,
}

/// Minimum size of an InvalidateRequest payload: 8 (epoch) + 8 (seq) + 1 (orig) + 1 (scope) + 2 (key_len) = 20 bytes.
pub const MIN_INVALIDATE_REQUEST_PAYLOAD: usize = 20;

/// Maximum size of an InvalidateRequest payload: 20 + 4096 = 4116 bytes.
pub const MAX_INVALIDATE_REQUEST_PAYLOAD: usize = 20 + MAX_KEY_LENGTH_BYTES;

/// Exact size of an InvalidateAck payload: 8 (epoch) + 8 (seq) + 1 (resp) + 1 (status) + 2 (reserved) + 4 (reclaimed) = 24 bytes.
pub const INVALIDATE_ACK_PAYLOAD_LEN: usize = 24;

/// Exact size of an AntiEntropySyncRequest payload: 8 (epoch) + 8 (seq) + 1 (orig) + 1 (reserved) + 32 (digest) = 50 bytes.
pub const ANTI_ENTROPY_SYNC_REQUEST_PAYLOAD_LEN: usize = 50;

/// Minimum size of an AntiEntropySyncResponse payload: 1 (status) + 1 (reserved) + 8 (start) + 8 (end) + 2 (count) = 20 bytes.
pub const MIN_ANTI_ENTROPY_SYNC_RESPONSE_PAYLOAD: usize = 20;

/// Decoded representation of an InvalidateRequest frame payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidateRequestPayload {
    pub membership_epoch: u64,
    pub stream_seq: u64,
    pub originator_id: u8,
    pub scope: InvalidationScope,
    pub key: String,
}

/// Decoded representation of an InvalidateAck frame payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidateAckPayload {
    pub membership_epoch: u64,
    pub stream_seq: u64,
    pub responder_id: u8,
    pub status: InvalidationStatus,
    pub reclaimed_bytes: u32,
}

/// Decoded representation of an AntiEntropySyncRequest frame payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AntiEntropySyncRequestPayload {
    pub membership_epoch: u64,
    pub highest_acked_seq: u64,
    pub sender_node_id: u8,
    pub ring_digest: [u8; 32],
}

/// Decoded representation of an AntiEntropySyncResponse frame payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AntiEntropySyncResponsePayload {
    pub status: AntiEntropyStatus,
    pub range_start: u64,
    pub range_end: u64,
    pub record_count: u16,
    pub records: Vec<InvalidateRecord>,
}

/// Errors encountered when encoding, decoding, or validating PhantomMesh frames.
#[derive(Debug, PartialEq, Eq)]
pub enum FrameError {
    BadMagic([u8; 4]),
    UnsupportedVersion(u8),
    UnknownMessageType(u8),
    PayloadTooLarge { length: u32, max: u32 },
    KeyTooLarge { length: usize, max: usize },
    InvalidKeyUtf8,
    InvalidScope(u8),
    InvalidStatus(u8),
    InvalidAntiEntropyStatus(u8),
    InvalidPayloadLength { expected: usize, received: usize },
    TruncatedHeader { expected: usize, received: usize },
    TruncatedPayload { expected: u32, received: usize },
    ConnectionClosed,
    Io(std::io::ErrorKind),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadMagic(bytes) => write!(
                f,
                "Invalid frame magic: {:02x?} (expected 'PHAN')",
                bytes
            ),
            Self::UnsupportedVersion(v) => write!(
                f,
                "Unsupported frame version: {} (expected {})",
                v, VERSION_1
            ),
            Self::UnknownMessageType(t) => write!(f, "Unknown message type: 0x{:02x}", t),
            Self::PayloadTooLarge { length, max } => write!(
                f,
                "Frame payload size {} exceeds limit of {} bytes",
                length, max
            ),
            Self::KeyTooLarge { length, max } => write!(
                f,
                "Key size {} exceeds limit of {} bytes",
                length, max
            ),
            Self::InvalidKeyUtf8 => write!(f, "Key is not valid UTF-8"),
            Self::InvalidScope(s) => write!(f, "Invalid invalidation scope: 0x{:02x}", s),
            Self::InvalidStatus(s) => write!(f, "Invalid invalidation status: 0x{:02x}", s),
            Self::InvalidAntiEntropyStatus(s) => write!(f, "Invalid anti-entropy status: 0x{:02x}", s),
            Self::InvalidPayloadLength { expected, received } => write!(
                f,
                "Invalid payload length: received {} bytes, expected {}",
                received, expected
            ),
            Self::TruncatedHeader { expected, received } => write!(
                f,
                "Truncated header: received {} bytes, expected {}",
                received, expected
            ),
            Self::TruncatedPayload { expected, received } => write!(
                f,
                "Truncated payload: received {} bytes, expected {}",
                received, expected
            ),
            Self::ConnectionClosed => write!(f, "Connection closed cleanly before frame start"),
            Self::Io(kind) => write!(f, "I/O error during frame operation: {:?}", kind),
        }
    }
}

impl std::error::Error for FrameError {}

impl From<std::io::Error> for FrameError {
    fn from(err: std::io::Error) -> Self {
        if err.kind() == std::io::ErrorKind::UnexpectedEof {
            FrameError::ConnectionClosed
        } else {
            FrameError::Io(err.kind())
        }
    }
}

/// 28-byte header representing the wire layout of a PhantomMesh frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameHeader {
    pub magic: [u8; 4],
    pub version: u8,
    pub msg_type: MessageType,
    pub flags: u16,
    pub request_id: u64,
    pub sequence_number: u64,
    pub payload_length: u32,
}

impl FrameHeader {
    /// Serializes the 28-byte header into network byte order (Big-Endian).
    pub fn encode(&self) -> [u8; HEADER_LEN] {
        let mut buf = [0u8; HEADER_LEN];
        buf[0..4].copy_from_slice(&self.magic);
        buf[4] = self.version;
        buf[5] = self.msg_type.to_u8();
        buf[6..8].copy_from_slice(&self.flags.to_be_bytes());
        buf[8..16].copy_from_slice(&self.request_id.to_be_bytes());
        buf[16..24].copy_from_slice(&self.sequence_number.to_be_bytes());
        buf[24..28].copy_from_slice(&self.payload_length.to_be_bytes());
        buf
    }

    /// Validates and parses a 28-byte raw buffer without heap allocations.
    pub fn decode(buf: &[u8; HEADER_LEN]) -> Result<Self, FrameError> {
        let magic: [u8; 4] = [buf[0], buf[1], buf[2], buf[3]];
        if magic != MAGIC {
            return Err(FrameError::BadMagic(magic));
        }

        let version = buf[4];
        if version != VERSION_1 {
            return Err(FrameError::UnsupportedVersion(version));
        }

        let msg_type = MessageType::from_u8(buf[5])?;
        let flags = u16::from_be_bytes([buf[6], buf[7]]);
        let request_id = u64::from_be_bytes([
            buf[8], buf[9], buf[10], buf[11], buf[12], buf[13], buf[14], buf[15],
        ]);
        let sequence_number = u64::from_be_bytes([
            buf[16], buf[17], buf[18], buf[19], buf[20], buf[21], buf[22], buf[23],
        ]);
        let payload_length = u32::from_be_bytes([buf[24], buf[25], buf[26], buf[27]]);

        if payload_length > MAX_FRAME_PAYLOAD_BYTES {
            return Err(FrameError::PayloadTooLarge {
                length: payload_length,
                max: MAX_FRAME_PAYLOAD_BYTES,
            });
        }

        Ok(Self {
            magic,
            version,
            msg_type,
            flags,
            request_id,
            sequence_number,
            payload_length,
        })
    }
}

/// In-memory representation of a validated PhantomMesh protocol frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhantomMeshFrame {
    pub header: FrameHeader,
    pub payload: Vec<u8>,
}

impl PhantomMeshFrame {
    /// Creates a new frame, validating that the payload does not exceed the ceiling.
    pub fn new(
        msg_type: MessageType,
        flags: u16,
        request_id: u64,
        sequence_number: u64,
        payload: Vec<u8>,
    ) -> Result<Self, FrameError> {
        let payload_len = payload.len();
        if payload_len > MAX_FRAME_PAYLOAD_BYTES as usize {
            return Err(FrameError::PayloadTooLarge {
                length: payload_len as u32,
                max: MAX_FRAME_PAYLOAD_BYTES,
            });
        }

        let header = FrameHeader {
            magic: MAGIC,
            version: VERSION_1,
            msg_type,
            flags,
            request_id,
            sequence_number,
            payload_length: payload_len as u32,
        };

        Ok(Self { header, payload })
    }

    /// Convenience constructor for a GetRequest frame.
    pub fn new_get_request(
        request_id: u64,
        sequence_number: u64,
        key: &str,
    ) -> Result<Self, FrameError> {
        let key_bytes = key.as_bytes();
        if key_bytes.len() > MAX_KEY_LENGTH_BYTES {
            return Err(FrameError::KeyTooLarge {
                length: key_bytes.len(),
                max: MAX_KEY_LENGTH_BYTES,
            });
        }

        Self::new(
            MessageType::GetRequest,
            0,
            request_id,
            sequence_number,
            key_bytes.to_vec(),
        )
    }

    /// Convenience constructor for a GetResponse frame.
    pub fn new_get_response(
        request_id: u64,
        sequence_number: u64,
        response_body: &str,
    ) -> Result<Self, FrameError> {
        Self::new(
            MessageType::GetResponse,
            0,
            request_id,
            sequence_number,
            response_body.as_bytes().to_vec(),
        )
    }

    /// Extracts the key string from a GetRequest payload, enforcing UTF-8 and length limits.
    pub fn extract_key(&self) -> Result<&str, FrameError> {
        if self.header.msg_type != MessageType::GetRequest {
            return Err(FrameError::UnknownMessageType(self.header.msg_type.to_u8()));
        }
        if self.payload.len() > MAX_KEY_LENGTH_BYTES {
            return Err(FrameError::KeyTooLarge {
                length: self.payload.len(),
                max: MAX_KEY_LENGTH_BYTES,
            });
        }
        std::str::from_utf8(&self.payload).map_err(|_| FrameError::InvalidKeyUtf8)
    }

    /// Extracts the response string from a GetResponse payload.
    pub fn extract_response(&self) -> Result<&str, FrameError> {
        if self.header.msg_type != MessageType::GetResponse {
            return Err(FrameError::UnknownMessageType(self.header.msg_type.to_u8()));
        }
        std::str::from_utf8(&self.payload).map_err(|_| FrameError::InvalidKeyUtf8)
    }

    /// Convenience constructor for an InvalidateRequest frame.
    #[allow(clippy::too_many_arguments)]
    pub fn new_invalidate_request(
        request_id: u64,
        sequence_number: u64,
        membership_epoch: u64,
        stream_seq: u64,
        originator_id: u8,
        scope: InvalidationScope,
        key: &str,
    ) -> Result<Self, FrameError> {
        let key_bytes = key.as_bytes();
        if key_bytes.len() > MAX_KEY_LENGTH_BYTES {
            return Err(FrameError::KeyTooLarge {
                length: key_bytes.len(),
                max: MAX_KEY_LENGTH_BYTES,
            });
        }

        let mut payload = Vec::with_capacity(MIN_INVALIDATE_REQUEST_PAYLOAD + key_bytes.len());
        payload.extend_from_slice(&membership_epoch.to_be_bytes());
        payload.extend_from_slice(&stream_seq.to_be_bytes());
        payload.push(originator_id);
        payload.push(scope.to_u8());
        payload.extend_from_slice(&(key_bytes.len() as u16).to_be_bytes());
        payload.extend_from_slice(key_bytes);

        Self::new(
            MessageType::InvalidateRequest,
            0,
            request_id,
            sequence_number,
            payload,
        )
    }

    /// Decodes an InvalidateRequest payload, validating size bounds, scope, and key UTF-8.
    pub fn extract_invalidate_request(&self) -> Result<InvalidateRequestPayload, FrameError> {
        if self.header.msg_type != MessageType::InvalidateRequest {
            return Err(FrameError::UnknownMessageType(self.header.msg_type.to_u8()));
        }

        let payload_len = self.payload.len();
        if payload_len < MIN_INVALIDATE_REQUEST_PAYLOAD {
            return Err(FrameError::TruncatedPayload {
                expected: MIN_INVALIDATE_REQUEST_PAYLOAD as u32,
                received: payload_len,
            });
        }
        if payload_len > MAX_INVALIDATE_REQUEST_PAYLOAD {
            return Err(FrameError::PayloadTooLarge {
                length: payload_len as u32,
                max: MAX_INVALIDATE_REQUEST_PAYLOAD as u32,
            });
        }

        let membership_epoch = u64::from_be_bytes([
            self.payload[0], self.payload[1], self.payload[2], self.payload[3],
            self.payload[4], self.payload[5], self.payload[6], self.payload[7],
        ]);
        let stream_seq = u64::from_be_bytes([
            self.payload[8], self.payload[9], self.payload[10], self.payload[11],
            self.payload[12], self.payload[13], self.payload[14], self.payload[15],
        ]);
        let originator_id = self.payload[16];
        let scope = InvalidationScope::from_u8(self.payload[17])?;
        let key_len = u16::from_be_bytes([self.payload[18], self.payload[19]]) as usize;

        if payload_len != MIN_INVALIDATE_REQUEST_PAYLOAD + key_len {
            return Err(FrameError::InvalidPayloadLength {
                expected: MIN_INVALIDATE_REQUEST_PAYLOAD + key_len,
                received: payload_len,
            });
        }

        let key = std::str::from_utf8(&self.payload[20..20 + key_len])
            .map_err(|_| FrameError::InvalidKeyUtf8)?
            .to_string();

        Ok(InvalidateRequestPayload {
            membership_epoch,
            stream_seq,
            originator_id,
            scope,
            key,
        })
    }

    /// Convenience constructor for an InvalidateAck frame.
    #[allow(clippy::too_many_arguments)]
    pub fn new_invalidate_ack(
        request_id: u64,
        sequence_number: u64,
        membership_epoch: u64,
        stream_seq: u64,
        responder_id: u8,
        status: InvalidationStatus,
        reclaimed_bytes: u32,
    ) -> Result<Self, FrameError> {
        let mut payload = Vec::with_capacity(INVALIDATE_ACK_PAYLOAD_LEN);
        payload.extend_from_slice(&membership_epoch.to_be_bytes());
        payload.extend_from_slice(&stream_seq.to_be_bytes());
        payload.push(responder_id);
        payload.push(status.to_u8());
        payload.extend_from_slice(&0u16.to_be_bytes()); // reserved
        payload.extend_from_slice(&reclaimed_bytes.to_be_bytes());

        Self::new(
            MessageType::InvalidateAck,
            0,
            request_id,
            sequence_number,
            payload,
        )
    }

    /// Decodes an InvalidateAck payload, enforcing exact 24-byte layout and status validity.
    pub fn extract_invalidate_ack(&self) -> Result<InvalidateAckPayload, FrameError> {
        if self.header.msg_type != MessageType::InvalidateAck {
            return Err(FrameError::UnknownMessageType(self.header.msg_type.to_u8()));
        }

        let payload_len = self.payload.len();
        if payload_len != INVALIDATE_ACK_PAYLOAD_LEN {
            return Err(FrameError::InvalidPayloadLength {
                expected: INVALIDATE_ACK_PAYLOAD_LEN,
                received: payload_len,
            });
        }

        let membership_epoch = u64::from_be_bytes([
            self.payload[0], self.payload[1], self.payload[2], self.payload[3],
            self.payload[4], self.payload[5], self.payload[6], self.payload[7],
        ]);
        let stream_seq = u64::from_be_bytes([
            self.payload[8], self.payload[9], self.payload[10], self.payload[11],
            self.payload[12], self.payload[13], self.payload[14], self.payload[15],
        ]);
        let responder_id = self.payload[16];
        let status = InvalidationStatus::from_u8(self.payload[17])?;
        // self.payload[18..20] is reserved
        let reclaimed_bytes = u32::from_be_bytes([
            self.payload[20], self.payload[21], self.payload[22], self.payload[23],
        ]);

        Ok(InvalidateAckPayload {
            membership_epoch,
            stream_seq,
            responder_id,
            status,
            reclaimed_bytes,
        })
    }

    /// Convenience constructor for an AntiEntropySyncRequest frame.
    pub fn new_anti_entropy_sync_request(
        request_id: u64,
        sequence_number: u64,
        membership_epoch: u64,
        highest_acked_seq: u64,
        sender_node_id: u8,
        ring_digest: [u8; 32],
    ) -> Result<Self, FrameError> {
        let mut payload = Vec::with_capacity(ANTI_ENTROPY_SYNC_REQUEST_PAYLOAD_LEN);
        payload.extend_from_slice(&membership_epoch.to_be_bytes());
        payload.extend_from_slice(&highest_acked_seq.to_be_bytes());
        payload.push(sender_node_id);
        payload.push(0u8); // reserved
        payload.extend_from_slice(&ring_digest);

        Self::new(
            MessageType::AntiEntropySyncRequest,
            0,
            request_id,
            sequence_number,
            payload,
        )
    }

    /// Decodes an AntiEntropySyncRequest payload, validating exact 50-byte size.
    pub fn extract_anti_entropy_sync_request(&self) -> Result<AntiEntropySyncRequestPayload, FrameError> {
        if self.header.msg_type != MessageType::AntiEntropySyncRequest {
            return Err(FrameError::UnknownMessageType(self.header.msg_type.to_u8()));
        }

        let payload_len = self.payload.len();
        if payload_len != ANTI_ENTROPY_SYNC_REQUEST_PAYLOAD_LEN {
            return Err(FrameError::InvalidPayloadLength {
                expected: ANTI_ENTROPY_SYNC_REQUEST_PAYLOAD_LEN,
                received: payload_len,
            });
        }

        let membership_epoch = u64::from_be_bytes([
            self.payload[0], self.payload[1], self.payload[2], self.payload[3],
            self.payload[4], self.payload[5], self.payload[6], self.payload[7],
        ]);
        let highest_acked_seq = u64::from_be_bytes([
            self.payload[8], self.payload[9], self.payload[10], self.payload[11],
            self.payload[12], self.payload[13], self.payload[14], self.payload[15],
        ]);
        let sender_node_id = self.payload[16];
        // self.payload[17] is reserved
        let mut ring_digest = [0u8; 32];
        ring_digest.copy_from_slice(&self.payload[18..50]);

        Ok(AntiEntropySyncRequestPayload {
            membership_epoch,
            highest_acked_seq,
            sender_node_id,
            ring_digest,
        })
    }

    /// Convenience constructor for an AntiEntropySyncResponse frame.
    pub fn new_anti_entropy_sync_response(
        request_id: u64,
        sequence_number: u64,
        status: AntiEntropyStatus,
        range_start: u64,
        range_end: u64,
        records: &[InvalidateRecord],
    ) -> Result<Self, FrameError> {
        let mut payload = Vec::with_capacity(MIN_ANTI_ENTROPY_SYNC_RESPONSE_PAYLOAD + records.len() * 32);
        payload.push(status.to_u8());
        payload.push(0u8); // reserved
        payload.extend_from_slice(&range_start.to_be_bytes());
        payload.extend_from_slice(&range_end.to_be_bytes());
        payload.extend_from_slice(&(records.len() as u16).to_be_bytes());

        for rec in records {
            let key_bytes = rec.key.as_bytes();
            if key_bytes.len() > MAX_KEY_LENGTH_BYTES {
                return Err(FrameError::KeyTooLarge {
                    length: key_bytes.len(),
                    max: MAX_KEY_LENGTH_BYTES,
                });
            }
            payload.extend_from_slice(&rec.stream_seq.to_be_bytes());
            payload.push(rec.scope.to_u8());
            payload.extend_from_slice(&(key_bytes.len() as u16).to_be_bytes());
            payload.extend_from_slice(key_bytes);
        }

        if payload.len() as u32 > MAX_FRAME_PAYLOAD_BYTES {
            return Err(FrameError::PayloadTooLarge {
                length: payload.len() as u32,
                max: MAX_FRAME_PAYLOAD_BYTES,
            });
        }

        Self::new(
            MessageType::AntiEntropySyncResponse,
            0,
            request_id,
            sequence_number,
            payload,
        )
    }

    /// Decodes an AntiEntropySyncResponse payload, enforcing length bounds and layout.
    pub fn extract_anti_entropy_sync_response(&self) -> Result<AntiEntropySyncResponsePayload, FrameError> {
        if self.header.msg_type != MessageType::AntiEntropySyncResponse {
            return Err(FrameError::UnknownMessageType(self.header.msg_type.to_u8()));
        }

        let payload_len = self.payload.len();
        if payload_len < MIN_ANTI_ENTROPY_SYNC_RESPONSE_PAYLOAD {
            return Err(FrameError::TruncatedPayload {
                expected: MIN_ANTI_ENTROPY_SYNC_RESPONSE_PAYLOAD as u32,
                received: payload_len,
            });
        }
        if payload_len as u32 > MAX_FRAME_PAYLOAD_BYTES {
            return Err(FrameError::PayloadTooLarge {
                length: payload_len as u32,
                max: MAX_FRAME_PAYLOAD_BYTES,
            });
        }

        let status = AntiEntropyStatus::from_u8(self.payload[0])?;
        // self.payload[1] is reserved
        let range_start = u64::from_be_bytes([
            self.payload[2], self.payload[3], self.payload[4], self.payload[5],
            self.payload[6], self.payload[7], self.payload[8], self.payload[9],
        ]);
        let range_end = u64::from_be_bytes([
            self.payload[10], self.payload[11], self.payload[12], self.payload[13],
            self.payload[14], self.payload[15], self.payload[16], self.payload[17],
        ]);
        let record_count = u16::from_be_bytes([self.payload[18], self.payload[19]]) as usize;

        let mut records = Vec::with_capacity(record_count);
        let mut cursor = 20;

        for _ in 0..record_count {
            if cursor + 11 > payload_len {
                return Err(FrameError::TruncatedPayload {
                    expected: (cursor + 11) as u32,
                    received: payload_len,
                });
            }
            let stream_seq = u64::from_be_bytes([
                self.payload[cursor], self.payload[cursor + 1], self.payload[cursor + 2], self.payload[cursor + 3],
                self.payload[cursor + 4], self.payload[cursor + 5], self.payload[cursor + 6], self.payload[cursor + 7],
            ]);
            let scope = InvalidationScope::from_u8(self.payload[cursor + 8])?;
            let key_len = u16::from_be_bytes([self.payload[cursor + 9], self.payload[cursor + 10]]) as usize;
            cursor += 11;

            if cursor + key_len > payload_len {
                return Err(FrameError::TruncatedPayload {
                    expected: (cursor + key_len) as u32,
                    received: payload_len,
                });
            }
            let key = std::str::from_utf8(&self.payload[cursor..cursor + key_len])
                .map_err(|_| FrameError::InvalidKeyUtf8)?
                .to_string();
            cursor += key_len;

            records.push(InvalidateRecord {
                stream_seq,
                scope,
                key,
            });
        }

        if cursor != payload_len {
            return Err(FrameError::InvalidPayloadLength {
                expected: cursor,
                received: payload_len,
            });
        }

        Ok(AntiEntropySyncResponsePayload {
            status,
            range_start,
            range_end,
            record_count: record_count as u16,
            records,
        })
    }
}

/// Reads a single complete frame from an asynchronous stream.
///
/// Safety guarantees:
/// 1. Reads the exact 28-byte header into a fixed stack array.
/// 2. Validates magic, version, message type, and payload length BEFORE heap allocation.
/// 3. Returns `FrameError::ConnectionClosed` if 0 bytes are read at the start.
/// 4. Returns `FrameError::TruncatedHeader` if stream ends before 28 bytes.
/// 5. Allocates exactly `payload_length` bytes and reads until filled or EOF.
pub async fn read_frame<R>(reader: &mut R) -> Result<PhantomMeshFrame, FrameError>
where
    R: AsyncReadExt + Unpin,
{
    let mut header_buf = [0u8; HEADER_LEN];

    // Read the first byte to detect clean connection closure vs mid-header truncation
    let mut bytes_read = 0;
    while bytes_read < HEADER_LEN {
        match reader.read(&mut header_buf[bytes_read..]).await {
            Ok(0) => {
                if bytes_read == 0 {
                    return Err(FrameError::ConnectionClosed);
                } else {
                    return Err(FrameError::TruncatedHeader {
                        expected: HEADER_LEN,
                        received: bytes_read,
                    });
                }
            }
            Ok(n) => {
                bytes_read += n;
                // Fail-fast on invalid magic as soon as 4 bytes are received (e.g. legacy ASCII "GET ")
                if bytes_read >= 4 && header_buf[0..4] != MAGIC {
                    let mut bad_magic = [0u8; 4];
                    bad_magic.copy_from_slice(&header_buf[0..4]);
                    return Err(FrameError::BadMagic(bad_magic));
                }
            }
            Err(e) => return Err(FrameError::from(e)),
        }
    }

    // Decode and validate header (zero heap allocations)
    let header = FrameHeader::decode(&header_buf)?;

    let payload_len = header.payload_length as usize;
    if payload_len == 0 {
        return Ok(PhantomMeshFrame {
            header,
            payload: Vec::new(),
        });
    }

    // Allocate bounded payload buffer
    let mut payload = vec![0u8; payload_len];
    let mut payload_read = 0;

    while payload_read < payload_len {
        match reader.read(&mut payload[payload_read..]).await {
            Ok(0) => {
                return Err(FrameError::TruncatedPayload {
                    expected: header.payload_length,
                    received: payload_read,
                });
            }
            Ok(n) => {
                payload_read += n;
            }
            Err(e) => return Err(FrameError::from(e)),
        }
    }

    Ok(PhantomMeshFrame { header, payload })
}

/// Writes a single complete frame to an asynchronous stream and flushes.
pub async fn write_frame<W>(writer: &mut W, frame: &PhantomMeshFrame) -> Result<(), FrameError>
where
    W: AsyncWriteExt + Unpin,
{
    let header_bytes = frame.header.encode();
    writer.write_all(&header_bytes).await.map_err(FrameError::from)?;
    if !frame.payload.is_empty() {
        writer.write_all(&frame.payload).await.map_err(FrameError::from)?;
    }
    writer.flush().await.map_err(FrameError::from)?;
    Ok(())
}


// ---------------------------------------------------------------------------------------------
// Peer read with full resource metadata (PC-AUTO-041 / 042).
//
// The legacy GetRequest/GetResponse pair returns only a body string ("HIT <body>"), which loses the
// status, Content-Type, ETag, validators and the entry age. PeerReadRequest/PeerReadResponse carry
// all of them. Time is sent as RELATIVE durations (age and remaining lifetime), never as absolute
// timestamps, so the result does not depend on the two hosts clocks.
//
// Version guard: both payloads start with a format byte (`PEER_ENTRY_FORMAT`). A node that does not
// know the message type closes the connection (unsupported message type); a node that knows the type
// but not the format answers `PeerReadKind::Unsupported`. The asking gateway then refuses the answer
// and falls back to the origin; it never guesses at a layout.
// ---------------------------------------------------------------------------------------------

/// Current layout of the peer-read payloads.
pub const PEER_ENTRY_FORMAT: u8 = 1;

/// What a peer answered to a peer read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PeerReadKind {
    /// The peer holds no fresh entry for the key (or will not serve it: Recovering/Suspect, too large).
    Miss = 0,
    /// A fresh entry follows.
    Hit = 1,
    /// The peer does not understand the requested format.
    Unsupported = 2,
}

/// A cache entry as transferred between peers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerEntry {
    pub status: u16,
    pub content_type: String,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub must_revalidate: bool,
    pub stale_while_revalidate_ms: Option<u64>,
    pub stale_if_error_ms: Option<u64>,
    pub date_ms: Option<u64>,
    pub initial_age_ms: u64,
    /// Time since the entry was created at the source, at the moment the source answered.
    pub age_ms: u64,
    /// Remaining freshness lifetime at the moment the source answered.
    pub remaining_ms: u64,
    pub body: Vec<u8>,
}

const PEER_FLAG_MUST_REVALIDATE: u8 = 1 << 0;
const PEER_FLAG_ETAG: u8 = 1 << 1;
const PEER_FLAG_LAST_MODIFIED: u8 = 1 << 2;
const PEER_FLAG_SWR: u8 = 1 << 3;
const PEER_FLAG_SIE: u8 = 1 << 4;
const PEER_FLAG_DATE: u8 = 1 << 5;
/// Fixed part of a hit payload: format, kind, status, flags, 6 x u64, 3 x u16 lengths.
const PEER_HIT_FIXED_LEN: usize = 1 + 1 + 2 + 1 + 6 * 8 + 3 * 2;

impl PeerEntry {
    /// Encodes a hit payload. Fails if the whole payload would exceed the frame limit or a string is
    /// longer than 65535 bytes.
    pub fn encode_hit(&self) -> Result<Vec<u8>, FrameError> {
        let ct = self.content_type.as_bytes();
        let etag = self.etag.as_deref().unwrap_or("").as_bytes();
        let lm = self.last_modified.as_deref().unwrap_or("").as_bytes();
        for s in [ct, etag, lm] {
            if s.len() > u16::MAX as usize {
                return Err(FrameError::PayloadTooLarge { length: s.len() as u32, max: u16::MAX as u32 });
            }
        }
        let total = PEER_HIT_FIXED_LEN + ct.len() + etag.len() + lm.len() + self.body.len();
        if total > MAX_FRAME_PAYLOAD_BYTES as usize {
            return Err(FrameError::PayloadTooLarge { length: total as u32, max: MAX_FRAME_PAYLOAD_BYTES });
        }
        let mut flags = 0u8;
        if self.must_revalidate { flags |= PEER_FLAG_MUST_REVALIDATE; }
        if self.etag.is_some() { flags |= PEER_FLAG_ETAG; }
        if self.last_modified.is_some() { flags |= PEER_FLAG_LAST_MODIFIED; }
        if self.stale_while_revalidate_ms.is_some() { flags |= PEER_FLAG_SWR; }
        if self.stale_if_error_ms.is_some() { flags |= PEER_FLAG_SIE; }
        if self.date_ms.is_some() { flags |= PEER_FLAG_DATE; }
        let mut out = Vec::with_capacity(total);
        out.push(PEER_ENTRY_FORMAT);
        out.push(PeerReadKind::Hit as u8);
        out.extend_from_slice(&self.status.to_be_bytes());
        out.push(flags);
        out.extend_from_slice(&self.age_ms.to_be_bytes());
        out.extend_from_slice(&self.remaining_ms.to_be_bytes());
        out.extend_from_slice(&self.stale_while_revalidate_ms.unwrap_or(0).to_be_bytes());
        out.extend_from_slice(&self.stale_if_error_ms.unwrap_or(0).to_be_bytes());
        out.extend_from_slice(&self.date_ms.unwrap_or(0).to_be_bytes());
        out.extend_from_slice(&self.initial_age_ms.to_be_bytes());
        out.extend_from_slice(&(ct.len() as u16).to_be_bytes());
        out.extend_from_slice(&(etag.len() as u16).to_be_bytes());
        out.extend_from_slice(&(lm.len() as u16).to_be_bytes());
        out.extend_from_slice(ct);
        out.extend_from_slice(etag);
        out.extend_from_slice(lm);
        out.extend_from_slice(&self.body);
        Ok(out)
    }

    /// Decodes a PeerReadResponse payload. `Ok(None)` is a Miss. A format this build does not
    /// speak, or an `Unsupported` answer, is an error: the caller must not interpret the bytes.
    pub fn decode_response(payload: &[u8]) -> Result<Option<PeerEntry>, FrameError> {
        if payload.len() < 2 {
            return Err(FrameError::InvalidPayloadLength { expected: 2, received: payload.len() });
        }
        if payload[0] != PEER_ENTRY_FORMAT {
            return Err(FrameError::UnsupportedVersion(payload[0]));
        }
        match payload[1] {
            0 => return Ok(None),
            1 => {}
            2 => return Err(FrameError::UnsupportedVersion(0)),
            _ => return Err(FrameError::InvalidStatus(payload[1])),
        }
        if payload.len() < PEER_HIT_FIXED_LEN {
            return Err(FrameError::InvalidPayloadLength { expected: PEER_HIT_FIXED_LEN, received: payload.len() });
        }
        let u64_at = |o: usize| u64::from_be_bytes(payload[o..o + 8].try_into().unwrap());
        let u16_at = |o: usize| u16::from_be_bytes(payload[o..o + 2].try_into().unwrap());
        let status = u16_at(2);
        let flags = payload[4];
        let age_ms = u64_at(5);
        let remaining_ms = u64_at(13);
        let swr = u64_at(21);
        let sie = u64_at(29);
        let date = u64_at(37);
        let initial_age_ms = u64_at(45);
        let ct_len = u16_at(53) as usize;
        let etag_len = u16_at(55) as usize;
        let lm_len = u16_at(57) as usize;
        let mut off = PEER_HIT_FIXED_LEN;
        let need = off + ct_len + etag_len + lm_len;
        if payload.len() < need {
            return Err(FrameError::InvalidPayloadLength { expected: need, received: payload.len() });
        }
        let take = |off: &mut usize, len: usize| -> Result<String, FrameError> {
            let s = std::str::from_utf8(&payload[*off..*off + len]).map_err(|_| FrameError::InvalidKeyUtf8)?.to_string();
            *off += len;
            Ok(s)
        };
        let content_type = take(&mut off, ct_len)?;
        let etag = take(&mut off, etag_len)?;
        let last_modified = take(&mut off, lm_len)?;
        Ok(Some(PeerEntry {
            status,
            content_type,
            etag: if flags & PEER_FLAG_ETAG != 0 { Some(etag) } else { None },
            last_modified: if flags & PEER_FLAG_LAST_MODIFIED != 0 { Some(last_modified) } else { None },
            must_revalidate: flags & PEER_FLAG_MUST_REVALIDATE != 0,
            stale_while_revalidate_ms: if flags & PEER_FLAG_SWR != 0 { Some(swr) } else { None },
            stale_if_error_ms: if flags & PEER_FLAG_SIE != 0 { Some(sie) } else { None },
            date_ms: if flags & PEER_FLAG_DATE != 0 { Some(date) } else { None },
            initial_age_ms,
            age_ms,
            remaining_ms,
            body: payload[off..].to_vec(),
        }))
    }
}

impl PhantomMeshFrame {
    /// PeerReadRequest: `[PEER_ENTRY_FORMAT][key bytes]`.
    pub fn new_peer_read_request(request_id: u64, sequence_number: u64, key: &str) -> Result<Self, FrameError> {
        let key_bytes = key.as_bytes();
        if key_bytes.len() > MAX_KEY_LENGTH_BYTES {
            return Err(FrameError::KeyTooLarge { length: key_bytes.len(), max: MAX_KEY_LENGTH_BYTES });
        }
        let mut payload = Vec::with_capacity(1 + key_bytes.len());
        payload.push(PEER_ENTRY_FORMAT);
        payload.extend_from_slice(key_bytes);
        Self::new(MessageType::PeerReadRequest, 0, request_id, sequence_number, payload)
    }

    /// Returns `(requested_format, key)`. The caller decides what to do about a format it does not speak.
    pub fn extract_peer_read_request(&self) -> Result<(u8, &str), FrameError> {
        if self.header.msg_type != MessageType::PeerReadRequest {
            return Err(FrameError::UnknownMessageType(self.header.msg_type.to_u8()));
        }
        if self.payload.is_empty() || self.payload.len() > 1 + MAX_KEY_LENGTH_BYTES {
            return Err(FrameError::InvalidPayloadLength { expected: 1 + MAX_KEY_LENGTH_BYTES, received: self.payload.len() });
        }
        let key = std::str::from_utf8(&self.payload[1..]).map_err(|_| FrameError::InvalidKeyUtf8)?;
        Ok((self.payload[0], key))
    }

    /// PeerReadResponse carrying a Miss.
    pub fn new_peer_read_miss(request_id: u64, sequence_number: u64) -> Result<Self, FrameError> {
        Self::new(MessageType::PeerReadResponse, 0, request_id, sequence_number, vec![PEER_ENTRY_FORMAT, PeerReadKind::Miss as u8])
    }

    /// PeerReadResponse for a request whose format this build does not speak.
    pub fn new_peer_read_unsupported(request_id: u64, sequence_number: u64) -> Result<Self, FrameError> {
        Self::new(MessageType::PeerReadResponse, 0, request_id, sequence_number, vec![PEER_ENTRY_FORMAT, PeerReadKind::Unsupported as u8])
    }

    /// PeerReadResponse carrying a hit.
    pub fn new_peer_read_hit(request_id: u64, sequence_number: u64, entry: &PeerEntry) -> Result<Self, FrameError> {
        Self::new(MessageType::PeerReadResponse, 0, request_id, sequence_number, entry.encode_hit()?)
    }

    pub fn extract_peer_read_response(&self) -> Result<Option<PeerEntry>, FrameError> {
        if self.header.msg_type != MessageType::PeerReadResponse {
            return Err(FrameError::UnknownMessageType(self.header.msg_type.to_u8()));
        }
        PeerEntry::decode_response(&self.payload)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_encode_decode_roundtrip() {
        let original = FrameHeader {
            magic: MAGIC,
            version: VERSION_1,
            msg_type: MessageType::GetRequest,
            flags: 0x0102,
            request_id: 0x1122334455667788,
            sequence_number: 0x8877665544332211,
            payload_length: 42,
        };

        let encoded = original.encode();
        assert_eq!(encoded.len(), HEADER_LEN);

        let decoded = FrameHeader::decode(&encoded).expect("Decode should succeed");
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_bad_magic_rejected() {
        let mut buf = [0u8; HEADER_LEN];
        buf[0..4].copy_from_slice(b"GET "); // Legacy ASCII prefix
        buf[4] = 1;
        buf[5] = 1;

        let err = FrameHeader::decode(&buf).unwrap_err();
        assert_eq!(err, FrameError::BadMagic([0x47, 0x45, 0x54, 0x20]));
    }

    #[test]
    fn test_unsupported_version_rejected() {
        let mut buf = [0u8; HEADER_LEN];
        buf[0..4].copy_from_slice(&MAGIC);
        buf[4] = 2; // Version 2
        buf[5] = 1;

        let err = FrameHeader::decode(&buf).unwrap_err();
        assert_eq!(err, FrameError::UnsupportedVersion(2));
    }

    #[test]
    fn test_unknown_message_type_rejected() {
        let mut buf = [0u8; HEADER_LEN];
        buf[0..4].copy_from_slice(&MAGIC);
        buf[4] = 1;
        buf[5] = 0x99; // Invalid type

        let err = FrameHeader::decode(&buf).unwrap_err();
        assert_eq!(err, FrameError::UnknownMessageType(0x99));
    }

    #[test]
    fn test_oversized_payload_rejected_in_header() {
        let mut buf = [0u8; HEADER_LEN];
        buf[0..4].copy_from_slice(&MAGIC);
        buf[4] = 1;
        buf[5] = 1;
        let oversized = MAX_FRAME_PAYLOAD_BYTES + 1;
        buf[24..28].copy_from_slice(&oversized.to_be_bytes());

        let err = FrameHeader::decode(&buf).unwrap_err();
        assert_eq!(
            err,
            FrameError::PayloadTooLarge {
                length: oversized,
                max: MAX_FRAME_PAYLOAD_BYTES
            }
        );
    }

    #[test]
    fn test_key_size_limit_enforced() {
        let huge_key = "k".repeat(MAX_KEY_LENGTH_BYTES + 1);
        let err = PhantomMeshFrame::new_get_request(1, 1, &huge_key).unwrap_err();
        assert_eq!(
            err,
            FrameError::KeyTooLarge {
                length: MAX_KEY_LENGTH_BYTES + 1,
                max: MAX_KEY_LENGTH_BYTES
            }
        );
    }

    #[tokio::test]
    async fn test_frame_read_write_roundtrip() {
        let frame = PhantomMeshFrame::new_get_request(100, 1, "test_cache_key").unwrap();

        let mut buffer = Vec::new();
        write_frame(&mut buffer, &frame).await.unwrap();

        let mut cursor = std::io::Cursor::new(buffer);
        let decoded = read_frame(&mut cursor).await.unwrap();

        assert_eq!(decoded.header.msg_type, MessageType::GetRequest);
        assert_eq!(decoded.header.request_id, 100);
        assert_eq!(decoded.header.sequence_number, 1);
        assert_eq!(decoded.extract_key().unwrap(), "test_cache_key");
    }

    #[test]
    fn test_invalidate_request_codec_roundtrip() {
        let frame = PhantomMeshFrame::new_invalidate_request(
            42,
            1,
            100,
            500,
            2,
            InvalidationScope::ExactKey,
            "tenant_a:/api/v1/resource:?id=9",
        )
        .unwrap();

        assert_eq!(frame.header.msg_type, MessageType::InvalidateRequest);
        assert_eq!(frame.header.request_id, 42);

        let payload = frame.extract_invalidate_request().unwrap();
        assert_eq!(payload.membership_epoch, 100);
        assert_eq!(payload.stream_seq, 500);
        assert_eq!(payload.originator_id, 2);
        assert_eq!(payload.scope, InvalidationScope::ExactKey);
        assert_eq!(payload.key, "tenant_a:/api/v1/resource:?id=9");
    }

    #[test]
    fn test_invalidate_ack_codec_roundtrip() {
        let frame = PhantomMeshFrame::new_invalidate_ack(
            42,
            2,
            100,
            500,
            3,
            InvalidationStatus::SuccessEvicted,
            2048,
        )
        .unwrap();

        assert_eq!(frame.header.msg_type, MessageType::InvalidateAck);
        let payload = frame.extract_invalidate_ack().unwrap();
        assert_eq!(payload.membership_epoch, 100);
        assert_eq!(payload.stream_seq, 500);
        assert_eq!(payload.responder_id, 3);
        assert_eq!(payload.status, InvalidationStatus::SuccessEvicted);
        assert_eq!(payload.reclaimed_bytes, 2048);
    }

    #[test]
    fn test_invalidate_request_oversized_key_rejected() {
        let huge_key = "x".repeat(MAX_KEY_LENGTH_BYTES + 1);
        let err = PhantomMeshFrame::new_invalidate_request(
            1,
            1,
            1,
            1,
            1,
            InvalidationScope::ExactKey,
            &huge_key,
        )
        .unwrap_err();

        assert_eq!(
            err,
            FrameError::KeyTooLarge {
                length: MAX_KEY_LENGTH_BYTES + 1,
                max: MAX_KEY_LENGTH_BYTES,
            }
        );
    }

    #[test]
    fn test_invalidate_request_truncated_payload_rejected() {
        // Less than 20 bytes payload
        let short_payload = vec![0u8; 15];
        let frame = PhantomMeshFrame::new(
            MessageType::InvalidateRequest,
            0,
            1,
            1,
            short_payload,
        )
        .unwrap();

        let err = frame.extract_invalidate_request().unwrap_err();
        assert_eq!(
            err,
            FrameError::TruncatedPayload {
                expected: MIN_INVALIDATE_REQUEST_PAYLOAD as u32,
                received: 15,
            }
        );
    }

    #[test]
    fn test_invalidate_request_invalid_scope_rejected() {
        let mut payload = vec![0u8; 20];
        payload[17] = 0x99; // Invalid scope
        let frame = PhantomMeshFrame::new(
            MessageType::InvalidateRequest,
            0,
            1,
            1,
            payload,
        )
        .unwrap();

        let err = frame.extract_invalidate_request().unwrap_err();
        assert_eq!(err, FrameError::InvalidScope(0x99));
    }

    #[test]
    fn test_invalidate_request_invalid_utf8_rejected() {
        let mut payload = vec![0u8; 24];
        payload[18] = 0;
        payload[19] = 4; // key_len = 4
        payload[20..24].copy_from_slice(&[0xff, 0xff, 0xff, 0xff]); // invalid UTF-8

        let frame = PhantomMeshFrame::new(
            MessageType::InvalidateRequest,
            0,
            1,
            1,
            payload,
        )
        .unwrap();

        let err = frame.extract_invalidate_request().unwrap_err();
        assert_eq!(err, FrameError::InvalidKeyUtf8);
    }

    #[test]
    fn test_invalidate_ack_invalid_status_rejected() {
        let mut payload = vec![0u8; 24];
        payload[17] = 0xAA; // invalid status

        let frame = PhantomMeshFrame::new(
            MessageType::InvalidateAck,
            0,
            1,
            1,
            payload,
        )
        .unwrap();

        let err = frame.extract_invalidate_ack().unwrap_err();
        assert_eq!(err, FrameError::InvalidStatus(0xAA));
    }

    #[test]
    fn test_invalidate_ack_invalid_length_rejected() {
        let payload = vec![0u8; 20]; // 20 bytes instead of 24
        let frame = PhantomMeshFrame::new(
            MessageType::InvalidateAck,
            0,
            1,
            1,
            payload,
        )
        .unwrap();

        let err = frame.extract_invalidate_ack().unwrap_err();
        assert_eq!(
            err,
            FrameError::InvalidPayloadLength {
                expected: INVALIDATE_ACK_PAYLOAD_LEN,
                received: 20,
            }
        );
    }
}
