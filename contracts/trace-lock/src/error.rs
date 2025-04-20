use ckb_std::error::SysError;

/// Error
#[repr(i8)]
pub enum TraceLockError {
    IndexOutOfBound = -1,
    ItemMissing = -2,
    LengthNotEnough = -3,
    Encoding = -4,
    Unknown = -100,
    InvalidArgs = 101,
    NoOwnerLockProvided = 102,
    IncompatibleCKBFSData = 103,
    ForbidOperationRelease = 104,
    ForbidOperationTransfer = 105,       // transfer operation data updates not meet
    InvalidOperationLog = 106,
    NoOperationLogMatch = 107,
    InvalidRelease = 108,
    InvalidTransfer = 109,       // transfer operation data updates not meet
    InvalidScriptHash = 110, // invalid lock script hash string in trace log
    InvalidMint = 111,
    
}

impl From<SysError> for TraceLockError {
    fn from(err: SysError) -> Self {
        use SysError::*;
        match err {
            IndexOutOfBound => Self::IndexOutOfBound,
            ItemMissing => Self::ItemMissing,
            LengthNotEnough(_) => Self::LengthNotEnough,
            Encoding => Self::Encoding,
            _ => Self::Unknown,
        }
    }
}