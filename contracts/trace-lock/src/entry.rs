use alloc::vec::Vec;
use ckbfs_types::{CKBFSData, CKBFSDataNative};
use trace_lock::CKBFS_CODE_HASH;
use crate::{error::TraceLockError, utils::{parse_operation, unpack_script_args, Operation}};
use ckb_std::{ckb_constants::Source, ckb_types::prelude::{ShouldBeOk, Unpack}, high_level::{load_cell_data, load_cell_lock, load_cell_lock_hash, load_cell_type, load_cell_type_hash, load_script, load_script_hash, load_witness, QueryIter}};
use molecule::prelude::Entity;

fn owner_lock_provided(owner_lock_hash: [u8; 32]) -> bool {
    // check if there is any owner lock provided
    return QueryIter::new(load_cell_lock_hash, Source::Input)
    .any(|lock_hash| lock_hash[..] == owner_lock_hash[..]);
}

pub fn main() -> Result<(), TraceLockError> {

    let script_hash = load_script_hash()?;
    
    let trace_lock_in_input = QueryIter::new(load_cell_lock_hash, Source::GroupInput)
    .enumerate()
    .filter(|(_, lock_hash)| lock_hash[..] == script_hash[..])
    .map(|(index, _)| index)
    .collect::<Vec<usize>>();

    let script = load_script()?;
    let args: Vec<u8> = script.args().unpack();

    let unpacked_args = unpack_script_args(&args)?;


    // check if there is any owner lock provided
    if !QueryIter::new(load_cell_lock_hash, Source::Input)
            .any(|lock_hash| lock_hash[..] == unpacked_args.lock_hash[..]) {
                return Err(TraceLockError::NoOwnerLockProvided);
    }

    for in_index in trace_lock_in_input { // iterate all trace lock cell in input
        let type_script = load_cell_type(in_index, Source::GroupInput)?;
        let type_hash = load_cell_type_hash(in_index, Source::GroupInput)?.unwrap_or_default();
        let type_script_code_hash: [u8; 32] = type_script.unwrap_or_default().code_hash().unpack();
        if type_script_code_hash[..] == CKBFS_CODE_HASH {

            // 1. find same type in output
            let output_index = QueryIter::new(load_cell_type_hash, Source::Output)
            .position(|cell_type_hash| cell_type_hash.unwrap_or_default()[..] == type_hash[..]).should_be_ok(); // CKBFS can not be destroyed, so this should always be ok
            
            let output_lock = load_cell_lock(output_index, Source::Output)?;

            // try unpack data
            let output_data = load_cell_data(output_index, Source::Output)?;
            let unpacked_data = CKBFSData::from_compatible_slice(&output_data).map_err(|_| TraceLockError::IncompatibleCKBFSData)?;
            let unpacked_output_lock_code_hash: [u8; 32] = output_lock.code_hash().unpack();

            if unpacked_output_lock_code_hash[..] != script_hash[..] {
                // should be a REALEASE OPERATION
                if !unpacked_args.feature_flags.enable_release {
                    return Err(TraceLockError::ForbidOperationRelease);
                }
                let native_data: CKBFSDataNative = unpacked_data.into();

                // load raw data from witnesses
                let mut raw_data = Vec::new();
                for witnesse_index in native_data.indexes {
                    let witness = load_witness(witnesse_index as usize, Source::Input)?;
                    raw_data.extend_from_slice(&witness);
                }
                let mut maybe_extension_operation = false;

                // content is from 7th byte to the end
                let content = &raw_data[7..];
                // read line by line, and check if there is any operation log
                for line in content.split(|c| *c == b'\n') {
                    let operation = parse_operation(line, true)?;
                    match operation {
                        Operation::Release(former_owner) => {
                            if former_owner == unpacked_args.lock_hash {
                                // check if there is any owner lock provided
                                if !owner_lock_provided(former_owner) {
                                    return Err(TraceLockError::NoOwnerLockProvided);
                                }
                                return Ok(());
                            }
                        },
                        Operation::GiveName(_) | Operation::Transfer(_) | Operation::Mint(_) => {}, // do not update extension flag
                        _ => {
                            // do nothing
                            maybe_extension_operation = true;
                        }
                    }
                }
                if maybe_extension_operation {
                    return Ok(());
                }
                // return error if no operation log found
                return Err(TraceLockError::NoOperationLogMatch);

            } else { // still using trace lock, should be a TRANSFER OPERATION, or some other extension operations
                let output_args: Vec<u8> = output_lock.args().unpack();
                let output_lock_args = unpack_script_args(&output_args).map_err(|_| TraceLockError::InvalidScriptHash)?;

                if !unpacked_args.feature_flags.enable_transfer {
                    return Err(TraceLockError::ForbidOperationTransfer);
                }

                let native_data: CKBFSDataNative = unpacked_data.into();
                // load raw data from witnesses
                let mut raw_data = Vec::new();
                for witnesse_index in native_data.indexes {
                    let witness = load_witness(witnesse_index as usize, Source::Input)?;
                    raw_data.extend_from_slice(&witness);
                }

                let mut maybe_extension_operation = false;

                // content is from 7th byte to the end
                let content = &raw_data[7..];
                // read line by line, and check if there is any operation log
                for line in content.split(|c| *c == b'\n') {

                    if line.is_empty(){ // skip empty line and end of line
                        continue;
                    }

                    let operation = parse_operation(line, true)?;
                    if !owner_lock_provided(unpacked_args.lock_hash) {
                        return Err(TraceLockError::NoOwnerLockProvided);
                    }
                    match operation {
                        Operation::Transfer((from, to)) => {
                            if (from, to) == (unpacked_args.lock_hash, output_lock_args.lock_hash) {
                                
                                return Ok(());
                            }
                        },
                        Operation::Release(_) | Operation::Mint(_) => {}, // do not update extension flag
                        _ => {
                            // do nothing
                            maybe_extension_operation = true;
                        }
                    }
                }
                if maybe_extension_operation {
                    return Ok(());
                }
                // return error if no operation log found
                return Err(TraceLockError::NoOperationLogMatch);
            }
                
        } else {
            // do normal proxy lock check
            // check if there is any lock hash matches
            if QueryIter::new(load_cell_lock_hash, Source::Input)
            .any(|lock_hash| lock_hash[..] == unpacked_args.lock_hash[..]) {
                return Ok(());
            }
            return Err(TraceLockError::NoOwnerLockProvided);
        }
    }

    Ok(())
}