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
            Some(FindOptions::builder().sort(doc! { "rank": 1 }).build()),
        ).await.map_err(|e| e.to_string())?;

        let mut leaderboard: Vec<LeaderboardEntry> = Vec::new();
        while let Some(entry) = cursor.try_next().await.map_err(|e| e.to_string())? { // Fix: Removed the extra argument
            leaderboard.push(entry); 
        }

        Ok(leaderboard)
    }

    pub async fn update_leaderboard(&self, user_id: ObjectId, category_id: ObjectId, score: i32) -> Result<(), String> {
        // Find existing entry for the user and category
        let filter = doc! { "user_id": user_id, "category_id": category_id };
        let existing_entry = self.collection.find_one(filter.clone(), None).await.map_err(|e| e.to_string())?;

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
            mongodb::options::UpdateOptions::builder().upsert(true).build(),
        ).await.map_err(|e| e.to_string())?;

        // Recalculate ranks for the category
        let mut cursor = self.collection.find(
            doc! { "category_id": category_id },
            FindOptions::builder().sort(doc! { "score": -1 }).build(),
        ).await.map_err(|e| e.to_string())?;

        let mut ranked_entries: Vec<LeaderboardEntry> = Vec::new();
        let mut rank = 1;
        while let Some(mut entry) = cursor.try_next().await.map_err(|e| e.to_string())? {
            entry.rank = rank;
            self.collection.update_one(doc! { "_id": entry.id }, doc! { "$set": { "rank": rank } }, None).await.map_err(|e| e.to_string())?;
            ranked_entries.push(entry);
            rank += 1;
        }
        Ok(())
    }
}
