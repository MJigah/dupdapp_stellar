#![no_std]

mod test;

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum Role {
    ReadOnly,
    ComplianceAdmin,
    OperationsAdmin,
    SuperAdmin,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Role(Address),
    SuperAdmins,  // Track active super admins for recovery and rotation
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct RoleGrantedEvent {
    pub account: Address,
    pub role: Role,
    pub granted_by: Address,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct RoleRevokedEvent {
    pub account: Address,
    pub revoked_by: Address,
}

#[contract]
pub struct RbacAccessContract;

#[contractimpl]
impl RbacAccessContract {
    pub fn __constructor(env: Env, super_admin: Address) {
        // Issue #1026: Store roles in persistent() instead of instance()
        env.storage()
            .persistent()
            .set(&DataKey::Role(super_admin.clone()), &Role::SuperAdmin);
        // Track SuperAdmins for recovery and rotation
        let mut admins: soroban_sdk::Vec<Address> = soroban_sdk::Vec::new(&env);
        admins.push_back(super_admin);
        env.storage()
            .persistent()
            .set(&DataKey::SuperAdmins, &admins);
    }

    pub fn grant_role(env: Env, caller: Address, account: Address, role: Role) {
        caller.require_auth();
        Self::require_role(&env, &caller, Role::SuperAdmin);

        // Issue #1026: Store roles in persistent() instead of instance()
        env.storage()
            .persistent()
            .set(&DataKey::Role(account.clone()), &role);

        // If granting SuperAdmin, add to tracked admins
        if role == Role::SuperAdmin {
            if let Some(mut admins) = env.storage().persistent().get::<DataKey, soroban_sdk::Vec<Address>>(&DataKey::SuperAdmins) {
                if !admins.iter().any(|a| a == account) {
                    admins.push_back(account.clone());
                    env.storage().persistent().set(&DataKey::SuperAdmins, &admins);
                }
            }
        }

        env.events().publish(
            ("RBAC", "role_granted"),
            RoleGrantedEvent {
                account,
                role,
                granted_by: caller,
            },
        );
    }

    pub fn revoke_role(env: Env, caller: Address, account: Address) {
        caller.require_auth();
        Self::require_role(&env, &caller, Role::SuperAdmin);

        // Issue #1026: Prevent self-revocation and removing the last SuperAdmin
        if &account == &caller {
            panic!("cannot revoke your own role");
        }

        let key = DataKey::Role(account.clone());
        if !env.storage().persistent().has(&key) {
            panic!("role not assigned");
        }

        // Check if this is a SuperAdmin
        if let Some(role) = env.storage().persistent().get::<DataKey, Role>(&key) {
            if role == Role::SuperAdmin {
                if let Some(admins) = env.storage().persistent().get::<DataKey, soroban_sdk::Vec<Address>>(&DataKey::SuperAdmins) {
                    // Prevent removing the last SuperAdmin
                    let remaining_admins: usize = admins.iter()
                        .filter(|a| {
                            if a == &account {
                                false
                            } else if let Some(role) = env.storage().persistent().get::<DataKey, Role>(&DataKey::Role(a.clone())) {
                                role == Role::SuperAdmin
                            } else {
                                false
                            }
                        })
                        .count();

                    if remaining_admins == 0 {
                        panic!("cannot revoke the last SuperAdmin");
                    }
                }
            }
        }

        // Issue #1026: Store roles in persistent() instead of instance()
        env.storage().persistent().remove(&key);
        env.events().publish(
            ("RBAC", "role_revoked"),
            RoleRevokedEvent {
                account,
                revoked_by: caller,
            },
        );
    }

    pub fn get_role(env: Env, account: Address) -> Option<Role> {
        // Issue #1026: Use persistent() instead of instance()
        env.storage().persistent().get(&DataKey::Role(account))
    }

    /// Sensitive operation requiring minimum `OperationsAdmin`.
    pub fn execute_operations_task(env: Env, caller: Address) {
        caller.require_auth();
        Self::require_role(&env, &caller, Role::OperationsAdmin);
    }

    /// Sensitive operation requiring minimum `ComplianceAdmin`.
    pub fn execute_compliance_task(env: Env, caller: Address) {
        caller.require_auth();
        Self::require_role(&env, &caller, Role::ComplianceAdmin);
    }

    /// Sensitive operation requiring minimum `ReadOnly`.
    pub fn execute_read_task(env: Env, caller: Address) {
        caller.require_auth();
        Self::require_role(&env, &caller, Role::ReadOnly);
    }

    fn require_role(env: &Env, caller: &Address, minimum_role: Role) {
        // Issue #1026: Use persistent() instead of instance()
        let caller_role = env
            .storage()
            .persistent()
            .get::<DataKey, Role>(&DataKey::Role(caller.clone()))
            .expect("role not assigned");

        if Self::role_rank(caller_role) < Self::role_rank(minimum_role) {
            panic!("insufficient role");
        }
    }

    fn role_rank(role: Role) -> u32 {
        match role {
            Role::ReadOnly => 1,
            Role::ComplianceAdmin => 2,
            Role::OperationsAdmin => 3,
            Role::SuperAdmin => 4,
        }
    }
}
