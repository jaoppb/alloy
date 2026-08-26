use crate::domain::command::RenderCommand;

/// First-class collection wrapping declarative rendering commands (ADR-0010, PRD-005:60-72).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DisplayList {
    commands: Vec<RenderCommand>,
}

impl DisplayList {
    /// Creates an empty display list.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    /// Adds a render command to the display list.
    pub fn push(&mut self, command: RenderCommand) {
        self.commands.push(command);
    }

    /// Returns a slice of all queued render commands.
    #[must_use]
    pub fn commands(&self) -> &[RenderCommand] {
        &self.commands
    }

    /// Returns the number of commands in the display list.
    #[must_use]
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Checks if the display list contains any commands.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Clears all commands from the display list.
    pub fn clear(&mut self) {
        self.commands.clear();
    }

    /// Serializes the display list into a JSON string representation (PRD-005:91, C-18).
    #[must_use]
    pub fn serialize_to_json(&self) -> String {
        let mut out = String::from("[");
        for (i, cmd) in self.commands.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            match cmd {
                RenderCommand::Clear(c) => {
                    out.push_str(&format!(
                        r#"{{"type":"clear","color":"rgba({},{},{},{})"}}"#,
                        c.r(),
                        c.g(),
                        c.b(),
                        c.a()
                    ));
                }
                RenderCommand::DrawRect { rect, color } => {
                    out.push_str(&format!(
                        r#"{{"type":"draw_rect","x":{},"y":{},"w":{},"h":{},"color":"rgba({},{},{},{})"}}"#,
                        rect.x(),
                        rect.y(),
                        rect.width(),
                        rect.height(),
                        color.r(),
                        color.g(),
                        color.b(),
                        color.a()
                    ));
                }
                RenderCommand::DrawBorder { rect, color, width } => {
                    out.push_str(&format!(
                        r#"{{"type":"draw_border","x":{},"y":{},"w":{},"h":{},"width":{},"color":"rgba({},{},{},{})"}}"#,
                        rect.x(),
                        rect.y(),
                        rect.width(),
                        rect.height(),
                        width,
                        color.r(),
                        color.g(),
                        color.b(),
                        color.a()
                    ));
                }
                RenderCommand::DrawText {
                    text,
                    rect,
                    color,
                    font_size,
                } => {
                    out.push_str(&format!(
                        r#"{{"type":"draw_text","text":"{}","x":{},"y":{},"font_size":{},"color":"rgba({},{},{},{})"}}"#,
                        text.replace('"', "\\\""),
                        rect.x(),
                        rect.y(),
                        font_size,
                        color.r(),
                        color.g(),
                        color.b(),
                        color.a()
                    ));
                }
            }
        }
        out.push(']');
        out
    }
}
