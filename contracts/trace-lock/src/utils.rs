use crate::error::TraceLockError;
use alloc::{vec, vec::Vec};
use ckb_std::{
    ckb_constants::Source,
    debug,
    high_level::{load_cell_data_hash, load_cell_lock_hash, load_cell_type_hash, QueryIter},
};
use alloc::string::{String, ToString};
use hex::decode_to_slice;

#[derive(Debug)]
pub struct FeatureFlags {
    pub enable_release: bool,
    pub enable_transfer: bool,
}

impl FeatureFlags {
    pub fn unpack(flag_bits: u8) -> FeatureFlags {
        FeatureFlags {
            enable_release: (flag_bits & 0b00000001) != 0,
            enable_transfer: (flag_bits & 0b00000010) != 0
        }
    }
}

#[derive(Debug)]
pub struct UnpackedTraceArgs {
    pub feature_flags: FeatureFlags,
    pub lock_hash: [u8; 32], // looking for lock_hash as ownership approve

}

pub fn unpack_script_args(args: &[u8]) -> Result<UnpackedTraceArgs, TraceLockError> {
    Ok(UnpackedTraceArgs{
        feature_flags: FeatureFlags::unpack(args[0]),
        lock_hash: args[1..33].try_into().map_err(|_| TraceLockError::InvalidArgs)?,
    })
}


pub fn check_input_output_contain_same_cell(
    input_index: usize,
    source: Source,
    check_data: bool,
    check_lock: bool,
) -> Result<Vec<usize>, TraceLockError> {
    debug!("input_index: {input_index}, source: {:?}", source);
    let input_type_hash = load_cell_type_hash(input_index, source)?;

    let data_position = if check_data {
        let data_hash = load_cell_data_hash(input_index, source)?;
        QueryIter::new(load_cell_data_hash, Source::Output)
            .enumerate()
            .filter(|(_, x)| x == &data_hash)
            .map(|(position, _)| position)
            .collect::<Vec<usize>>()
    } else {
        vec![]
    };

    let lock_position = if check_lock {
        let lock_hash = load_cell_lock_hash(input_index, source)?;
        QueryIter::new(load_cell_lock_hash, Source::Output)
            .enumerate()
            .filter(|(_, x)| x == &lock_hash)
            .map(|(position, _)| position)
            .collect::<Vec<usize>>()
    } else {
        vec![]
    };

    let found_same_cell = QueryIter::new(load_cell_type_hash, Source::Output)
        .enumerate()
        .filter(|(_, x)| x == &input_type_hash)
        .filter(|(tp, _)| {
            let data_matches = !check_data || data_position.contains(&tp);
            let lock_matches = !check_lock || lock_position.contains(&tp);
            debug!("index: {tp}, data_matches: {data_matches}, lock_matches: {lock_matches}");
            data_matches && lock_matches
        })
        .map(|(same_index, _)| same_index)
        .collect::<Vec<usize>>();

    // Now check if all positions (type, lock, data) are Some and are equal

    // Return None if any of the checks failed or if positions are not equal
    Ok(found_same_cell)
}


pub enum Operation {
    Mint([u8; 32]), // MINT, TO:NEW_OWNER
    Release([u8; 32]), // RELEASE, FROM:FORMER_OWNER
    Transfer(([u8; 32], [u8; 32])), // TRANSFER, FROM:FORMER_OWNER, TO:NEW_OWNER
    GiveName(String), // GIVE_NAME, NEW_NAME:NAME
    ExtensionOP, // for unknown operation
}

// decode address from hex string
pub fn decode_address(address: &str) -> Result<[u8; 32], TraceLockError> {
    let address = address.trim_matches(' ');
    // trim '0x' prefix
    let address = address.trim_start_matches("0x");
    let mut address_bytes = [0u8; 32];
    decode_to_slice(address, &mut address_bytes).map_err(|_| TraceLockError::InvalidScriptHash)?;
    Ok(address_bytes)
}

