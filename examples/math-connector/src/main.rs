use camunda_connector_rs::{camunda_connector, connector_main};
use serde::{Deserialize, Serialize};

connector_main!(port = 8080);

#[derive(Deserialize, Debug)]
pub struct MyInput {
    pub a: u64,
    pub b: u64,
}

#[derive(Serialize, Debug)]
pub struct MyOutput {
    pub result: u64,
}

#[camunda_connector(name = "math", operation="add")]
pub async fn add(id : u64 , params: MyInput) -> Result<MyOutput, String> {
    println!("[add] id: {}, params: {:?}", id, params);
    Ok(MyOutput{
        result: params.a + params.b,
    })
}

#[camunda_connector(name = "math", operation="sub")]
pub async fn sub(id : u64, params: MyInput) -> Result<MyOutput, String> {
    println!("[sub] id: {}, params: {:?}", id, params);
    Ok(MyOutput{
        result: params.a - params.b,
    })
}