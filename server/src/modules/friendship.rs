use spacetimedb::{ReducerContext, Timestamp, Identity};
use spacetimedsl::dsl;

#[derive(Clone, PartialEq, Eq, spacetimedb::SpacetimeType)]
pub enum FriendshipStatus {
    Pending,
    Active,
}

#[dsl]
#[table(name = friendship, public)]
pub struct Friendship {
    #[primary_key]
    pub requester: Identity,
    #[unique]
    pub addressee: Identity,
    pub status: FriendshipStatus,
    pub requested_at: Timestamp,
    pub updated_at: Timestamp,
}

#[reducer]
pub fn send_friend_request(ctx: &ReducerContext, addressee: Identity) {
    let dsl = dsl(ctx);
    if ctx.sender == addressee {
        return; // Cannot friend yourself
    }
    // Prevent duplicate requests or existing friendships
    if dsl.db().friendship().requester().addressee().find((ctx.sender, addressee)).is_none() {
        dsl.db().friendship().create_row(Friendship {
            requester: ctx.sender,
            addressee,
            status: FriendshipStatus::Pending,
            requested_at: ctx.timestamp,
            updated_at: ctx.timestamp,
        }).ok();
    }
}

#[spacetimedb::reducer]
pub fn accept_friend_request(ctx: &ReducerContext, requester: Identity) {
    let dsl = dsl(ctx);
    if let Some(mut friendship) = dsl.db().friendship().requester().addressee().find((requester, ctx.sender)) {
        if friendship.status == FriendshipStatus::Pending {
            friendship.status = FriendshipStatus::Active;
            friendship.updated_at = ctx.timestamp;
            dsl.db().friendship().requester().addressee().update(friendship);
        }
    }
}

#[spacetimedb::reducer]
pub fn decline_friend_request(ctx: &ReducerContext, requester: Identity) {
    let dsl = dsl(ctx);
    if let Some(friendship) = dsl.db().friendship().requester().addressee().find((requester, ctx.sender)) {
        if friendship.status == FriendshipStatus::Pending {
            dsl.db().friendship().requester().addressee().delete(friendship);
        }
    }
}

#[spacetimedb::reducer]
pub fn remove_friend(ctx: &ReducerContext, other: Identity) {
    let dsl = dsl(ctx);
    // Remove if sender is requester
    if let Some(friendship) = dsl.db().friendship().requester().addressee().find((ctx.sender, other)) {
        dsl.db().friendship().requester().addressee().delete(friendship);
        return;
    }
    // Remove if sender is addressee
    if let Some(friendship) = dsl.db().friendship().requester().addressee().find((other, ctx.sender)) {
        dsl.db().friendship().requester().addressee().delete(friendship);
    }
}
