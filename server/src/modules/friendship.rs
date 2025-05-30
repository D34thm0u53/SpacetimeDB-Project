use spacetimedb::{table, Identity, ReducerContext, Timestamp, SpacetimeType};
use spacetimedsl::dsl;

use crate::modules::chats::*;

use super::common::GetCountOfOwnerIdentityRows;

#[derive(SpacetimeType, Debug, Clone, PartialEq, Eq,)]
pub enum FriendshipStatus {
    Pending,
    Rejected,
    Active,
    
}

#[dsl(plural_name = friendships, unique_index(name = party_one_and_party_two))]
#[table(name = friendship, public, index(name = party_one_and_party_two, btree(columns = [party_one, party_two])))]
pub struct Friendship {
    #[primary_key]
    #[auto_inc]
    id: u64,
    pub party_one: Identity,
    pub party_two: Identity,
    pub status: FriendshipStatus,
    created_at: Timestamp,
    modified_at: Timestamp,
}

#[spacetimedb::reducer]
pub fn send_friend_request(ctx: &ReducerContext, receiver: Identity) -> Result<(), String> {
    let sender = ctx.sender;
    if sender == receiver {
        return Err("Cannot send a friend request to yourself.".to_string());
    }
    let dsl = dsl(ctx);
    // Check if receiver is in sender's ignore list
    
    // Check if a friendship already exists
    if let Some(existing_friendship) = dsl.get_friendship_by_party_one_and_party_two(party_one, party_two)
    }
    
    dsl.create_friendship(sender, receiver, FriendshipStatus::Pending);
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