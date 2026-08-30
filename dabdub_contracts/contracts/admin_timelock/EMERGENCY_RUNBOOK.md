# Emergency Runbook: admin_timelock

## Overview

The `admin_timelock` contract gates all privileged parameter changes across the Stellar platform. It implements a time-delayed execution model: configuration changes are proposed, stored, and executed only after a mandatory delay (`ledgers_to_lock`). This runbook covers incident response when:

1. A malicious or erroneous parameter change is scheduled but not yet executed
2. An admin key is compromised and used to schedule bad changes
3. A bad scheduled change must be cancelled before `execute_after`

## Incident: Malicious/Erroneous Scheduled Change

### Symptoms
- A scheduled change has been proposed that, if executed, would:
  - Lock out all users (e.g., setting an invalid contract parameter)
  - Drain funds (e.g., changing fee tiers to 100%)
  - Break critical functionality (e.g., disabling a required dependency)

### Response Steps

**1. Verify the pending change**

```bash
# Query the contract for pending changes (read-only)
soroban contract invoke \
  --id ADMIN_TIMELOCK_CONTRACT_ID \
  -- get_pending_changes
```

Examine the pending change details:
- Target parameter key
- Proposed value
- Execution timestamp (ledger sequence)
- Time remaining until `execute_after`

**2. Assess urgency**

- If `execute_after` is more than 24 hours away: proceed with deliberate cancellation
- If `execute_after` is within 1-4 hours: fast-track emergency council/governance decision
- If `execute_after` is within 1 hour: invoke emergency cancellation immediately (see step 3)

**3. Initiate cancellation**

The `cancel_change` function removes the scheduled change from pending execution. Requires the admin key.

```bash
soroban contract invoke \
  --id ADMIN_TIMELOCK_CONTRACT_ID \
  --source ADMIN_KEYPAIR \
  -- cancel_change \
  --change_id CHANGE_ID_FROM_STEP_1
```

**4. Verify cancellation**

Re-query to confirm the change is no longer scheduled:

```bash
soroban contract invoke \
  --id ADMIN_TIMELOCK_CONTRACT_ID \
  -- get_pending_changes
```

**5. Communicate resolution**

- Notify stakeholders that the bad change was cancelled
- Document the incident cause (e.g., "Admin key compromised" or "Backend bug")
- If admin key compromise is suspected, proceed with emergency admin rotation

## Incident: Compromised Admin Key

If a scheduled change was made with a compromised admin key:

### Recovery Steps

1. **Immediately cancel all pending changes** using `cancel_change` with the emergency admin key
2. **Rotate admin key**:
   - Use the current (uncompromised) admin key to call `set_admin(new_admin_address)`
   - Revoke access for the compromised key
3. **Audit contract interaction logs** to identify any other changes made with the compromised key
4. **Verify no changes are pending** before restoring normal operations

## Contract Interface Reference

### Privileged Functions

- `schedule_change(env: Env, caller: Address, change_id: BytesN<32>, value: Bytes) -> u32`
  - Requires: admin key
  - Returns: execution ledger

- `cancel_change(env: Env, caller: Address, change_id: BytesN<32>)`
  - Requires: admin key
  - Effect: removes pending change; callable until `execute_after`

- `execute_change(env: Env, caller: Address, change_id: BytesN<32>)`
  - Requires: admin key, and current ledger >= `execute_after`
  - Effect: applies the scheduled change

- `set_admin(env: Env, caller: Address, new_admin: Address)`
  - Requires: current admin key
  - Effect: transfers admin authority to new_admin

### Query Functions

- `get_pending_changes(env: Env) -> Vec<PendingChange>`
  - Returns all scheduled changes awaiting execution
  
- `get_admin(env: Env) -> Address`
  - Returns current admin address

## Prevention

- Keep admin key in secure hardware wallet (never in environments accessible to developers)
- Require at least 2 signers for admin operations (if feasible via governance)
- Monitor all scheduled changes via event logs; set up alerts for `schedule_change` events
- Use short `ledgers_to_lock` values (1-2 hours) to limit attack window
- Implement a governance council or DAO vote before executing critical changes

## Escalation

If unable to cancel a malicious change:

1. Contact the Stellar network governance team
2. Initiate an emergency halt procedure (if implemented)
3. Prepare a transaction proposal to overwrite the bad state (if rollback is possible)
