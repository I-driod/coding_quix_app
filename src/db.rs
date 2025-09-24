use mongodb::{Client, Database};


pub async fn init_db(uri: &str) -> Database {
    let client = Client::with_uri_str(uri).await.expect("Failed to connect to MongoDB");
    client.database("quiz_db")
}