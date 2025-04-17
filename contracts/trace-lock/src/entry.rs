use alloc::vec::Vec;
use crate::{error::TraceLockError, utils::unpack_script_args};
use ckb_std::{ckb_constants::Source, ckb_types::prelude::Unpack, high_level::{load_cell_lock_hash, load_script, load_script_hash, QueryIter}};

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

    Ok(())
}