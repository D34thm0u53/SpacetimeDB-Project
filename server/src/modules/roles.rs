use spacetimedb::{reducer, table, Identity, ReducerContext, Timestamp};
use spacetimedb::SpacetimeType;

use spacetimedsl::{dsl, Wrapper};

use crate::modules::player::*;
use crate::modules::common::*;

// Store User Roles
#[dsl(plural_name = roles)]
#[table(name = role, public)]
pub struct Role {
    #[primary_key]
    #[create_wrapper]
    id: u32,
    #[unique]
    #[use_wrapper(path = crate::modules::player::PlayerAccountId)]
    #[foreign_key(path = crate::modules::player, table = player_account, column = id, on_delete = Delete)]
    pub user_id: u32,
    pub is_trusted_user: bool,
    pub is_game_admin: bool,
    pub is_server_administrator: bool, 
}

#[dsl(plural_name = roles)]
#[table(name = roles_audit, private)]
pub struct RolesAudit {
    #[primary_key]
    #[auto_inc]
    #[create_wrapper]
    id: u32,
    identity: Identity,
    previous_role: RoleType,
    new_role: RoleType,
    created_at: Timestamp,
}

#[derive(SpacetimeType, Debug, Clone, PartialEq, Eq,)]
pub enum RoleType {
    User,
    TrustedUser,
    GameAdmin,
    ServerAdmin,
}

// Admin Tools

#[reducer]
/// Allows game admins and server admins to set another player's roles.
pub fn set_player_roles(ctx: &ReducerContext, target_identity: Identity, requested_role: RoleType) -> Result<(), String> {
    if !try_server_or_dev(ctx) {
        log::warn!("Unauthorized attempt to set roles by {:?}", ctx.sender);
        return Err("Unauthorized access".to_string());
    }
    
    


    
    let dsl = dsl(ctx);

    // Get target account and handle errors properly
    let target_account = dsl.get_player_account_by_identity(&target_identity)
        .map_err(|e| format!("Failed to find player account: {:?}", e))?;

    let target_user_id = target_account.get_id();

    // Fetch or create the user's role profile
    let mut user_roles_profile = match dsl.get_role_by_user_id(&target_user_id) {
        Ok(existing_role) => existing_role,
        Err(spacetimedsl::SpacetimeDSLError::NotFoundError { .. }) => {
            // Create a default role if none exists

            dsl.create_role(
                0,
                target_user_id,
                false,
                false,
                false
            )
            .map_err(|e| format!("Failed to create default role for user: {:?}", e))?
        }
        Err(e) => {
            return Err(format!("Failed to retrieve role: {:?}", e));
        }
    };

    let previous_role = get_role_type(&user_roles_profile);

    // Update the target player's roles based on requested role
    match requested_role {
        RoleType::User => {
            user_roles_profile.is_trusted_user = false;
            user_roles_profile.is_game_admin = false;
            user_roles_profile.is_server_administrator = false;
        },
        RoleType::TrustedUser => {
            user_roles_profile.is_trusted_user = true;
            user_roles_profile.is_game_admin = false;
            user_roles_profile.is_server_administrator = false;
        },
        RoleType::GameAdmin => {
            // Game admins inherit trusted user permissions
            user_roles_profile.is_trusted_user = true;
            user_roles_profile.is_game_admin = true;
            user_roles_profile.is_server_administrator = false;
        },
        RoleType::ServerAdmin => {
            // Server admins inherit all permissions
            user_roles_profile.is_trusted_user = true;
            user_roles_profile.is_game_admin = true;
            user_roles_profile.is_server_administrator = true;
        }
    }

    // Update the role in the database
    dsl.update_role_by_user_id(user_roles_profile)
        .map_err(|e| format!("Failed to update user roles: {:?}", e))?;


    // Log the role change in the audit table, recording who performed the change
    dsl.create_roles_audit(ctx.sender.clone(), previous_role.clone(), requested_role.clone())
        .map_err(|e| format!("Failed to create audit log: {:?}", e))?;
    
    log::info!(
        "[Reducer: set_player_roles] Role updated for user {} from {:?} to {:?} by {} (action: set_player_roles)",
        target_identity, previous_role, requested_role, ctx.sender
    );

    Ok(())
}


// Helper function to check if the caller is authorized to use admin tools
fn is_admin_tools_authorized(ctx: &ReducerContext) -> bool {
    let dsl = dsl(ctx);
    let mut authorised = false;

    let target_user = dsl.get_player_account_by_identity(&ctx.sender);

    if target_user.is_ok() {
        let target_user_role = dsl.get_role_by_user_id(target_user.unwrap().get_id());

        if target_user_role.is_ok() {
            let roles = target_user_role.unwrap();
            if roles.is_game_admin || roles.is_server_administrator {
                authorised = true;
            }
        }
    }

    return authorised
}

// Helper function to determine the RoleType from the Roles struct
fn get_role_type(roles: &Role) -> RoleType {
    if roles.is_server_administrator {
        RoleType::ServerAdmin
    } else if roles.is_game_admin {
        RoleType::GameAdmin
    } else if roles.is_trusted_user {
        RoleType::TrustedUser
    } else {
        RoleType::User
    }
}