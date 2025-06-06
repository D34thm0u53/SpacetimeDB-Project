use spacetimedb::{table, Identity, ReducerContext, Timestamp, SpacetimeType};
use spacetimedsl::dsl;

use super::player::*;
use super::common::*;


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

#[derive(SpacetimeType, Debug, Clone, PartialEq, Eq,)]
pub enum FriendshipStatus {
    Pending,
    Active,
    
}


#[spacetimedb::reducer]
pub fn send_friend_request(ctx: &ReducerContext, receiver: String) -> Result<(), String> {
    let dsl = dsl(ctx);
    
    // Check if the receiver exists in the database.
    if let Some(receiver_identity) = get_player_identity_by_username(ctx, &receiver) {
        // check if a friendship request already exists
        if let Some(mut friendship_record) = get_existing_friendship_request(ctx, &receiver_identity) {
            // If the friendship exists, check its status
            match friendship_record.status {
                FriendshipStatus::Active => {
                    // Already friends
                    log::warn!("Friendship request from {} to {} is already active.", &ctx.sender, receiver);
                    return Err("You are already friends with this player.".to_string());
                    // ToDo: When sending a friend request to a player you are already friends with. Auto send the receiver a message saying sender appreciates the friendship.
                    
                }
                FriendshipStatus::Pending => {
                    // If the receiver has previously sent a request to the sender, we will auto accept it.
                    friendship_record.status = FriendshipStatus::Active;
                    // Handle the Result to avoid unused_must_use warning
                    match dsl.update_friendship_by_id(friendship_record) {
                        Ok(_) => {
                            log::info!("Successfully updated friendship status to Active");
                        }
                        Err(_) => {
                            log::warn!("Failed to update friendship status to Active");
                            return Err("We attempted to auto accept an existing fried request, but failed to save. Please try again or contact support.".to_string());
                        }
                    }
                     // A pending request already exists, so return true
                }
            }
        }
        else {
            // If no existing friendship request, proceed to create a new one
            log::debug!("No existing friendship request found from {} to {}", &ctx.sender, receiver);
            
            // Validate the friendship request
            if is_valid_friendship_request(ctx, &receiver_identity) {
                // If the request is valid, proceed to create it
                if dsl.create_friendship(ctx.sender, receiver_identity, FriendshipStatus::Pending).is_err() {
                    log::error!("Failed to create friendship request from {} to {}", &ctx.sender, receiver);
                    return Err("Failed to create friendship request.".to_string());
                }
            }
            else {
                log::warn!("Invalid friendship request from {} to {}", &ctx.sender, receiver);
                return Err("Invalid friendship request.".to_string());
            }
        }
    }
    else {
        log::warn!("Receiver {} does not exist.", receiver);
        return Err("Receiver does not exist.".to_string());
    }
    // Check if receiver is in sender's ignore list
    return Ok(());

}

#[spacetimedb::reducer]
pub fn accept_friend_request(ctx: &ReducerContext, request_sender_username: String) -> Result<(), String> {
    let dsl = spacetimedsl::dsl(ctx);

    if let Some(receiver_identity) = get_player_identity_by_username(ctx, &request_sender_username) {
        // Check if the receiver exists in the database.
        if let Some(mut friendship) = dsl.get_friendship_by_party_one_and_party_two(&receiver_identity, &ctx.sender) {
        if friendship.status == FriendshipStatus::Pending {
            friendship.status = FriendshipStatus::Active;
            dsl.update_friendship_by_id(friendship)?;
            return Ok(());
        }
        }
    } else {
        return Err("Receiver does not exist.".to_string());
    }
    return Err("No pending friend request found from this user or request already accepted.".to_string())
}

#[spacetimedb::reducer]
pub fn decline_friend_request(ctx: &ReducerContext, request_sender: String) -> Result<(), String> {
    let dsl = spacetimedsl::dsl(ctx);
    
    let sender_identity = match dsl.get_online_player_by_username(&request_sender) {
        Some(player) => player.identity,
        None => return Err("Receiver does not exist.".to_string()),
    }; 

    if sender_identity == ctx.sender {
        return Err("Cannot decline a friend request from yourself.".to_string());
    }

    let existing_friendship = dsl.get_friendship_by_party_one_and_party_two(&sender_identity, &ctx.sender);
    if let Some(friendship) = existing_friendship {
        if friendship.status == FriendshipStatus::Pending {
            dsl.delete_friendship_by_id(&friendship.id);
            return Ok(());
        }
    } else {
        // Check the reverse direction
        if let Some(friendship) = dsl.get_friendship_by_party_one_and_party_two(&ctx.sender, &sender_identity) {
            if friendship.status == FriendshipStatus::Pending {
                dsl.delete_friendship_by_id(&friendship.id);
                return Ok(());
            }
        }
    }    

    return Err("No pending friend request found from this user.".to_string())

}


/*  is_valid_friendship_request
Validates if a friendship request can be sent to the receiver.
This function checks the following conditions:
1. The receiver exists in the database.
2. The sender is not trying to friend themselves.
3. The sender has not already sent a friend request to the receiver.
4. The receiver has not sent a friend request to the sender that is still pending.

Change list:
01/06/2025 - KS - Initial Version

 */
fn is_valid_friendship_request(ctx: &ReducerContext, receiver_identity: &Identity) -> bool {
    // You cannot Friend yourself
    if &ctx.sender == receiver_identity {
        return false;
    }
    else {
        return true
    }

    // Future Validations

    // ToDo: Check if the receiver is in the sender's ignore list
    // ToDo: Check if the sender is in the receivers ignore list
    


    // If we hit this point, the request is valid

}


/*  get_existing_friendship_request
Get reciever's Identity by userrname.
If the receiver is online, we get their identity from the online player list.
If the receiver is offline, we get their identity from the offline player list.
If the receiver does not exist, we return None.

Change list :
01/06/2025 - KS - Initial Version
*/
fn get_existing_friendship_request(ctx: &ReducerContext, receiver_identity: &Identity) -> Option<Friendship> {
    let dsl: spacetimedsl::DSL<'_> = dsl(ctx);
    
    // Check if a friendship request already exists from the sender to the receiver
    if dsl.get_friendship_by_party_one_and_party_two(&ctx.sender, &receiver_identity).is_some() {
                let friendship = dsl.get_friendship_by_party_one_and_party_two(&ctx.sender, &receiver_identity).unwrap();
                return Some(friendship);
            }
    // Check if a friendship request already exists from the receiver to the sender
    if dsl.get_friendship_by_party_one_and_party_two(&receiver_identity, &ctx.sender).is_some() {
        let friendship = dsl.get_friendship_by_party_one_and_party_two(&receiver_identity, &ctx.sender).unwrap();
        return Some(friendship);
    }

    return None
}
