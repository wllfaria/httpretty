use crate::components::Component;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

#[derive(Debug)]
struct ReqBuilderLayout {
    method_selector: Rect,
    _url_input: Rect,
    request_button: Rect,
}

#[derive(Debug)]
pub struct ReqBuilder {
    layout: ReqBuilderLayout,
}

impl ReqBuilder {
    pub fn new(size: Rect) -> Self {
        Self {
            layout: build_layout(size),
        }
    }
}

impl Component for ReqBuilder {
    fn draw(&mut self, frame: &mut Frame, _area: Rect) -> anyhow::Result<()> {
        let b = Paragraph::new("lol").block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(Style::default().gray().dim()),
        );

        frame.render_widget(b.clone(), self.layout.method_selector);
        frame.render_widget(b, self.layout.request_button);

        Ok(())
    }

    fn resize(&mut self, new_size: Rect) {
        self.layout = build_layout(new_size);
    }
}

fn build_layout(size: Rect) -> ReqBuilderLayout {
    let [method_selector, url_input, request_button] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(10),
            Constraint::Fill(1),
            Constraint::Length(10),
        ])
        .areas(size);

    ReqBuilderLayout {
        method_selector,
        _url_input: url_input,
        request_button,
    }
}