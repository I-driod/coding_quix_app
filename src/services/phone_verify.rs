use std::env;

use reqwest::Client;
use serde_json::Value;




#[derive(Clone)]
pub struct TwilioClient{
    client: Client,
    account_sid: String,
    auth_token: String,
    service_sid: String,

}


impl TwilioClient {
    pub fn new_from_env() -> Self {
        let account_sid = env::var("TWILIO_ACCOUNT_SID")
            .expect("TWILIO_ACCOUNT_SID must be set");
        let auth_token = env::var("TWILIO_AUTH_TOKEN")
            .expect("TWILIO_AUTH_TOKEN must be set");
        let service_sid = env::var("TWILIO_VERIFY_SERVICE_SID")
            .expect("TWILIO_VERIFY_SERVICE_SID must be set");

        Self {
            client: Client::new(),
            account_sid,
            auth_token,
            service_sid,
        }
    }

    /// Send verification (OTP) via Twilio Verify (SMS).
    /// `phone_e164` must be a normalized E.164 phone number like "+2348012345678".
    pub async fn send_verification(&self, phone_e164: &str) -> Result<(), String> {
        let url = format!(
            "https://verify.twilio.com/v2/Services/{}/Verifications",
            self.service_sid
        );

        let res = self.client
            .post(&url)
            .basic_auth(&self.account_sid, Some(&self.auth_token))
            .form(&[("To", phone_e164), ("Channel", "sms")])
            .send()
            .await
            .map_err(|e| format!("Twilio send request failed: {}", e))?;

        let status = res.status();
        let body = res.text().await.unwrap_or_else(|_| "<no body>".into());
        

        if status.is_success() {
            Ok(())
        } else {
            Err(format!("Twilio send error {}: {}", status, body))
        }
    }

 

    /// Verify the code using Twilio Verify VerificationCheck.
    // pub async fn check_verification(&self, phone_e164: &str, code: &str) -> Result<bool, String> {
    //     let url = format!(
    //         "https://verify.twilio.com/v2/Services/{}/VerificationCheck",
    //         self.service_sid
    //     );
        

    //     let res = self.client
    //         .post(&url)
    //         .basic_auth(&self.account_sid, Some(&self.auth_token))
    //         .form(&[("To", phone_e164), ("Code", code)])
    //         .send()
    //         .await
    //         .map_err(|e| format!("Twilio check request failed: {}", e))?;

    //     let status = res.status();
    //     let body = res.text().await.unwrap_or_else(|_| "<no body>".into());

    //     if !status.is_success() {
    //         return Err(format!("Twilio check error {}: {}", status, body));
    //     }

    //     // Twilio returns "status" (approved) and "valid" boolean
    //     let v: Value = serde_json::from_str(&body)
    //         .map_err(|e| format!("parse json error: {}", e))?;

    //     if v.get("status").and_then(|s| s.as_str()) == Some("approved") {
    //         return Ok(true);
    //     }
    //     if v.get("valid").and_then(|b| b.as_bool()) == Some(true) {
    //         return Ok(true);
    //     }
    //     Ok(false)
    // }

    pub async fn check_verification(&self, phone_e164: &str, code: &str) -> Result<bool, String> {
    println!("Checking SID={} To={} Code={}", self.service_sid, phone_e164, code);

    let url = format!(
        "https://verify.twilio.com/v2/Services/{}/VerificationCheck",
        self.service_sid
    );

    let res = self.client
        .post(&url)
        .basic_auth(&self.account_sid, Some(&self.auth_token))
        .form(&[("To", phone_e164), ("Code", code)])
        .send()
        .await
        .map_err(|e| format!("Twilio check request failed: {}", e))?;
    
    let status = res.status();
    let body = res.text().await.unwrap_or_else(|_| "<no body>".into());

    if !status.is_success() {
        return Err(format!("Twilio check error {}: {}", status, body));
    }

    let v: Value = serde_json::from_str(&body)
        .map_err(|e| format!("parse json error: {}", e))?;

    if v.get("status").and_then(|s| s.as_str()) == Some("approved") {
        return Ok(true);
    }
    if v.get("valid").and_then(|b| b.as_bool()) == Some(true) {
        return Ok(true);
    }
    Ok(false)
}

}
