use crate::{
    components::Component,
    components::{api_explorer::ApiExplorer, dashboard::Dashboard},
    event_pool::Event,
};
use httpretty::command::Command;

use ratatui::{layout::Rect, Frame};
use tokio::sync::mpsc::UnboundedSender;

pub enum Screens {
    Editor,
    Dashboard,
}

pub struct ScreenManager<'a> {
    cur_screen: Screens,
    editor: Option<ApiExplorer>,
    dashboard: Dashboard<'a>,
    size: Rect,
}

impl<'a> ScreenManager<'a> {
    pub fn new(size: Rect, colors: &'a colors::Colors) -> anyhow::Result<Self> {
        Ok(Self {
            cur_screen: Screens::Dashboard,
            editor: None,
            dashboard: Dashboard::new(size, colors)?,
            size,
        })
    }

    fn switch_screen(&mut self, screen: Screens) {
        self.cur_screen = screen;
    }

    pub fn update(&mut self, event: Option<Event>) -> anyhow::Result<Option<Command>> {
        match self.cur_screen {
            Screens::Editor => self.editor.as_mut().unwrap().handle_event(event),
            Screens::Dashboard => self.dashboard.handle_event(event),
        }
    }

    pub fn handle_command(&mut self, command: Command) {
        match command {
            Command::SelectSchema(schema) | Command::CreateSchema(schema) => {
                self.switch_screen(Screens::Editor);
                self.editor = Some(ApiExplorer::new(self.size, schema));
            }
            Command::Error(msg) => {
                self.dashboard.display_error(msg);
            }
            _ => {}
        }
    }
}

impl Component for ScreenManager<'_> {
    fn draw(&mut self, frame: &mut Frame, _size: Rect) -> anyhow::Result<()> {
        match &self.cur_screen {
            Screens::Editor => self.editor.as_mut().unwrap().draw(frame, frame.size())?,
            Screens::Dashboard => self.dashboard.draw(frame, frame.size())?,
        };

        Ok(())
    }

    fn register_command_handler(&mut self, sender: UnboundedSender<Command>) -> anyhow::Result<()> {
        self.dashboard.register_command_handler(sender.clone())?;
        Ok(())
    }

    fn resize(&mut self, new_size: Rect) {
        self.size = new_size;
        self.dashboard.resize(new_size);

        if let Some(e) = self.editor.as_mut() {
            e.resize(new_size)
        }
    }
}