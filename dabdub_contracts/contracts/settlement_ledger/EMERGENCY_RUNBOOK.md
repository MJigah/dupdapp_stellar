# Emergency Runbook: settlement_ledger

## Overview

The `settlement_ledger` contract is the permanent on-chain audit trail of all fiat settlements. Records are append-only (immutable once written) and keyed by `payment_id`. This runbook covers:

1. Detecting and responding to erroneous settlement records
2. Reconciliation after a backend bug or data corruption
3. Disputes over settlement amounts (and how they're handled)

## Incident: Erroneous Settlement Record

### Symptoms

- A settlement record was written for a `payment_id` that was never actually released in escrow
- The amount, merchant, or fee in the settlement doesn't match the escrow payment
- A backend bug or compromised admin key wrote an incorrect record
- Records are immutable, so the error cannot be "fixed" in-place

### Assessment

**1. Identify the erroneous record**

```bash
# Query the erroneous settlement
soroban contract invoke \
  --id SETTLEMENT_LEDGER_CONTRACT_ID \
  -- get_settlement \
  --payment_id PAYMENT_ID
```

Record details:
- `merchant`: which merchant was affected
- `amount`, `fee`, `net`: claimed settlement amounts
- `fiat_ref`: bank reference used in the payment

**2. Verify against the escrow contract**

```bash
# Query the payment escrow to confirm the real state
soroban contract invoke \
  --id PAYMENT_ESCROW_CONTRACT_ID \
  -- get_payment \
  --payment_id PAYMENT_ID
```

Check:
- Does the payment exist?
- Is the status `Released`?
- Do the amounts match?
- Is the merchant correct?

### Recovery Options

Since settlement records are immutable, you have three options:

#### Option 1: Create a Correcting Settlement Record (Recommended)

Create a new, correcting settlement record to reconcile the error:

```bash
# Record the correction (assumes error was over-settlement)
soroban contract invoke \
  --id SETTLEMENT_LEDGER_CONTRACT_ID \
  --source ADMIN_KEYPAIR \
  -- record_settlement \
  --payment_id CORRECTION_PAYMENT_ID \
  --merchant MERCHANT_ADDRESS \
  --amount 0 \
  --fee CORRECTION_AMOUNT \
  --net -CORRECTION_AMOUNT \
  --timestamp $(date +%s) \
  --fiat_ref "CORRECTION: refund for erroneous settlement ID $ORIGINAL_PAYMENT_ID"
```

This approach:
- Leaves the original erroneous record visible (for audit trail)
- Adds a correcting entry to reconcile the ledger
- Allows downstream reconciliation (fiat systems) to offset the error

#### Option 2: Offline Dispute Resolution

If the error is small or the merchant disputes it:

1. **Document the error**: Record in your backend system that settlement `$PAYMENT_ID` is disputed
2. **Mark the merchant account**: Flag the account as under dispute/reconciliation
3. **Negotiate offline**: Work with the merchant and fiat processor to agree on the correction
4. **Record the settlement**: Once agreed, record the correction via Option 1

#### Option 3: Admin Key Rotation + Re-settlement (Last Resort)

If multiple erroneous records exist from a compromised admin key:

1. **Rotate the admin key** immediately (see `/admin_timelock/EMERGENCY_RUNBOOK.md`)
2. **Audit all recent settlements**: Query all settlements written in the last N hours/days
3. **Identify all errors**: Cross-reference with escrow contract to find discrepancies
4. **Create bulk correction records**: Write correcting entries for each identified error
5. **Notify all affected merchants**: Explain the error and the correction in their account

## Incident: Disputed Settlement Amount

### Symptoms

- A merchant claims the settlement amount is incorrect (e.g., "I should have received more")
- The settlement record is immutable, so it cannot be changed
- The escrow contract shows a different released amount
- Need to determine root cause (backend bug, merchant error, fee miscalculation)

### Response

**1. Verify the amounts**

Check the escrow payment:
```bash
soroban contract invoke \
  --id PAYMENT_ESCROW_CONTRACT_ID \
  -- get_payment \
  --payment_id PAYMENT_ID
```

Settlement ledger:
```bash
soroban contract invoke \
  --id SETTLEMENT_LEDGER_CONTRACT_ID \
  -- get_settlement \
  --payment_id PAYMENT_ID
```

**2. Trace the discrepancy**

Compare:
- `escrow.amount` vs `settlement.amount`: gross amount should match
- `escrow.released_amount` vs `settlement.net`: net received should match
- `settlement.fee`: compare against fee_calculator's calculated fee

**3. Determine root cause**

- **Backend bug**: Fee miscalculation, amount truncation, or lost digit
- **Escrow bug**: Payment released with wrong amount
- **Merchant error**: Merchant misread their account balance
- **Double-settlement**: Payment was settled twice

**4. Resolution**

- **Confirmed error**: Record a correcting settlement (Option 1 above)
- **Merchant misunderstanding**: Provide clear documentation of the breakdown (fee, net, exchange rate)
- **Ambiguous**: Mark as dispute and escalate to legal/compliance team

## Incident: Settlement Ledger Corruption (Multiple Errors)

If many recent settlement records appear erroneous:

### Immediate Steps

1. **Halt new settlements**: Pause the admin key from writing new records until root cause is found
2. **Audit backend logs**: Check for recent bugs, database corruption, or admin key misuse
3. **Query the ledger**: List all settlements from the suspect time window
4. **Cross-check escrow**: For each settlement, verify against the escrow contract

### Recovery

If a systemic issue is confirmed:

1. **Isolate the error window**: Identify the time range of corrupted records
2. **Create a correcting batch**: Write correcting entries for each error
3. **Notify affected merchants**: Explain the incident and the correction
4. **Root-cause analysis**: Update backend code to prevent recurrence
5. **Audit trail**: Document all corrections with timestamps and explanations

## Contract Interface Reference

### Privileged Functions

- `record_settlement(env: Env, caller: Address, payment_id: BytesN<32>, merchant: Address, amount: i128, fee: i128, net: i128, timestamp: u64, fiat_ref: String)`
  - Requires: admin key
  - Effect: writes immutable settlement record (panics if payment_id already exists)

- `set_payment_escrow_contract(env: Env, caller: Address, payment_escrow_contract: Address)`
  - Requires: admin key
  - Effect: configures the PaymentEscrow contract address for cross-validation

### Query Functions

- `get_settlement(env: Env, payment_id: BytesN<32>) -> SettlementRecord`
  - Returns the settlement record or panics if not found

- `list_settlements(env: Env, merchant: Address, page: u32) -> Vec<SettlementRecord>`
  - Returns paginated list of settlements for a merchant

- `settlement_count(env: Env, merchant: Address) -> u32`
  - Returns total number of settlements for a merchant

## Prevention

- **Always validate against escrow**: Configure the PaymentEscrow contract address so record_settlement can cross-check
- **Immutable by design**: Embrace immutability; don't attempt to patch bad records—instead, create correcting entries
- **Clear fiat_ref**: Always include a descriptive `fiat_ref` in each record (makes offline reconciliation easier)
- **Admin key security**: Keep the admin key secure (hardware wallet, multisig signing)
- **Audit logging**: Enable on-chain event logging so all settlements are queryable and auditable
- **Reconciliation**: Regularly (daily or weekly) reconcile the settlement ledger against your fiat processor's records

## Escalation

If unable to resolve a disputed settlement:

1. Escalate to the compliance/legal team
2. Prepare a summary of the error with evidence from both escrow and settlement ledger
3. If needed, propose an emergency off-chain refund (bypassing the settlement ledger)
4. Document the incident for regulatory audits
