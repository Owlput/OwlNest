use surrealdb::{Surreal, engine::local::Mem};

pub async fn database_init()->surrealdb::Result<()>{
    let db = Surreal::new::<Mem>(()).await?;
    db.use_ns("owlnest").use_db("test").await
}