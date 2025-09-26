use std::sync::Arc;

use bson::{doc, oid::ObjectId};
use chrono::Utc;
use futures::TryStreamExt;
use mongodb::{Collection, Database};
use uuid::Uuid;

use crate::{models::{category::{self, Category,  CategoryWithTopUserResponse}, question::{Difficulty, Question}, quiz::Quiz, user::UserResponse}, services::{leaderboard_service::LeaderboardService, user_service::UserService}};



pub struct QuizService {
    quiz_collection: Collection<Quiz>,
    category_collection: Collection<Category>,
    question_collection: Collection<Question>,
    pub leaderboard_service: Arc<LeaderboardService>
}

impl QuizService {
    pub fn new(db: Arc<Database>, leaderboard_service:Arc<LeaderboardService>) -> Self {
        Self {
            quiz_collection: db.collection("quizzes"),
            category_collection: db.collection("categories"),
            question_collection: db.collection("questions"),
            leaderboard_service
        }
    }


    pub async fn get_categories_with_top_users(&self, user_service: &UserService) -> Result<Vec<CategoryWithTopUserResponse>, String> {
        let mut cursor = self.category_collection.find(doc! {}, ).await.map_err(|e| e.to_string())?;
        let mut results = Vec::new();

        while let Some(doc) = cursor.try_next().await.map_err(|e| e.to_string())? {
            let top_user = if let Some(user_id) = doc.top_user_id {
                user_service.get_user(user_id).await.ok()
            } else {
                None
            };

            results.push(CategoryWithTopUserResponse {
                category: category::CategoryResponse::from((doc, None)),
                top_user: top_user.map(|u| u.into()),
            });
        }
        Ok(results)
    }

       pub async fn top_user_for_category(
        &self,
        category_id: ObjectId,
        user_service: &UserService,
    ) -> Result<Option<UserResponse>, String> {
        let category = self.category_collection
            .find_one(doc! { "_id": category_id }, )
            .await
            .map_err(|e| e.to_string())?
            .ok_or("Category not found")?;
        
        if let Some(user_id) = category.top_user_id {
            let user = user_service.get_user(user_id).await?;
            Ok(Some(user.into()))
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

    pub async fn pause_quiz(&self, quiz_id: ObjectId, paused: bool) -> Result<Quiz, String> {
        let update = doc! { "$set": { "paused": paused } };
        self.quiz_collection
            .find_one_and_update(doc! { "_id": quiz_id }, update, )
            .await
            .map_err(|_| "Failed to update quiz".to_string())?
            .ok_or("Quiz not found".to_string())
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

    // Check and update top user for the category
    self.update_category_top_user(quiz.user_id, quiz.category_id, user_service).await?;

    // Update leaderboard
    self.leaderboard_service.update_leaderboard(quiz.user_id, quiz.category_id, quiz.score).await?;

    Ok(quiz.score)
}

   async fn update_category_top_user(
    &self,
    _user_id: ObjectId,
    category_id: ObjectId,
    _user_service: &UserService,
) -> Result<(), String> {
    // 1. Fetch all finished quizzes for the category
    let mut cursor = self.quiz_collection.find(
        doc! { "category_id": category_id, "end_time": { "$exists": true } },
    ).await.map_err(|e| e.to_string())?;

    // 2. Group scores by user
    let mut user_scores: std::collections::HashMap<ObjectId, i32> = std::collections::HashMap::new();
    while let Some(quiz) = cursor.try_next().await.map_err(|e| e.to_string())? {
        *user_scores.entry(quiz.user_id).or_insert(0) += quiz.score;
    }

    // 3. Find the top user
    let top_user = user_scores.into_iter().max_by_key(|&(_, score)| score);

    // 4. Update the category with the top user
    if let Some((top_user_id, _)) = top_user {
        self.category_collection.update_one(
            doc! { "_id": category_id },
            doc! { "$set": { "top_user_id": top_user_id } },
        ).await.map_err(|e| e.to_string())?;
    }

    Ok(())
}
}
