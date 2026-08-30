# Emergency Runbook: multisig_admin

## Overview

The `multisig_admin` contract implements M-of-N signature-based governance for critical platform operations. It gates all privileged actions across the system (admin role transitions, parameter changes, emergency procedures). This runbook covers:

1. Recovery when a signer key is lost or compromised
2. Restoring consensus when M-of-N quorum cannot be reached
3. Emergency rotation of signing authorities

## Incident: Lost Signer Key

### Symptoms
- A multisig signer has lost access to their key (key deleted, mnemonic lost, hardware wallet failure)
- Transactions requiring M signatures now lack sufficient signers
- System is at risk if one more signer is lost (no longer achieves M-of-N quorum)

### Assessment

Check the current state:

```bash
# Query active signers
soroban contract invoke \
  --id MULTISIG_ADMIN_CONTRACT_ID \
  -- get_admin_signers
```

Current configuration: **M-of-N** (e.g., 2-of-3)
- If a signer is lost and M-of-N quorum is still achievable → proceed with key rotation
- If quorum is no longer achievable → escalate to emergency multi-signer recovery

### Recovery: Standard Signer Rotation

**Note:** The current `multisig_admin` contract does not provide a `remove_admin` or `replace_admin` function. Recovery requires one of:

#### Option A: Multisig Governance Vote (Recommended)

If governance operates via this contract:

1. **Proposal Phase**: Remaining M signers propose a signer rotation:
   ```bash
   soroban contract invoke \
     --id MULTISIG_ADMIN_CONTRACT_ID \
     --source SIGNER_1 \
     -- propose_signer_rotation \
     --old_signer LOST_SIGNER_ADDRESS \
     --new_signer NEW_SIGNER_ADDRESS
   ```

2. **Voting Phase**: Other M-1 signers approve the proposal

3. **Execution**: Execute the approved rotation

#### Option B: Emergency Multi-Signature Update (Requires All Available Signers)

If a `set_signers` function exists that accepts a new M-of-N configuration:

1. Gather signatures from all currently-accessible signers
2. Propose new configuration with replacement signer
3. Execute update

#### Option C: Contract Upgrade (Last Resort)

If the multisig contract cannot be modified via governance:

1. Deploy a new multisig contract with updated signers
2. Manually transfer admin authority to the new contract
3. Sunset the old contract
4. Update all dependent contracts to reference the new multisig

### Prevention of Future Lost Keys

- Require signers to use hardware wallets (Ledger, Trezor, YubiHSM)
- Maintain geographically distributed signing authority
- Require annual key rotation and security audits
- Implement key backup and recovery in secure escrow (without compromising security)

## Incident: Compromised Signer Key

### Symptoms
- A signer key may have been exposed in code, logs, or a security breach
- The attacker could sign transactions on behalf of the compromised signer
- If the attacker plus M-1 other signers coordinate, they can authorize any action

### Immediate Response

1. **Identify the compromised signer** address
2. **Initiate emergency key rotation** using the process above (Option A or B)
3. **Audit recent transactions**: Check contract event logs for all recent multisig approvals; verify they were authorized
4. **Revoke old key**: Once rotation is complete, the old key can no longer authorize new transactions

### Investigation

Query multisig events to identify suspicious activity:

```bash
# Example: list all multisig approvals from the last 24 hours
# (implementation depends on event indexing)
```

## Incident: Unable to Reach M-of-N Quorum

### Symptoms
- Multiple signer keys are lost or unavailable
- Current quorum is less than M signers
- No governance proposal can be approved
- System is stuck (no privileged actions possible)

### Escalation Path

This is a critical failure state. Recovery requires one of:

1. **Wait for Signer Recovery**: If the lost signers can eventually be recovered (e.g., hardware wallet found), restore keys and proceed with governance
2. **Governance Override**: If a higher-level governance mechanism exists (e.g., Stellar Foundation, community vote), invoke emergency powers to reinitialize multisig
3. **Contract Pause**: Pause the system and pause all dependent contracts to prevent further damage while recovery proceeds
4. **Redeploy**: As a last resort, redeploy all contracts with a new multisig configuration and manual state migration

## Contract Interface Reference

### Privileged Functions

- `propose_transaction(env: Env, caller: Address, tx_id: BytesN<32>, action: Bytes) -> u32`
  - Requires: one of the M signers
  - Returns: proposal ID

- `approve_transaction(env: Env, caller: Address, tx_id: BytesN<32>) -> bool`
  - Requires: one of the M signers
  - Returns: true if transaction now has M approvals

- `execute_transaction(env: Env, caller: Address, tx_id: BytesN<32>)`
  - Requires: transaction has M approvals
  - Effect: executes the approved transaction

### Query Functions

- `get_admin_signers(env: Env) -> Vec<Address>`
  - Returns the list of M authorized signers

- `get_quorum(env: Env) -> u32`
  - Returns M (number of signatures required)

- `is_approved(env: Env, tx_id: BytesN<32>) -> bool`
  - Returns whether a transaction has reached M approvals

## Prevention

- Use hardware wallets for all signers
- Distribute signers geographically and organizationally
- Conduct regular key rotation (quarterly or annually)
- Maintain detailed audit logs of all multisig actions
- Require cold storage backup for signer keys (in secure escrow)
- Implement timelock governance (e.g., scheduled changes with 24-48 hour review period)

## Escalation

If the contract itself is broken or cannot process governance:

1. Contact Stellar network governance
2. Propose a network upgrade or halt if necessary
3. Prepare a manual state-migration transaction to recover critical functionality
