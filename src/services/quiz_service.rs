use std::sync::Arc;

use bson::{doc, oid::ObjectId};
use chrono::Utc;
use futures::TryStreamExt;
use mongodb::{Collection, Database};
use uuid::Uuid;

use crate::{models::{category::{Category, CategoryResponse, CategoryWithTopUserResponse}, question::{Difficulty, Question}, quiz::Quiz, user::{User, UserResponse}}, services::user_service::UserService};



pub struct QuizService {
    quiz_collection: Collection<Quiz>,
    category_collection: Collection<Category>,
    question_collection: Collection<Question>,
}

impl QuizService {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            quiz_collection: db.collection("quizzes"),
            category_collection: db.collection("categories"),
            question_collection: db.collection("questions"),
        }
    }


    pub async fn get_categories_with_top_users(&self, user_service: &UserService) -> Result<Vec<CategoryWithTopUserResponse>, String> {
        let pipeline = vec![
            // 1. Group quizzes by category and user, summing scores
            doc! { "$group": { "_id": { "category_id": "$category_id", "user_id": "$user_id" }, "total_score": { "$sum": "$score" } } },
            // 2. Sort by score to find the top score for each category
            doc! { "$sort": { "total_score": -1 } },
            // 3. Group by category to get the top user and their score
            doc! { "$group": { "_id": "$_id.category_id", "top_user_id": { "$first": "$_id.user_id" }, "top_score": { "$first": "$total_score" } } },
            // 4. Join with the categories collection to get category details
            doc! { "$lookup": { "from": "categories", "localField": "_id", "foreignField": "_id", "as": "category_info" } },
            // 5. Unwind the category_info array
            doc! { "$unwind": "$category_info" },
            // 6. Join with the users collection to get top user details
            doc! { "$lookup": { "from": "users", "localField": "top_user_id", "foreignField": "_id", "as": "top_user_info" } },
            // 7. Project the final shape
            doc! { "$project": {
                "category": "$category_info",
                "top_user": { "$arrayElemAt": ["$top_user_info", 0] }
            } },
        ];

        let mut cursor = self.quiz_collection.aggregate(pipeline, ).await.map_err(|e| e.to_string())?;
        let mut results = Vec::new();

        while let Some(doc) = cursor.try_next().await.map_err(|e| e.to_string())? {
            let category_doc = doc.get_document("category").map_err(|_| "Category document missing".to_string())?;
             let category: Category = bson::from_document(category_doc.clone()).map_err(|e| e.to_string())?;
            let top_user: Option<User> = doc.get("top_user").and_then(|u| bson::from_bson(u.clone()).ok());

            results.push(CategoryWithTopUserResponse {
                category: CategoryResponse::from((category, None)),
                top_user: top_user.map(|u| u.into()),
            });
        }

        // Now, get categories that have no quizzes taken yet
        let categories_with_scores_ids: Vec<ObjectId> = results.iter().map(|r| ObjectId::parse_str(r.category.id.as_ref().unwrap()).unwrap()).collect();
        let categories_without_scores_cursor = self.category_collection.find(doc! { "_id": { "$nin": categories_with_scores_ids } }, ).await.map_err(|e| e.to_string())?;
        let categories_without_scores: Vec<Category> = categories_without_scores_cursor.try_collect().await.map_err(|e| e.to_string())?;

        let without_scores_responses: Vec<CategoryWithTopUserResponse> = categories_without_scores
            .into_iter()
            .map(|c| CategoryWithTopUserResponse {
                category: CategoryResponse::from((c, None)),
                top_user: None,
            })
            .collect();

        results.extend(without_scores_responses);
        Ok(results)
    }

       pub async fn top_user_for_category(
        &self,
        category_id: ObjectId,
        user_service: &UserService,
    ) -> Result<Option<UserResponse>, String> {
        // Aggregate quizzes by user_id, sum scores, filter by category_id
        let pipeline = vec![
            doc! { "$match": { "category_id": category_id.clone() } },
            doc! { "$group": { "_id": "$user_id", "total_score": { "$sum": "$score" } } },
            doc! { "$sort": { "total_score": -1 } },
            doc! { "$limit": 1 },
        ];
        let mut cursor = self.quiz_collection.aggregate(pipeline,)
            .await
            .map_err(|_| "Failed to aggregate scores".to_string())?;

        if let Some(doc) = cursor.try_next().await.map_err(|_| "Error iterating".to_string())? {
            if let Ok(user_id) = doc.get_object_id("_id") {
                let user = user_service.get_user(user_id).await?;
                Ok(Some(user.into()))
            } else {
                // No user has taken a quiz in this category yet
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }


    pub async fn start_quiz(
        &self,
        user_id: ObjectId,
        category_id: ObjectId,
        difficulty: Difficulty,
        num_questions: i32,
    ) -> Result<Quiz, String> {


    let  difficulty_str:&str = match difficulty {
    Difficulty::Beginner => "Beginner",
    Difficulty::Intermediate => "Intermediate",
    Difficulty::Advanced => "Advanced",
    Difficulty::Expert => "Expert",
};

        let mut cursor = self.question_collection.aggregate(vec![
            doc! { "$match": { "category_id": &category_id, "difficulty": difficulty_str } },
            doc! { "$sample": { "size": num_questions } }
        ], ).await.map_err(|_| "Failed to fetch questions".to_string())?;

        let mut questions = Vec::new();
        while let Some(doc) = cursor.try_next().await.map_err(|_| "Error iterating questions".to_string())? {
            let q: Question = bson::from_document(doc).map_err(|_| "Failed to parse question".to_string())?;
            questions.push(q);
        }

        if questions.len() < num_questions as usize {
            return Err("Not enough questions available".to_string());
        }

        let quiz = Quiz {
            id: None,
            uuid: Uuid::new_v4(),
            user_id,
            category_id,
            difficulty,
            questions: questions.iter().map(|q| q.id.clone().unwrap()).collect(),
            answers: vec![],
            start_time: Utc::now(),
            end_time: None,
            score: 0,
            paused: false,
        };

        let insert_result = self.quiz_collection
            .insert_one(quiz.clone(), )
            .await
            .map_err(|_| "Failed to start quiz".to_string())?;

        let mut quiz_with_id = quiz;
        quiz_with_id.id = insert_result.inserted_id.as_object_id();

        Ok(quiz_with_id)
    }

    pub async fn submit_answer(
        &self,
        quiz_id: ObjectId,
        question_id: ObjectId,
        answer: String,
        time_taken: i64,
    ) -> Result<Quiz, String> {
        let mut quiz = self.quiz_collection
            .find_one(doc! { "_id": &quiz_id }, )
            .await
            .map_err(|_| "Failed to fetch quiz".to_string())?
            .ok_or("Quiz not found".to_string())?;

        if quiz.paused {
            return Err("Cannot submit answer to a paused quiz".to_string());
        }

        let question = self.question_collection
            .find_one(doc! { "_id": &question_id }, )
            .await
            .map_err(|_| "Failed to fetch question".to_string())?
            .ok_or("Question not found".to_string())?;

        let correct = question.correct_answer == answer;
        let base_points = match question.difficulty {
            Difficulty::Beginner => 5,
            Difficulty::Intermediate => 10,
            Difficulty::Advanced => 20,
            Difficulty::Expert => 30,
        };
        let time_bonus = if time_taken < question.timer.num_seconds() / 2 { 10 } else { 0 };
        let points = if correct { base_points + time_bonus } else { 0 };

        quiz.score += points;
        quiz.answers.push(crate::models::quiz::UserAnswer {
            question_id,
            answer,
            time_taken: chrono::Duration::seconds(time_taken),
            correct,
        });

        let anser_bson = bson::to_bson(&quiz.answers).map_err(|_| "Failed to serialize answers".to_string())?;

        self.quiz_collection.update_one(
            doc! { "_id": &quiz_id },
            doc! { "$set": { "answers": anser_bson, "score": quiz.score } },
            
        ).await.map_err(|_| "Failed to update quiz".to_string())?;

        Ok(quiz)
    }

   pub async fn finish_quiz(    
    &self, 
    user_service: &UserService,  
    quiz_id: ObjectId
) -> Result<i32, String> {

    let time_bson = bson::to_bson(&Utc::now()).map_err(|_| "Failed to serialize end_time".to_string())?;
    let update = doc! { "$set": { "end_time": time_bson, "paused": false } };

    self.quiz_collection
        .update_one(doc! { "_id": quiz_id }, update, )
        .await
        .map_err(|_| "Failed to finish quiz".to_string())?;

    let quiz = self.quiz_collection
        .find_one(doc! { "_id": quiz_id }, )
        .await
        .map_err(|_| "Failed to fetch quiz".to_string())?
        .ok_or("Quiz not found".to_string())?;

    // ✅ Add quiz history
    user_service
        .add_quiz_history(quiz.user_id, quiz.uuid)
        .await?;

    // ✅ Award XP based on score (customize logic as you like)
    let xp_earned = quiz.score; // simple: 1 XP per point of score
    if xp_earned > 0 {
        user_service
            .add_xp(quiz.user_id, xp_earned)
            .await?;
    }

    Ok(quiz.score)
}

}
