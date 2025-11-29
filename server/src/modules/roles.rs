use spacetimedb::{reducer, table, Identity, ReducerContext, Timestamp};
use spacetimedb::SpacetimeType;

use spacetimedsl::*;

use crate::modules::player::*;
use crate::modules::common::*;

// Store User Roles
#[dsl(plural_name = roles,
    method(
        update = true,
        delete = true
    )
)]
#[table(name = role,
    public
)]
pub struct Role {
    #[auto_inc]
    #[primary_key]
    #[index(btree)]
    #[create_wrapper]
    id: u32,
    #[unique]
    #[use_wrapper(crate::modules::player::PlayerAccountId)]
    #[foreign_key(path = crate::modules::player, table = player_account, column = id, on_delete = Delete)]
    pub user_id: u32,
    pub is_trusted_user: bool,
    pub is_game_admin: bool,
    pub is_server_administrator: bool, 
}



#[dsl(plural_name = roles_audits,
    method(
        update = false
    )
)]
#[table(name = roles_audit, private)]
pub struct RolesAudit {
    #[primary_key]
    #[auto_inc]
    #[create_wrapper]
    id: u32,
    actioner: Identity,
    identity: Identity,
    previous_role: RoleType,
    new_role: RoleType,
    created_at: Timestamp,
}

#[derive(SpacetimeType, Debug, Clone, PartialEq, Eq,)]
pub enum RoleType {
    GuestUser,
    TrustedUser,
    GameAdmin,
    ServerAdmin,
}

fn get_role_type(roles: &Role) -> RoleType {
    if roles.is_server_administrator {
        RoleType::ServerAdmin
    } else if roles.is_game_admin {
        RoleType::GameAdmin
    } else if roles.is_trusted_user {
        RoleType::TrustedUser
    } else {
        RoleType::GuestUser
    }
}

impl Role {
    fn update_role(&mut self, requested_role: &RoleType) {
        // Update the target player's roles based on requested role
        match requested_role {
            RoleType::GuestUser => {
                self.is_trusted_user = false;
                self.is_game_admin = false;
                self.is_server_administrator = false;
            },
            RoleType::TrustedUser => {
                self.is_trusted_user = true;
                self.is_game_admin = false;
                self.is_server_administrator = false;
            },
            RoleType::GameAdmin => {
                // Game admins inherit trusted user permissions
                self.is_trusted_user = true;
                self.is_game_admin = true;
                self.is_server_administrator = false;
            },
            RoleType::ServerAdmin => {
                // Server admins inherit all permissions
                self.is_trusted_user = true;
                self.is_game_admin = true;
                self.is_server_administrator = true;
            }
        }
    }
    
}


// Create default roles for new users
pub fn create_default_roles(dsl: &spacetimedsl::DSL, user_id: PlayerAccountId) -> Result<Role, spacetimedsl::SpacetimeDSLError> {
    dsl.create_role(CreateRole {
        user_id,
        is_trusted_user: false,
        is_game_admin: false,
        is_server_administrator: false,
    })
}




// Is the caller a game admin or server admin?
fn is_admin_tools_authorized(ctx: &ReducerContext) -> bool {
    let dsl = dsl(ctx);

    let requesting_user = dsl.get_player_account_by_identity(&ctx.sender);
    if requesting_user.is_ok() {
        let target_user_role = dsl.get_role_by_user_id(requesting_user.unwrap().get_id());

        if target_user_role.is_ok() {
            let roles = target_user_role.unwrap();
            if roles.is_game_admin || roles.is_server_administrator {
                return true;
            }
        }
    }
    return false
}


// Admin Tools
#[reducer]
/// Allows game admins and server admins to override another player's role.
pub fn set_player_roles(ctx: &ReducerContext, target_identity: Identity, requested_role: RoleType) -> Result<(), String> {
    if !try_server_or_dev(ctx) {
        if !is_admin_tools_authorized(ctx) {
            log::warn!("SECURITY: Unauthorized attempt to set roles by {:?}", ctx.sender);
            return Err("Unauthorized access".to_string());
        }
    }
    
    let dsl = dsl(ctx);

    // Get target account and handle errors properly
    let target_account: PlayerAccount = dsl.get_player_account_by_identity(&target_identity)?;
    let mut user_roles_profile: Role = dsl.get_role_by_user_id(&target_account.get_id())?;
    let previous_user_roles_profile = get_role_type(&user_roles_profile);

    // Update the target player's roles based on requested role
    user_roles_profile.update_role(&requested_role);

    // Update the role in the database
    dsl.update_role_by_user_id(user_roles_profile)
        .map_err(|e| format!("Failed to update user roles: {:?}", e))?;

    // Log the role change in the audit table, recording who performed the change
    dsl.create_roles_audit(CreateRolesAudit {
        actioner: ctx.sender,
        identity: target_identity,
        previous_role: previous_user_roles_profile.clone(),
        new_role: requested_role.clone(),
    })?;
    
    log::warn!(
        "Role updated for user {} from {:?} to {:?} by {} (requires monitoring)",
        target_identity, previous_user_roles_profile, requested_role, ctx.sender
    );

    Ok(())
}


// Check if a user has a specific role
pub fn has_role(ctx: &ReducerContext, user_identity: &Identity, role_type: &RoleType) -> bool {
    let dsl = dsl(ctx);

    let user_account = dsl.get_player_account_by_identity(user_identity);
    if user_account.is_ok() {
        let user_roles = dsl.get_role_by_user_id(&user_account.unwrap().get_id());
        if user_roles.is_ok() {
            let roles = user_roles.unwrap();
            match role_type {
                RoleType::GuestUser => true, // All users are at least GuestUser
                RoleType::TrustedUser => roles.is_trusted_user,
                RoleType::GameAdmin => roles.is_game_admin,
                RoleType::ServerAdmin => roles.is_server_administrator,
            }
        } else {
            false
        }
    } else {
        false
    }
}