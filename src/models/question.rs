use bson::oid::ObjectId;
use chrono::Duration;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// DB model (internal, used for Mongo)
#[derive(Serialize, Deserialize, Clone)]
pub struct Question {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub category_id: ObjectId,
    pub question: String,
    pub question_type: QuestionType,
    pub options: Vec<String>,
    pub correct_answer: String,
    pub explanation: String,
    pub difficulty: Difficulty,
    #[serde(with = "duration_as_secs")]
    pub timer: Duration,
    pub tags: Vec<String>,
}

/// Question type enum (safe for OpenAPI)
#[derive(Serialize, Deserialize, Clone, ToSchema)]
pub enum QuestionType {
    MultipleChoice,
    TrueFalse,
    CodePrediction,
}

/// Difficulty enum (safe for OpenAPI)
#[derive(Serialize, Deserialize, Clone, ToSchema)]
pub enum Difficulty {
    Beginner,
    Intermediate,
    Advanced,
    Expert,
}

/// Request DTO (client → server)
#[derive(Deserialize, ToSchema)]
pub struct CreateQuestionRequest {
    pub category_id: String, // hex string of ObjectId
    pub text: String,
    pub question_type: QuestionType,
    pub options: Vec<String>,
    pub correct_answer: String,
    pub explanation: String,
    pub difficulty: Difficulty,
    pub timer_secs: i64,
    pub tags: Vec<String>,
}

/// Response DTO (server → client)
#[derive(Serialize, ToSchema, Deserialize)]
pub struct QuestionResponse {
    pub id: Option<String>,
    pub category_id: String,
    pub text: String,
    pub question_type: QuestionType,
    pub options: Vec<String>,
    pub correct_answer: String,
    pub explanation: String,
    pub difficulty: Difficulty,
    pub timer_secs: i64,
    pub tags: Vec<String>,
}

impl From<Question> for QuestionResponse {
    fn from(q: Question) -> Self {
        QuestionResponse {
            id: q.id.map(|oid| oid.to_hex()),
            category_id: q.category_id.to_hex(),
            text: q.question,
            question_type: q.question_type,
            options: q.options,
            correct_answer: q.correct_answer,
            explanation: q.explanation,
            difficulty: q.difficulty,
            timer_secs: q.timer.num_seconds(),
            tags: q.tags,
        }
    }
}

impl TryFrom<CreateQuestionRequest> for Question {
    type Error = String;

    fn try_from(req: CreateQuestionRequest) -> Result<Self, Self::Error> {
        let category_oid = ObjectId::parse_str(&req.category_id)
            .map_err(|_| "Invalid category_id ObjectId".to_string())?;

        Ok(Question {
            id: None,
            category_id: category_oid,
            question: req.text,
            question_type: req.question_type,
            options: req.options,
            correct_answer: req.correct_answer,
            explanation: req.explanation,
            difficulty: req.difficulty,
            timer: Duration::seconds(req.timer_secs),
            tags: req.tags,
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct CreateQuestionResponse {
    pub message: String,
    pub question: QuestionResponse,
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

