
use bson::oid::ObjectId;
use mongodb::{Collection, Database};
use bson::doc;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::models::user::{Profile, Role, User, UserResponse};
use crate::services::phone_verify::TwilioClient;
use crate::utils::{hash_password, verify_password, generate_jwt};
use std::sync::Arc;
use std::fmt;
use serde::{Deserialize, Serialize};


#[derive(Serialize, ToSchema, Deserialize)]
pub struct LoginResponse {
    pub message: String,
    pub token: Option<String>,
    pub user: Option<UserResponse>,
}



pub struct UserService{
    collection: Collection<User>,
    twilio: Arc<TwilioClient>
}




impl UserService {
    pub fn new(db: Arc<Database>, tilio: Arc<TwilioClient>) -> Self{
        Self{
            collection: db.collection("users"),
            twilio: tilio
        }
    }


        /// Verify OTP and register user
    pub async fn verify_and_register(
        &self,
        phone_number: String,
        code: String,
        username: String,
        password: String,
        role: Role,
    ) -> Result<String, String> {
        // check OTP with Twilio
        let ok = self.twilio.check_verification(&phone_number, &code).await?;
        if !ok {
            return Err("Invalid OTP".into());
        }

        // uniqueness checks
        if self.collection.find_one(doc! { "phone_number": &phone_number }, ).await.map_err(|e| format!("DB error: {}", e))
?.is_some() {
            return Err("Phone Number already in use".into());
        }
        if self.collection.find_one(doc! { "username": &username }, ).await.map_err(|e| format!("DB error: {}", e))?.is_some() {
            return Err("Username already in use".into());
        }

        let password_hash = hash_password(&password);
        let new_user = User {
            id: None,
            phone_number,
            username,
            password_hash,
            role,
            profile: Profile {
                avatar: None,
                bio: None,
                preferred_language: None,
                country: None,
            },
            xp: 0,
            quiz_history: vec![],
        };

        let insert_result = self.collection.insert_one(new_user, ).await.map_err(|_| "Failed to register user".to_string())?;
        Ok(insert_result.inserted_id.as_object_id().unwrap().to_hex())
    }

        /// Start verification: send OTP
    pub async fn start_phone_verification(&self, phone_number: String) -> Result<(), String> {
        // phone_number must be E.164 (validate/normalize before calling)
        self.twilio.send_verification(&phone_number).await
    }


// pub async fn login(
//     &self,
//     phone_number: String,
//     password: String,
//     jwt_secret: &str,
// ) -> Result<(UserResponse, String), String> {
//     let user = self
//         .collection
//         .find_one(doc! { "phone_number": phone_number })
//         .await
//         .unwrap()
//         .ok_or("User not found")?;

//     if !verify_password(&user.password_hash, &password) {
//         return Err("Invalid password".to_string());
//     }

//     let user_response: UserResponse = user.clone().into();
//     let token = generate_jwt(
//         &user.id.as_ref().unwrap().to_hex(),
//         &user.role.to_string(),
//         jwt_secret,
//     );

//     Ok((user_response, token))
// }

pub async fn login(
    &self,
    phone_number: String,
    password: String,
    jwt_secret: &str,
) -> Result<(UserResponse, String), String> {
    // return DB-level errors as Err(String) instead of panicking
    let find_result = self
        .collection
        .find_one(doc! { "phone_number": &phone_number }, )
        .await
        .map_err(|e| format!("DB query error: {}", e))?;



let user = find_result.ok_or_else(|| "User not found".to_string())?;

if !verify_password(&user.password_hash, &password) {
    return Err("Invalid password".to_string());
}

// ensure id exists and capture role as owned Strings before consuming or cloning user
let user_id = match user.id.as_ref() {
    Some(oid) => oid.to_hex(),
    None => return Err("User record missing id".to_string()),
};
let role = user.role.to_string();

// clone user for conversion (avoids moving the original `user`)
let user_response: UserResponse = user.clone().into();

let token = generate_jwt(&user_id, &role, jwt_secret);

Ok((user_response, token))

}


    pub async fn update_profile(&self, user_id: ObjectId, profile: Profile) -> Result<(), String> {
        let update_result = self.collection.update_one(
            doc! { "_id": user_id },
            doc! { "$set": { "profile": bson::to_bson(&profile).unwrap() } }
        ).await.map_err(|_| "Failed to update profile".to_string())?;

        if update_result.matched_count == 0 {
            return Err("User not found".to_string());
        }

        Ok(())
    }

    pub async fn get_user(&self, user_id: ObjectId) -> Result<User, String> {
        self.collection
            .find_one(doc! { "_id": user_id })
            .await
            .map_err(|e| format!("DB query failed in get_user: {}", e))?
            .ok_or_else(|| format!("User with ID {} not found", user_id))
    }

    pub async fn add_xp(&self, user_id: ObjectId, xp: i32) -> Result<(), String> {
        let update_result = self.collection.update_one(
            doc! { "_id": user_id },
            doc! { "$inc": { "xp": xp } }
        ).await.map_err(|_| "Failed to add XP".to_string())?;

        if update_result.matched_count == 0 {
            return Err("User not found".to_string());
        }

        Ok(())
    }

    pub async fn add_quiz_history(&self, user_id: ObjectId, quiz_uuid: Uuid) -> Result<(), String> {
        // Serialize the Uuid to its binary representation for MongoDB
        let quiz_uuid_bson = bson::to_bson(&quiz_uuid).map_err(|_| "Failed to serialize quiz Uuid".to_string())?;

        let update_result = self.collection.update_one(
            doc! { "_id": user_id },
            doc! { "$push": { "quiz_history": quiz_uuid_bson } }
        ).await.map_err(|_| "Failed to add quiz history".to_string())?;

        if update_result.matched_count == 0 {
            return Err("User not found".to_string());
        }

        Ok(())
    }

}


impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::User => write!(f, "User"),
            Role::Admin => write!(f, "Admin"),
        }
    }
}
