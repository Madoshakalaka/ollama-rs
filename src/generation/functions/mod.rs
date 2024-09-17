pub mod pipelines;
pub mod request;
pub mod tools;

pub use crate::generation::functions::pipelines::meta_llama::request::LlamaFunctionCall;
pub use crate::generation::functions::pipelines::nous_hermes::request::NousFunctionCall;
pub use crate::generation::functions::pipelines::openai::request::OpenAIFunctionCall;
pub use crate::generation::functions::request::FunctionCallRequest;
use pipelines::meta_llama::request::LlamaFunctionCallSignature;
use regex::Regex;
pub use tools::Browserless;
pub use tools::DDGSearcher;
pub use tools::Scraper;
pub use tools::SerperSearchTool;
pub use tools::StockScraper;

use crate::error::OllamaError;
use crate::generation::chat::request::ChatMessageRequest;
use crate::generation::chat::{ChatMessage, ChatMessageResponse};
use crate::generation::functions::pipelines::RequestParserBase;
use crate::generation::functions::tools::Tool;
use std::sync::Arc;

#[cfg(feature = "function-calling")]
impl crate::Ollama {
    fn has_system_prompt(&self, messages: &[ChatMessage], system_prompt: &str) -> bool {
        let system_message = messages.first().unwrap().clone();
        system_message.content == system_prompt
    }

    fn has_system_prompt_history(&mut self) -> bool {
        self.get_messages_history("default").is_some()
    }

    #[cfg(feature = "chat-history")]
    pub async fn send_function_call_with_history(
        &mut self,
        request: FunctionCallRequest,
        parser: Arc<dyn RequestParserBase>,
        id: String,
    ) -> Result<ChatMessageResponse, OllamaError> {
        let mut request = request;

        if !self.has_system_prompt_history() {
            let system_prompt = parser.get_system_message(&request.tools).await;
            self.set_system_response(id.clone(), system_prompt.content);

            //format input
            let formatted_query = ChatMessage::user(
                parser.format_query(&request.chat.messages.first().unwrap().content),
            );
            //replace with formatted_query with previous chat_message
            request.chat.messages.remove(0);
            request.chat.messages.insert(0, formatted_query);
        }

        let tool_call_result = self
            .send_chat_messages_with_history(
                ChatMessageRequest::new(request.chat.model_name.clone(), request.chat.messages),
                id.clone(),
            )
            .await?;

        let tool_call_content: String = tool_call_result.message.clone().unwrap().content;
        let result = parser
            .parse(
                &tool_call_content,
                request.chat.model_name.clone(),
                request.tools,
            )
            .await;

        todo!("need to read about the ollama history API")
        // match result {
        //     Ok(r) => {
        //         self.add_assistant_response(id.clone(), r.message.clone().unwrap().content);
        //         Ok(r)
        //     }
        //     Err(e) => {
        //         self.add_assistant_response(id.clone(), e.message.clone().unwrap().content);
        //         Err(OllamaError::from(e.message.unwrap().content))
        //     }
        // }
    }

    pub async fn send_function_call(
        &self,
        request: FunctionCallRequest,
        parser: Arc<dyn RequestParserBase>,
    ) -> Result<Vec<LlamaFunctionCallSignature>, OllamaError> {
        let mut request = request;

        request.chat.stream = false;
        let system_prompt = parser.get_system_message(&request.tools).await;
        let model_name = request.chat.model_name.clone();

        //Make sure the first message in chat is the system prompt
        if !self.has_system_prompt(&request.chat.messages, &system_prompt.content) {
            request.chat.messages.insert(0, system_prompt);
        }
        let result = self.send_chat_messages(request.chat).await?;
        let response_content: String = result.message.clone().unwrap().content;

        // let result = parser
        //     .parse(&response_content, model_name, request.tools)
        //     .await;
        let result = parse_llama_function(&response_content, model_name, request.tools);
        Ok(result)

        // match result {
        //     Ok(r) => Ok(r),
        //     // Err(e) => Err(OllamaError::from(e.message.unwrap().content)),
        //     Err(e) => Err(OllamaError::from("parse error happened".to_string())),
        // }
    }
}

fn parse_llama_tool_response(response: &str) -> Option<Vec<LlamaFunctionCallSignature>> {
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

fn parse_llama_function(
    input: &str,
    model_name: String,
    tools: Vec<Arc<dyn Tool>>,
) -> Vec<LlamaFunctionCallSignature> {
    // let response_value = self.parse_tool_response(&self.clean_tool_call(input));
    let response_value = parse_llama_tool_response(input);
    match response_value {
        Some(response) => {
            // todo: get the return value from the function call
            response
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
            panic!("Error parsing function call");
            // return Err(
            //     self.error_handler(OllamaError::from("Error parsing function call".to_string()))
            // );
        }
    }
}
