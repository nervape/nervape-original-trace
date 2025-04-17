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
    InvalidTypeId = 102,
    InvalidFieldUpdate = 103,    // only backlinks is able to append, others are immutable
    DuplicatedOutputs = 107,     // there can not be two same ckbfs cell in output
    InvalidAppend = 108,         // append data updates not meet
    InvalidTransfer = 109,       // transfer operation data updates not meet
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