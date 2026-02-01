use crate::api::types::{Message, Role};

pub struct Conversation {
    system_prompt: String,
    messages: Vec<Message>,
}

impl Conversation {
    pub fn new(system_prompt: String) -> Self {
        Self {
            system_prompt,
            messages: Vec::new(),
        }
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub fn get_messages(&self) -> Vec<Message> {
        let mut result = vec![Message {
            role: Role::System,
            content: Some(self.system_prompt.clone()),
            tool_calls: None,
            tool_call_id: None,
        }];
        result.extend(self.messages.clone());
        result
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}
