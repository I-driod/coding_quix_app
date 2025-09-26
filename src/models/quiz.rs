use bson::oid::ObjectId;
use chrono::Duration;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use chrono::DateTime;
use chrono::Utc;

use crate::models::question::Difficulty;

#[derive(Serialize, Deserialize, Clone)]
pub struct Quiz {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub uuid: Uuid, 
    pub user_id: ObjectId,
    pub category_id: ObjectId,
    pub difficulty: Difficulty,
    pub questions: Vec<ObjectId>, 
    pub answers: Vec<UserAnswer>,  
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub score: i32,
    pub paused: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UserAnswer {
    pub question_id: ObjectId,
    pub answer: String,
    #[serde(with = "duration_as_secs")]
    pub time_taken: Duration,
    pub correct: bool,
}

mod duration_as_secs {
    use chrono::Duration;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(dur: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i64(dur.num_seconds())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = i64::deserialize(deserializer)?;
        Ok(Duration::seconds(secs))
    }
}





#[derive(Serialize, ToSchema)]
pub struct UserAnswerResponse {
    pub question_id: String, // hex string
    pub answer: String,
    pub time_taken_secs: i64,
    pub correct: bool,
}



#[derive(Serialize, ToSchema)]
pub struct QuizResponse {
    pub id: Option<String>,
    pub uuid: String,
    pub user_id: String,
    pub category_id: String,
    pub difficulty: Difficulty,
    pub questions: Vec<String>,
    pub answers: Vec<UserAnswerResponse>,
    pub start_time: String,          // RFC3339 string
    pub end_time: Option<String>,    // RFC3339 string or null
    pub score: i32,
    pub paused: bool,
}



// ----- Conversions from DB models to DTOs -----
impl From<UserAnswer> for UserAnswerResponse {
    fn from(a: UserAnswer) -> Self {
        UserAnswerResponse {
            question_id: a.question_id.to_hex(),
            answer: a.answer,
            time_taken_secs: a.time_taken.num_seconds(),
            correct: a.correct,
        }
    }
}

impl From<Quiz> for QuizResponse {
    fn from(q: Quiz) -> Self {
        QuizResponse {
            id: q.id.map(|oid| oid.to_hex()),
            uuid: q.uuid.to_string(),
            user_id: q.user_id.to_hex(),
            category_id: q.category_id.to_hex(),
            difficulty: q.difficulty,
            questions: q.questions.into_iter().map(|oid| oid.to_hex()).collect(),
            answers: q.answers.into_iter().map(|a| a.into()).collect(),
            start_time: q.start_time.to_rfc3339(),
            end_time: q.end_time.map(|d| d.to_rfc3339()),
            score: q.score,
            paused: q.paused,
        }
    }
}