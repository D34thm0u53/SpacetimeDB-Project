use spacetimedb::{ReducerContext, Timestamp, Identity};
use spacetimedsl::dsl;

use crate::modules::chats::*;

#[derive(Clone, PartialEq, Eq, spacetimedb::SpacetimeType)]
pub enum FriendshipStatus {
    Pending,
    Active,
}

#[dsl(plural_name = friendships, unique_index(name = party_one_&_party_two))]
#[table(name = friendship, public, )]
pub struct Friendship {
    #[primary_key]
    id: u64,
    pub party_one: Identity,
    pub party_two: Identity,
    pub status: FriendshipStatus,
    pub updated_at: Timestamp,
}

#[spacetimedb::reducer]
pub fn send_friend_request(ctx: &ReducerContext, receiver: Identity) -> Result<(), String> {
    let sender = ctx.sender_identity();
    if sender == receiver {
        return Err("Cannot send a friend request to yourself.".to_string());
    }

    let dsl = spacetimedsl::dsl(ctx);

    // Check if receiver is in sender's ignore list
    if IgnorePair::filter_by_ignorer_and_ignored_user(&dsl, &sender, &receiver).is_some() {
        // Silently fail as per "Send to Ignored" workflow
        return Ok(());
    }

    // Check if sender is in receiver's ignore list
    if IgnorePair::filter_by_ignorer_and_ignored_user(&dsl, &receiver, &sender).is_some() {
        // Silently fail as per "Send From Ignored" workflow
        return Ok(());
    }

    // Check for existing friendship or pending request
    // Case 1: Sender is party_one, Receiver is party_two
    if let Some(mut friendship) = Friendship::filter_by_party_one_and_party_two(&dsl, &sender, &receiver) {
        if friendship.status == FriendshipStatus::Active {
            // "Send to Existing" workflow
            return Ok(()); // Already friends, do nothing
        }
        // If pending, do nothing, let the other party act or this request is a duplicate.
        return Ok(());
    }

    // Case 2: Receiver is party_one, Sender is party_two
    if let Some(mut friendship) = Friendship::filter_by_party_one_and_party_two(&dsl, &receiver, &sender) {
        if friendship.status == FriendshipStatus::Active {
            // "Send to Existing" workflow
            return Ok(()); // Already friends, do nothing
        }
        if friendship.status == FriendshipStatus::Pending {
            // "Send and Send" workflow: John sends to Mike, Mike had already sent to John.
            // Accept the existing request.
            friendship.status = FriendshipStatus::Active;
            friendship.updated_at = ctx.timestamp();
            Friendship::update_by_id(&dsl, &friendship.id, friendship)?;
            return Ok(());
        }
    }

    // No existing active friendship or reverse pending request, create a new pending request.
    dsl.create_friendship(sender, receiver, FriendshipStatus::Pending, ctx.timestamp())?;

    Ok(())
}

#[spacetimedb::reducer]
pub fn accept_friend_request(ctx: &ReducerContext, request_sender: Identity) -> Result<(), String> {
    let request_receiver = ctx.sender_identity();
    let dsl = spacetimedsl::dsl(ctx);

    // Friendships are stored with party_one < party_two, but requests can be initiated by either.
    // We need to find the pending request where request_sender is party_one and request_receiver is party_two
    if let Some(mut friendship) = Friendship::filter_by_party_one_and_party_two(&dsl, &request_sender, &request_receiver) {
        if friendship.status == FriendshipStatus::Pending {
            friendship.status = FriendshipStatus::Active;
            friendship.updated_at = ctx.timestamp();
            Friendship::update_by_id(&dsl, &friendship.id, friendship)?;
            return Ok(());
        } else if friendship.status == FriendshipStatus::Active {
            return Err("Friendship is already active.".to_string());
        }
    }

    Err("No pending friend request found from this user or request already accepted.".to_string())
}

#[spacetimedb::reducer]
pub fn decline_friend_request(ctx: &ReducerContext, request_sender: Identity) -> Result<(), String> {
    let request_receiver = ctx.sender_identity();
    let dsl = spacetimedsl::dsl(ctx);

    // Find the pending request
    // Request sender is party_one, receiver (current user) is party_two
    if let Some(friendship) = Friendship::filter_by_party_one_and_party_two(&dsl, &request_sender, &request_receiver) {
        if friendship.status == FriendshipStatus::Pending {
            Friendship::delete_by_id(&dsl, &friendship.id)?;
            return Ok(());
        }
    }
    
    Err("No pending friend request found from this user to decline.".to_string())
}

// TODO: Add reducers for managing ignore list: add_to_ignore_list, remove_from_ignore_list
// TODO: Add reducer for removing an active friendship (unfriending)

