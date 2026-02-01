use crate::api::types::{Delta, DeltaToolCall, FunctionCall, Message, Role, ToolCall};

#[derive(Debug, Default)]
pub struct StreamAccumulator {
    content: String,
    tool_calls: Vec<ToolCallAccumulator>,
}

#[derive(Debug, Default, Clone)]
struct ToolCallAccumulator {
    id: String,
    call_type: String,
    name: String,
    arguments: String,
}

impl StreamAccumulator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn accumulate(&mut self, delta: &Delta) -> Option<String> {
        let mut new_content = None;

        if let Some(ref content) = delta.content {
            self.content.push_str(content);
            new_content = Some(content.clone());
        }

        if let Some(ref tool_calls) = delta.tool_calls {
            for delta_tc in tool_calls {
                self.accumulate_tool_call(delta_tc);
            }
        }

        new_content
    }

    fn accumulate_tool_call(&mut self, delta: &DeltaToolCall) {
        let index = delta.index;

        while self.tool_calls.len() <= index {
            self.tool_calls.push(ToolCallAccumulator::default());
        }

        let tc = &mut self.tool_calls[index];

        if let Some(ref id) = delta.id {
            tc.id = id.clone();
        }

        if let Some(ref call_type) = delta.call_type {
            tc.call_type = call_type.clone();
        }

        if let Some(ref func) = delta.function {
            if let Some(ref name) = func.name {
                tc.name = name.clone();
            }
            if let Some(ref args) = func.arguments {
                tc.arguments.push_str(args);
            }
        }
    }

    pub fn into_message(self) -> Message {
        let tool_calls = if self.tool_calls.is_empty() {
            None
        } else {
            Some(
                self.tool_calls
                    .into_iter()
                    .map(|tc| ToolCall {
                        id: tc.id,
                        call_type: if tc.call_type.is_empty() {
                            "function".to_string()
                        } else {
                            tc.call_type
                        },
                        function: FunctionCall {
                            name: tc.name,
                            arguments: tc.arguments,
                        },
                    })
                    .collect(),
            )
        };

        Message {
            role: Role::Assistant,
            content: if self.content.is_empty() {
                None
            } else {
                Some(self.content)
            },
            tool_calls,
            tool_call_id: None,
        }
    }
}

pub fn parse_sse_line(line: &str) -> Option<String> {
    let line = line.trim();

    if line.is_empty() || line.starts_with(':') {
        return None;
    }

    if let Some(data) = line.strip_prefix("data: ") {
        if data == "[DONE]" {
            return None;
        }
        return Some(data.to_string());
    }

    None
}
