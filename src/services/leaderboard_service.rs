use std::sync::Arc;
use futures::stream::TryStreamExt;
use mongodb::{Collection, Database, bson::{doc, oid::ObjectId}, options::FindOptions};
use crate::models::leaderboard::LeaderboardEntry;
use crate::services::user_service::UserService;
pub struct LeaderboardService {
    collection: Collection<LeaderboardEntry>,
}
 
impl LeaderboardService {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            collection: db.collection("leaderboards"),
        }
    }

pub async fn get_leaderboard(&self, category_id: ObjectId) -> Result<Vec<LeaderboardEntry>, String> {
    let mut cursor = self.collection.find(
        doc! { "category_id": category_id },
    )
    .sort(doc! { "rank": 1 })
    .await.map_err(|e| e.to_string())?;

    let mut leaderboard: Vec<LeaderboardEntry> = Vec::new();
    while let Some(entry) = cursor.try_next().await.map_err(|e| e.to_string())? {
        leaderboard.push(entry); 
    }

    Ok(leaderboard)
}

    pub async fn update_leaderboard(&self, user_id: ObjectId, category_id: ObjectId, score: i32) -> Result<(), String> {
        // Find existing entry for the user and category
        let filter = doc! { "user_id": user_id, "category_id": category_id };
        let existing_entry = self.collection.find_one(filter.clone(), ).await.map_err(|e| e.to_string())?;

        let new_score = if let Some(entry) = existing_entry {
            // Update existing score
            entry.score + score
        } else {
            // New entry
            score
        };

        // Upsert the user's score
     self.collection.update_one(
    filter,
    doc! { "$set": { "score": new_score, "user_id": user_id, "category_id": category_id } },
)
.upsert(true)
.await
.map_err(|e| e.to_string())?;

// Recalculate ranks for the category
let mut cursor = self.collection.find(
    doc! { "category_id": category_id },
)
.sort(doc! { "score": -1 })
.await
.map_err(|e| e.to_string())?;

        let mut ranked_entries: Vec<LeaderboardEntry> = Vec::new();
        let mut rank = 1;
        while let Some(mut entry) = cursor.try_next().await.map_err(|e| e.to_string())? { // Assuming LeaderboardEntry has an `id` field
            entry.rank = rank;
            self.collection.update_one(doc! { "user_id": entry.user_id, "category_id": entry.category_id }, doc! { "$set": { "rank": rank } }, ).await.map_err(|e| e.to_string())?;
            ranked_entries.push(entry);
            rank += 1;
        }
        Ok(())
    }
}
