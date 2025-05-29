use spacetimedb::{table, reducer, Identity, ReducerContext, Table, Timestamp};

#[derive(Clone, PartialEq, Eq)]
pub enum FriendshipStatus {
    Pending,
    Active,
}

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
    if ctx.sender == addressee {
        return; // Cannot friend yourself
    }
    // Prevent duplicate requests or existing friendships
    if ctx.db.friendship().requester().addressee().find(ctx.sender, addressee).is_none() {
        ctx.db.friendship().insert(Friendship {
            requester: ctx.sender,
            addressee,
            status: FriendshipStatus::Pending,
            requested_at: ctx.timestamp,
            updated_at: ctx.timestamp,
        });
    }
}

#[reducer]
pub fn accept_friend_request(ctx: &ReducerContext, requester: Identity) {
    if let Some(mut friendship) = ctx.db.friendship().requester().addressee().find(requester, ctx.sender) {
        if friendship.status == FriendshipStatus::Pending {
            friendship.status = FriendshipStatus::Active;
            friendship.updated_at = ctx.timestamp;
            ctx.db.friendship().requester().addressee().update(friendship);
        }
    }
}

#[reducer]
pub fn decline_friend_request(ctx: &ReducerContext, requester: Identity) {
    if let Some(friendship) = ctx.db.friendship().requester().addressee().find(requester, ctx.sender) {
        if friendship.status == FriendshipStatus::Pending {
            ctx.db.friendship().requester().addressee().delete(friendship);
        }
    }
}

#[reducer]
pub fn remove_friend(ctx: &ReducerContext, other: Identity) {
    // Remove if sender is requester
    if let Some(friendship) = ctx.db.friendship().requester().addressee().find(ctx.sender, other) {
        ctx.db.friendship().requester().addressee().delete(friendship);
        return;
    }
    // Remove if sender is addressee
    if let Some(friendship) = ctx.db.friendship().requester().addressee().find(other, ctx.sender) {
        ctx.db.friendship().requester().addressee().delete(friendship);
    }
}
