use spacetimedb::{reducer, table, Identity, ReducerContext, Timestamp};
use spacetimedb::SpacetimeType;

use spacetimedsl::dsl;

// Store User Roles
#[dsl(plural_name = roles)]
#[spacetimedb::table(name = role, public)]
pub struct Role {
    #[primary_key]
    #[wrap]
    id: u64,
    #[unique]
    pub user_identity: Identity,
    pub is_trusted_user: bool,
    pub is_game_admin: bool,
    pub is_server_administrator: bool, 
}

#[dsl(plural_name = roles)]
#[table(name = roles_audit, private)]
pub struct RolesAudit {
    #[primary_key]
    #[auto_inc]
    #[wrap]
    id: u64,
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

// Admin Tools

#[reducer]
/// Allows game admins and server admins to set another player's roles.
pub fn set_player_roles(ctx: &ReducerContext, target_identity: Identity, requested_role: RoleType) {
    // Authorization check: Ensure the caller is a game admin or server admin
    if !is_admin_tools_authorized(ctx) {
        log::warn!("Unauthorized attempt to set roles by {:?}", ctx.sender);
        return;
    }

    let dsl = dsl(ctx);

    if let Some(mut user_roles_profile) = dsl.get_role_by_user_identity(&target_identity) {
        let previous_role = &get_role_type(&user_roles_profile);

        // Update the target player's roles
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
                user_roles_profile.is_trusted_user = true;
                user_roles_profile.is_game_admin = true;
                user_roles_profile.is_server_administrator = false;
            },
            RoleType::ServerAdmin => {
                user_roles_profile.is_trusted_user = true;
                user_roles_profile.is_game_admin = true;
                user_roles_profile.is_server_administrator = true;
            },
        };


        dsl.update_role_by_user_identity(user_roles_profile).expect("Failed to update user roles");

        // Log the role change in the audit table
        dsl.create_roles_audit(ctx.sender, previous_role.clone(), requested_role.clone())
            .expect("Failed to create roles audit record");

    } else {
        log::warn!("Target identity {:?} not found in roles table", target_identity);
    }
}


// Helper function to check if the caller is authorized to use admin tools
fn is_admin_tools_authorized(ctx: &ReducerContext) -> bool {
    let dsl = dsl(ctx);

    match dsl.get_role_by_user_identity(&ctx.sender) {
        Some(roles) => roles.is_game_admin || roles.is_server_administrator,
        None => false,
    }
}
