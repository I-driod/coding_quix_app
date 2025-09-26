use aws_sdk_s3::Client;
use std::error::Error;

pub struct S3Service {
    s3_client: Client,
    bucket_name: String,
}

impl S3Service {
    pub fn new(s3_client: Client, bucket_name: String) -> Self {
        Self { s3_client, bucket_name }
    }

    pub async fn upload_image(&self, key: &str, body: Vec<u8>, content_type: &str) -> Result<String, Box<dyn Error>> {
        self.s3_client
            .put_object()
            .bucket(&self.bucket_name)
            .key(key)
            .body(body.into())
            .content_type(content_type)
            .send()
            .await?;

        let url = format!("https://{}.s3.amazonaws.com/{}", self.bucket_name, key);
        Ok(url)
    }
}
