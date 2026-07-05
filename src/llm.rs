use std::fmt::Display;

use futures::StreamExt;
use genai::{
    Client, adapter::AdapterKind, chat::{ChatMessage, ChatOptions, ChatRequest, ChatStreamEvent}, resolver::{AuthData, AuthResolver}
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{app_data::config::ApiKeys, model::{MessageKind, Model}};

pub enum Role {
    System,
    User,
    Assistant,
}

pub struct Message {
    pub content: String,
    pub role: Role,
}

impl Message {
    pub fn system(text: &impl Display) -> Self {
        Self {
            content: text.to_string(),
            role: Role::System,
        }
    }

    pub fn assistant(text: &impl Display) -> Self {
        Self {
            content: text.to_string(),
            role: Role::Assistant,
        }
    }

    pub fn user(text: &impl Display) -> Self {
        Self {
            content: text.to_string(),
            role: Role::User,
        }
    }
}

impl From<&Message> for ChatMessage {
    fn from(msg: &Message) -> Self {
        match msg.role {
            Role::System => {
                ChatMessage::system(msg.content.clone())
            },
            Role::User => {
                ChatMessage::user(msg.content.clone())
            },
            Role::Assistant => {
                ChatMessage::assistant(msg.content.clone())
            },
        }
    }
}

pub struct Request {
    pub model: String,
    pub temp: Option<f32>,
    pub top_p: Option<f32>,
    pub top_k: Option<u32>,
    pub min_p: Option<f32>,

    pub messages: Vec<Message>,
}

pub fn build_context(model: &Model) -> Vec<Message> {
    let mut messages: Vec<Message> = Vec::new();

    for message in model.content_messages() {
        match message.kind {
            MessageKind::Response => {
                messages.push(Message::assistant(&message.content));
            },
            MessageKind::User => {
                messages.push(Message::user(&message.content));
            },
            _ => continue,
        }
    }

    messages
}

pub enum Event {
    Token(String),
    ThinkingToken(String),
    Done,
    Error(String),
}

#[derive(Clone)]
pub struct StreamingEndpoint {
    pub client: Client,
    pub tx: UnboundedSender<Event>,
}

impl StreamingEndpoint {
    pub fn new(tx: UnboundedSender<Event>) -> Self {
        Self {
            client: Client::default(),
            tx 
        }
    }

    pub fn with_api_keys(self, keys: &ApiKeys) -> Self {
        let keys = keys.clone();
        let auth_resolver = AuthResolver::from_resolver_fn(move |iden: genai::ModelIden| {
            let key = match iden.adapter_kind {
                AdapterKind::Zai => keys.zai.clone(),
                AdapterKind::Gemini => keys.gemini.clone(),
                _ => None,
            };

            Ok(key.map(AuthData::from_single))
        });

        let client = genai::Client::builder()
            .with_auth_resolver(auth_resolver)
            .build();

        Self {
            client,
            ..self
        }
    }

    pub async fn stream_request(&mut self, request: &Request) {
        let (chat_req, options) = to_genai_request(request);

        let res = match self.client
            .exec_chat_stream(&request.model, chat_req, Some(&options))
            .await
        {
            Ok(res) => res,
            Err(e) => {
                let _ = self.tx.send(Event::Error(e.to_string()));
                return;
            }
        };

        let mut stream = res.stream;
        loop {
            match stream.next().await {
                Some(Ok(ChatStreamEvent::Chunk(chunk))) => {
                    if self.tx.send(Event::Token(chunk.content)).is_err() {
                        return; // the UI hung, drop the stream
                    }
                },
                Some(Ok(ChatStreamEvent::ThoughtSignatureChunk(chunk))) => {
                    if self.tx.send(Event::ThinkingToken(chunk.content)).is_err() {
                        return;
                    }
                },
                Some(Ok(ChatStreamEvent::ReasoningChunk(chunk))) => {
                    if self.tx.send(Event::ThinkingToken(chunk.content)).is_err() {
                        return;
                    }
                }
                Some(Ok(ChatStreamEvent::End(_))) => {
                    break; // done
                },
                Some(Ok(_)) => {
                    // Start, ReasoningRequest
                    // ToolCallChunk todo
                },
                Some(Err(e)) => {
                    let _ = self.tx.send(Event::Error(e.to_string()));
                    return;
                },
                None => break,
            }
        }

        let _ = self.tx.send(Event::Done);
    }
}

fn to_genai_request(req: &Request) -> (ChatRequest, ChatOptions) {
    let messages: Vec<ChatMessage> = req
        .messages
        .iter()
        .map(|msg| msg.into())
        .collect();

    let mut options = ChatOptions::default();
    if let Some(temp) = req.temp {
        options = options.with_temperature(temp as f64);
    }
    if let Some(tp) = req.top_p {
        options = options.with_top_p(tp as f64)
    }

    (ChatRequest::new(messages), options)
}