// operation details is a string, and may contains extra informations, like comment, or other operation extensions that splited by ','
// for example: "TRANSFER, FROM:FORMER_OWNER, TO:NEW_OWNER, COMMENT:HAHA // this is a comment"
// this util function is for filter out comment, and stick with the format requirement
// with the `detail_part` count provided
pub fn filter_comment(details: &str, detail_part: Option<usize>) -> Result<Vec<&str>, TraceLockError> {
    let parts = details.split(',').collect::<Vec<&str>>();
    let mut result = Vec::new();
    let mut index = 0;
    for part in parts {
        let part = part.trim();
        if part.starts_with("COMMENT:") {
            continue;
        }
        result.push(part);
        index += 1;
        if let Some(detail_part) = detail_part {
            if index == detail_part {
                break; // stop at the detail_part
            }
        }
    }
    Ok(result)
}

pub fn parse_operation(operation_content: &[u8], enable_unknown_op: bool) -> Result<Operation, TraceLockError> {
    // the content is a UTF-8 string, in format:
    // OP, DETAIL
    // for example, a release operation: "RELEASE, FROM:FORMER_OWNER"
    // a transfer operation: "TRANSFER, FROM:FORMER_OWNER, TO:NEW_OWNER"
    let operation_str = String::from_utf8_lossy(operation_content);
    let parts = operation_str.split(',').collect::<Vec<&str>>();
    if parts.len() < 2 { // at least 2 parts
        return Err(TraceLockError::InvalidOperationLog);
    }

    // trim head and tail spaces
    let op = parts[0].trim();

    match op {
        "MINT" => {
            let detail = parts[1].trim();
            let details = filter_comment(detail, Some(1))?;
            if details.len() == 1 && details[0].starts_with("TO:") {
                let new_owner = details[0].split(':').nth(1).unwrap_or_default();
                let new_owner = decode_address(new_owner).map_err(|_| TraceLockError::InvalidMint)?;
                Ok(Operation::Mint(new_owner))
            } else {
                Err(TraceLockError::InvalidMint)
            }
        },
        "RELEASE" => {
            let detail = parts[1].trim();
            // check if the detail is in format: FROM:FORMER_OWNER
            let details = filter_comment(detail, Some(1))?;
            if details.len() == 1 && details[0].starts_with("FROM:") {
                let former_owner = details[0].split(':').nth(1).unwrap_or_default();
                let former_owner = decode_address(former_owner).map_err(|_| TraceLockError::InvalidRelease)?;
                Ok(Operation::Release(former_owner))
            } else {
                Err(TraceLockError::InvalidRelease)
            }
        },
        "GIVE_NAME" => {
            let detail = parts[1].trim();
            let details = filter_comment(detail, None)?;
            if details.len() == 1 {
                Ok(Operation::GiveName(details[0].to_string()))
            } else {
                Err(TraceLockError::InvalidOperationLog)
            }
        },
        "TRANSFER" => {
            let detail = parts[1].trim();
            let details = filter_comment(detail, Some(2))?;
            if details.len() == 2 && details[0].starts_with("FROM:") && details[1].starts_with("TO:") {
                let former_owner = details[0].split(':').nth(1).unwrap_or_default();
                let new_owner = details[1].split(':').nth(1).unwrap_or_default();
                let former_owner = decode_address(former_owner).map_err(|_| TraceLockError::InvalidTransfer)?;
                let new_owner = decode_address(new_owner).map_err(|_| TraceLockError::InvalidTransfer)?;
                Ok(Operation::Transfer((former_owner, new_owner)))
            } else {
                Err(TraceLockError::InvalidTransfer)
            }
        },

        _ => {
            if enable_unknown_op {
                Ok(Operation::ExtensionOP)
            } else {
                Err(TraceLockError::InvalidOperationLog)
            }
        }
    }
    
}