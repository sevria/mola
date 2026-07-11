use anyhow::Result;
use mola::{
    Router, handler,
    openapi::{InfoBuilder, OpenApiBuilder},
};
use serde_json::{Value, json};
use sevria_core::init_env;

#[mola::endpoint(
    path = "/hello",
    method = "get",
    tag = "Greetings",
    description = "Returns a friendly hello-world greeting",
    response_desc = "A JSON object with a greeting message"
)]
async fn hello() -> Value {
    json!({ "message": "Hello, world!" })
}

#[tokio::main]
async fn main() -> Result<()> {
    let env = init_env()?;

    let openapi = OpenApiBuilder::new()
        .info(
            InfoBuilder::new()
                .title("Hello World")
                .version(&env.service_version),
        )
        .build();

    let router = Router::new().with_openapi(openapi).add(handler!(hello));

    router.run("127.0.0.1:3000").await?;

    Ok(())
}
