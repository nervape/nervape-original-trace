use crate::error::TraceLockError;
use alloc::{vec, vec::Vec};
use ckb_std::{
    ckb_constants::Source,
    debug,
    high_level::{load_cell_data_hash, load_cell_lock_hash, load_cell_type_hash, QueryIter},
};

#[derive(Debug)]
pub struct UnpackedTraceArgs {
    pub lock_hash: [u8; 32], // looking for lock_hash as ownership approve

}

pub fn unpack_script_args(args: &[u8]) -> Result<UnpackedTraceArgs, TraceLockError> {
    Ok(UnpackedTraceArgs{
        lock_hash: args[0..32].try_into().map_err(|_| TraceLockError::InvalidArgs)?,
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