


use std::sync::Arc;
use mongodb::{Collection, Database};
use bson::{doc, oid::ObjectId};
use futures::stream::TryStreamExt;
use crate::models::{
    category::Category,
    question::Question,
};

pub struct QuestionService {
    category_collection: Collection<Category>,
    question_collection: Collection<Question>,
}

impl QuestionService {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            category_collection: db.collection("categories"),
            question_collection: db.collection("questions"),
        }
    }

    // ---------- Category Methods ----------
    pub async fn create_category(&self, category: Category) -> Result<String, String> {
        let insert_result = self.category_collection
            .insert_one(category)
            .await
            .map_err(|_| "Failed to create category".to_string())?;
        Ok(insert_result.inserted_id.as_object_id().unwrap().to_hex())
    }

    pub async fn get_category(&self, id: ObjectId) -> Result<Category, String> {
        self.category_collection
            .find_one(doc! { "_id": id })
            .await
            .map_err(|_| "Failed to fetch category".to_string())?
            .ok_or("Category not found".to_string())
    }

pub async fn list_categories(&self) -> Result<Vec<Category>, String> {
    let mut cursor = self.category_collection
        .find(doc! {}) // <-- empty filter matches all
        .await
        .map_err(|_| "Failed to list categories".to_string())?;

    let mut categories = Vec::new();
    while let Some(cat) = cursor
        .try_next()
        .await
        .map_err(|_| "Error iterating categories".to_string())? 
    {
        categories.push(cat);
    }
    Ok(categories)
}
    pub async fn delete_category(&self, id: ObjectId) -> Result<(), String> {
        let result = self.category_collection
            .delete_one(doc! { "_id": id })
            .await
            .map_err(|_| "Failed to delete category".to_string())?;

        if result.deleted_count == 0 {
            return Err("Category not found".to_string());
        }
        Ok(())
    }

    // ---------- Question Methods ----------
    pub async fn create_question(&self, question: Question) -> Result<String, String> {
        let insert_result = self.question_collection
            .insert_one(question)
            .await
            .map_err(|_| "Failed to create question".to_string())?;
        Ok(insert_result.inserted_id.as_object_id().unwrap().to_hex())
    }

    pub async fn get_question(&self, id: ObjectId) -> Result<Question, String> {
        self.question_collection
            .find_one(doc! { "_id": id })
            .await
            .map_err(|_| "Failed to fetch question".to_string())?
            .ok_or("Question not found".to_string())
    }

    pub async fn list_questions(&self, category_id: Option<ObjectId>) -> Result<Vec<Question>, String> {
        let filter = category_id.map(|id| doc! { "category_id": id }).unwrap_or(doc! {});
        let mut cursor = self.question_collection
            .find(filter, )
            .await
            .map_err(|_| "Failed to list questions".to_string())?;

        let mut questions = Vec::new();
        while let Some(q) = cursor.try_next().await.map_err(|_| "Error iterating questions".to_string())? {
            questions.push(q);
        }
        Ok(questions)
    }

    pub async fn delete_question(&self, id: ObjectId) -> Result<(), String> {
        let result = self.question_collection
            .delete_one(doc! { "_id": id })
            .await
            .map_err(|_| "Failed to delete question".to_string())?;

        if result.deleted_count == 0 {
            return Err("Question not found".to_string());
        }
        Ok(())
    }
}
