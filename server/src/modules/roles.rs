use spacetimedb::{reducer, table, Identity, ReducerContext, Table, Timestamp};
use spacetimedb::SpacetimeType;


// Store User Roles
#[derive(Clone)]
#[spacetimedb::table(name = roles, public)]
pub struct Roles {
    #[primary_key]
    pub id: u64,
    #[unique]
    pub identity: Identity,
    pub is_trusted_user: bool,
    pub is_game_admin: bool,
    pub is_server_administrator: bool, 
}

#[table(name = roles_audit, private)]
pub struct RolesAudit {
    identity: Identity,
    previous_role: RoleType,
    new_role: RoleType,
    timestamp: Timestamp,
}

#[derive(SpacetimeType, Debug, Clone, PartialEq, Eq,)]
pub enum RoleType {
    User,
    TrustedUser,
    GameAdmin,
    ServerAdmin,
}

#[reducer]
pub fn set_user_role(ctx: &ReducerContext, role: RoleType) {

    // Check if the user already exists in the database
    if let Some(user) = ctx.db.roles().identity().find(ctx.sender) {
        // Log the role change request in the audit table
        ctx.db.roles_audit().insert(RolesAudit {
            identity: ctx.sender,
            previous_role: get_role_type(&user),
            new_role: role.clone(),
            timestamp: ctx.timestamp,
        });

        // User already exists, update their role
        match role {
            RoleType::User => {
                ctx.db.roles().identity().update(Roles {
                    is_trusted_user: false,
                    is_game_admin: false,
                    is_server_administrator: false,
                    ..user
                });
            }
            RoleType::TrustedUser => {
                ctx.db.roles().identity().update(Roles {
                    is_trusted_user: true,
                    is_game_admin: false,
                    is_server_administrator: false,
                    ..user
                });
            }
            RoleType::GameAdmin => {
                ctx.db.roles().identity().update(Roles {
                    is_trusted_user: true,
                    is_game_admin: true,
                    is_server_administrator: false,
                    ..user
                });
            }
            RoleType::ServerAdmin => {
                ctx.db.roles().identity().update(Roles {
                    is_trusted_user: true,
                    is_game_admin: true,
                    is_server_administrator: true,
                    ..user
                });
            }
        };
    } else {
        // This is a new user, create a new entry in the database. New users are always a base User
        ctx.db.roles().insert(Roles {
            id: 0,
            identity: ctx.sender,
            is_trusted_user: false,
            is_game_admin: false,
            is_server_administrator: false,
        });
    }
}


// Helper function to determine the RoleType from the Roles struct
fn get_role_type(roles: &Roles) -> RoleType {
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
pub fn set_player_roles(ctx: &ReducerContext, target_identity: Identity, role: RoleType) {
    // Authorization check: Ensure the caller is a game admin or server admin
    if !is_admin_tools_authorized(ctx, ctx.sender) {
        log::warn!("Unauthorized attempt to set roles by {:?}", ctx.sender);
        return;
    }

    // Fetch the target player's current roles
    if let Some(current_roles) = ctx.db.roles().identity().find(target_identity) {
        let previous_role = get_role_type(&current_roles);

        // Update the target player's roles
        match role {
            RoleType::User => ctx.db.roles().identity().update(Roles {
                is_trusted_user: false,
                is_game_admin: false,
                is_server_administrator: false,
                ..current_roles
            }),
            RoleType::TrustedUser => ctx.db.roles().identity().update(Roles {
                is_trusted_user: true,
                is_game_admin: false,
                is_server_administrator: false,
                ..current_roles
            }),
            RoleType::GameAdmin => ctx.db.roles().identity().update(Roles {
                is_trusted_user: true,
                is_game_admin: true,
                is_server_administrator: false,
                ..current_roles
            }),
            RoleType::ServerAdmin => ctx.db.roles().identity().update(Roles {
                is_trusted_user: true,
                is_game_admin: true,
                is_server_administrator: true,
                ..current_roles
            }),
        };

        // Log the role change in the audit table
        ctx.db.roles_audit().insert(RolesAudit {
            identity: target_identity,
            previous_role,
            new_role: role,
            timestamp: ctx.timestamp,
        });
    } else {
        log::warn!("Target identity {:?} not found in roles table", target_identity);
    }
}


// Helper function to check if the caller is authorized to use admin tools
fn is_admin_tools_authorized(ctx: &ReducerContext, caller_identity: Identity) -> bool {
    if let Some(caller_roles) = ctx.db.roles().identity().find(caller_identity) {
        caller_roles.is_game_admin || caller_roles.is_server_administrator
    } else {
        false
    }
}
