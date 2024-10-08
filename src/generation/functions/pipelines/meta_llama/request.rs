use crate::error::OllamaError;
use crate::generation::chat::{ChatMessage, ChatMessageResponse, MessageKind};
use crate::generation::functions::pipelines::meta_llama::DEFAULT_SYSTEM_TEMPLATE;
use crate::generation::functions::pipelines::RequestParserBase;
use crate::generation::functions::tools::Tool;
use async_trait::async_trait;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

pub fn convert_to_llama_tool(tool: &Arc<dyn Tool>) -> Value {
    let mut function = HashMap::new();
    function.insert("name".to_string(), Value::String(tool.name()));
    function.insert("description".to_string(), Value::String(tool.description()));
    function.insert("parameters".to_string(), tool.parameters());
    json!(format!(
        "Use the function '{name}' to '{description}': {json}",
        name = tool.name(),
        description = tool.description(),
        json = json!(function)
    ))
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LlamaFunctionCallSignature {
    pub function: String, //name of the tool
    pub arguments: Value,
}

pub struct LlamaFunctionCall {}

impl LlamaFunctionCall {
    pub async fn function_call_with_history(
        &self,
        model_name: String,
        tool_params: Value,
        tool: Arc<dyn Tool>,
    ) -> Result<ChatMessageResponse, ChatMessageResponse> {
        let result = tool.run(tool_params).await;
        match result {
            Ok(result) => Ok(ChatMessageResponse {
                model: model_name.clone(),
                created_at: "".to_string(),
                // message: Some(ChatMessage::assistant(result.to_string())),
                message: todo!(),
                done: true,
                final_data: None,
            }),
            Err(e) => Err(self.error_handler(OllamaError::from(e))),
        }
    }

    fn clean_tool_call(&self, json_str: &str) -> String {
        json_str
            .trim()
            .trim_start_matches("```json")
            .trim_end_matches("```")
            .trim()
            .to_string()
            .replace("{{", "{")
            .replace("}}", "}")
    }

    fn parse_tool_response(&self, response: &str) -> Option<Vec<LlamaFunctionCallSignature>> {
        let function_regex = Regex::new(r"<function=(\w+)>(.*?)</function>").unwrap();
        println!("Response: {}", response);

        let mut signatures = Vec::new();

        for caps in function_regex.captures_iter(response) {
            let function_name = caps.get(1).unwrap().as_str().to_string();
            let args_string = caps.get(2).unwrap().as_str();

            match serde_json::from_str(args_string) {
                Ok(arguments) => {
                    signatures.push(LlamaFunctionCallSignature {
                        function: function_name,
                        arguments,
                    });
                }
                Err(error) => {
                    println!("Error parsing function arguments: {}", error);
                    // todo: 
                }
            }
        }

        if signatures.is_empty() {
            None
        } else {
            Some(signatures)
        }
    }
}

#[async_trait]
impl RequestParserBase for LlamaFunctionCall {
    async fn parse(
        &self,
        input: &str,
        model_name: String,
        tools: Vec<Arc<dyn Tool>>,
    ) -> Result<ChatMessageResponse, ChatMessageResponse> {
        let response_value = self.parse_tool_response(&self.clean_tool_call(input));
        match response_value {
            Some(response) => {
                // todo: get the return value from the function call
                Ok(ChatMessageResponse {
                    model: model_name.clone(),
                    created_at: "".to_string(),
                    // message: Some(MessageKind::Parsed(response)),
                    message: todo!(),
                    done: true,
                    final_data: None,
                })
                // if let Some(tool) = tools.iter().find(|t| t.name() == response.function) {
                //     let tool_params = response.arguments;
                //     let result = self
                //         .function_call_with_history(
                //             model_name.clone(),
                //             tool_params.clone(),
                //             tool.clone(),
                //         )
                //         .await?;
                //     return Ok(result);
                // } else {
                //     return Err(self.error_handler(OllamaError::from("Tool not found".to_string())));
                // }
            }
            None => {
                return Err(self
                    .error_handler(OllamaError::from("Error parsing function call".to_string())));
            }
        }
    }

    async fn get_system_message(&self, tools: &[Arc<dyn Tool>]) -> ChatMessage {
        let tools_info: Vec<Value> = tools.iter().map(convert_to_llama_tool).collect();
        let tools_json = serde_json::to_string(&tools_info).unwrap();
        let system_message_content = DEFAULT_SYSTEM_TEMPLATE.replace("{tools}", &tools_json);
        ChatMessage::system(system_message_content)
    }

    fn error_handler(&self, error: OllamaError) -> ChatMessageResponse {
        ChatMessageResponse {
            model: "".to_string(),
            created_at: "".to_string(),
            // message: Some(MessageKind::Unparsed(ChatMessage::assistant(error.to_string()))),
            message: todo!(),
            done: true,
            final_data: None,
        }
    }
}
