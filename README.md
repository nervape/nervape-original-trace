# nervape-original-trace

A lock to use together with CKBFS that allows **OPERATION LOG FORMAT**

## Basic workflow

This lock will check if there is a new `APPEND` operation to the refer CKBFS cell, then

1. Verify if this operation is a `OPERATION LOG FORMAT`; If not, it won't unlock
2. Verify if the operation is a `TRANSFER` or `RELEASE` op;
2.1. If operation is a `RELEASE` op, the lock will be free to unlock
2.2. If operation is a `TRANSFER` op, the lock arg must be reset to target the transfer desitination `lock_hash` 
3. If the type script is not CKBFS contract, then it will act as a normal proxy lock

*This project was bootstrapped with [ckb-script-templates].*

[ckb-script-templates]: https://github.com/cryptape/ckb-script-templates
