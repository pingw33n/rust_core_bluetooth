use std::fmt;

use crate::platform::*;

pub(in crate) fn result<T, F: FnOnce() -> T>(error: Option<NSError>, ok: F) -> Result<T, Error> {
    if let Some(error) = error.map(Error::from_ns_error) {
        Err(error)
    } else {
        Ok(ok())
    }
}

#[derive(Clone, Debug)]
pub struct Error {
    kind: ErrorKind,
    description: String,
}

impl Error {
    pub(in crate) fn from_ns_error(err: NSError) -> Self {
        let domain = err.domain();
        let code = err.code();
        let kind = if domain.is_equal_to_string(unsafe { CBErrorDomain }) {
            ErrorKind::from_code(code)
        } else if domain.is_equal_to_string(unsafe { CBATTErrorDomain }) {
            ErrorKind::Att(AttErrorKind::from_code(code))
        } else {
            ErrorKind::Other
        };
        let description = err.description().as_str().to_owned();
        Self {
            kind,
            description,
        }
    }

    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.description)
    }
}

impl std::error::Error for Error {}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Error from unknown domain.
    Other,

    /// An unknown error occurred.
    Unknown,

    /// The specified parameters are invalid.
    InvalidParameters,

    /// The specified attribute handle is invalid.
    InvalidHandle,

    /// The device isn’t currently connected.
    NotConnected,

    /// The device has run out of space to complete the intended operation.
    OutOfSpace,

    /// The error represents a canceled operation.
    OperationCancelled,

    /// The connection timed out.
    ConnectionTimeout,

    /// The peripheral disconnected.
    PeripheralDisconnected,

    /// The specified UUID isn’t permitted.
    UuidNotAllowed,

    /// The peripheral is already advertising.
    AlreadyAdvertising,

    /// The connection failed.
    ConnectionFailed,

    /// The device already has the maximum number of connections.
    ConnectionLimitReached,

    /// The operation isn’t supported.
    OperationNotSupported,

    /// The device is unknown.
    UnknownDevice,

    Att(AttErrorKind),
}

impl ErrorKind {
    fn from_code(code: isize) -> Self {
        use ErrorKind::*;
        match code {
            1 => InvalidParameters,
            2 => InvalidHandle,
            3 => NotConnected,
            4 => OutOfSpace,
            5 => OperationCancelled,
            6 => ConnectionTimeout,
            7 => PeripheralDisconnected,
            8 => UuidNotAllowed,
            9 => AlreadyAdvertising,
            10 => ConnectionFailed,
            11 => ConnectionLimitReached,
            12 => UnknownDevice,
            13 => OperationNotSupported,
            _ => Unknown,
        }
    }
}

/// The possible errors returned by a GATT server (a remote peripheral) during
/// Bluetooth low energy ATT transactions.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum AttErrorKind {
    /// ATT error that didn't map to any of the existing variants.
    Other,

    /// The ATT command or request successfully completed.
    Success,

    /// The attribute handle is invalid on this peripheral.
    InvalidHandle,

    /// The permissions prohibit reading the attribute’s value.
    ReadNotPermitted,

    /// The permissions prohibit writing the attribute’s value.
    WriteNotPermitted,

    /// The attribute Protocol Data Unit (PDU) is invalid.
    InvalidPdu,

    /// Reading or writing the attribute’s value failed for lack of authentication.
    InsufficientAuthentication,

    /// The attribute server doesn’t support the request received from the client.
    RequestNotSupported,

    /// The specified offset value was past the end of the attribute’s value.
    InvalidOffset,

    /// Reading or writing the attribute’s value failed for lack of authorization.
    InsufficientAuthorization,

    /// The prepare queue is full, as a result of there being too many write requests in the queue.
    PrepareQueueFull,

    /// The attribute wasn’t found within the specified attribute handle range.
    AttributeNotFound,

    /// The ATT read blob request can’t read or write the attribute.
    AttributeNotLong,

    /// The encryption key size used for encrypting this link is insufficient.
    InsufficientEncryptionKeySize,

    /// The length of the attribute’s value is invalid for the intended operation.
    InvalidAttributeValueLength,

    /// The ATT request encountered an unlikely error and wasn’t completed.
    UnlikelyError,

    /// Reading or writing the attribute’s value failed for lack of encryption.
    InsufficientEncryption,

    /// The attribute type isn’t a supported grouping attribute as defined by a higher-layer specification.
    UnsupportedGroupType,

    /// Resources are insufficient to complete the ATT request.
    InsufficientResources,
}

impl AttErrorKind {
    fn from_code(code: isize) -> Self {
        use AttErrorKind::*;
        match code {
            0 => Success,
            1 => InvalidHandle,
            2 => ReadNotPermitted,
            3 => WriteNotPermitted,
            4 => InvalidPdu,
            5 => InsufficientAuthentication,
            6 => RequestNotSupported,
            7 => InvalidOffset,
            8 => InsufficientAuthorization,
            9 => PrepareQueueFull,
            10 => AttributeNotFound,
            11 => AttributeNotLong,
            12 => InsufficientEncryptionKeySize,
            13 => InvalidAttributeValueLength,
            14 => UnlikelyError,
            15 => InsufficientEncryption,
            16 => UnsupportedGroupType,
            17 => InsufficientResources,
            _ => Other,
        }
    }
}