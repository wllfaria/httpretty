use crate::components::{
    confirm_popup::ConfirmPopup,
    dashboard::{
        new_collection_form::{FormFocus, FormState, NewCollectionForm},
        schema_list::{SchemaList, SchemaListState},
    },
    error_popup::ErrorPopup,
    Component,
};
use httpretty::{
    command::Command,
    schema::{schema, types::Schema},
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Clear, Padding, Paragraph, StatefulWidget, Widget, Wrap},
    Frame,
};
use std::ops::Not;
use tokio::sync::mpsc::UnboundedSender;
use tui_big_text::{BigText, PixelSize};

#[derive(Debug)]
struct DashboardLayout {
    schemas_pane: Rect,
    help_pane: Rect,
    help_popup: Rect,
    title_pane: Rect,
    confirm_popup: Rect,
    form_popup: Rect,
    error_popup: Rect,
}

#[derive(Debug)]
pub struct Dashboard<'a> {
    layout: DashboardLayout,
    schemas: Vec<Schema>,
    list: SchemaList<'a>,
    list_state: SchemaListState,
    form_state: FormState,
    colors: &'a colors::Colors,
    show_list_keymaps: bool,
    show_filter: bool,
    filter: String,
    pane_focus: PaneFocus,
    prompt_delete_current: bool,
    sender: Option<UnboundedSender<Command>>,
    show_error_popup: bool,
    error_message: String,
}

#[derive(Debug, PartialEq, Eq)]
enum PaneFocus {
    List,
    Form,
}

impl<'a> Dashboard<'a> {
    pub fn new(size: Rect, colors: &'a colors::Colors) -> anyhow::Result<Self> {
        let mut schemas = schema::get_schemas()?;
        schemas.sort_by_key(|k| k.info.name.clone());
        let mut list_state = SchemaListState::new(schemas.clone());
        schemas.is_empty().not().then(|| list_state.select(Some(0)));

        Ok(Dashboard {
            list_state,
            form_state: FormState::default(),
            colors,
            layout: build_layout(size),
            schemas,
            list: SchemaList::new(colors),
            show_list_keymaps: false,
            filter: String::new(),
            show_filter: false,
            pane_focus: PaneFocus::List,
            prompt_delete_current: false,
            sender: None,
            show_error_popup: false,
            error_message: String::default(),
        })
    }

    pub fn display_error(&mut self, message: String) {
        self.show_error_popup = true;
        self.error_message = message;
        self.prompt_delete_current = false;
        self.pane_focus = PaneFocus::List;
    }

    fn filter_list(&mut self) {
        self.list_state.set_items(
            self.schemas
                .clone()
                .into_iter()
                .filter(|s| s.info.name.contains(&self.filter))
                .collect(),
        );
    }

    fn handle_filter_key_event(&mut self, key_event: KeyEvent) {
        match (key_event.code, key_event.modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) | (KeyCode::Esc, _) => {
                self.show_filter = false;
                self.filter = String::new();
                self.filter_list();
            }
            (KeyCode::Backspace, _) => {
                if self.filter.is_empty() {
                    self.show_filter = false;
                }
                self.filter.pop();
                self.filter_list();
            }
            (KeyCode::Enter, _) => {
                self.show_filter = false;
                self.filter_list();
            }
            (KeyCode::Char(c), _) => {
                self.filter.push(c);
                self.filter_list();
            }
            _ => {}
        }
    }

    fn handle_list_key_event(&mut self, key_event: KeyEvent) -> anyhow::Result<Option<Command>> {
        match key_event.code {
            KeyCode::Enter => {
                return Ok(self
                    .schemas
                    .is_empty()
                    .not()
                    .then(|| {
                        self.list_state
                            .selected()
                            .and_then(|i| self.schemas.get(i))
                            .expect("user should never be allowed to select a non existing schema")
                    })
                    .map(|schema| Command::SelectSchema(schema.clone())));
            }
            KeyCode::Char('d') => self.prompt_delete_current = true,
            KeyCode::Char('n') | KeyCode::Char('c') => {
                self.pane_focus = PaneFocus::Form;
            }
            KeyCode::Char('h') => self.list_state.select(
                self.list_state
                    .selected()
                    .map(|i| i.saturating_sub(1))
                    .or(Some(0)),
            ),
            KeyCode::Char('j') => self.list_state.select(
                self.list_state
                    .selected()
                    .map(|i| {
                        usize::min(
                            self.schemas.len() - 1,
                            i + self.list.items_per_row(&self.layout.schemas_pane),
                        )
                    })
                    .or(Some(0)),
            ),
            KeyCode::Char('k') => self.list_state.select(
                self.list_state
                    .selected()
                    .map(|i| i.saturating_sub(self.list.items_per_row(&self.layout.schemas_pane)))
                    .or(Some(0)),
            ),
            KeyCode::Char('l') => self.list_state.select(
                self.list_state
                    .selected()
                    .map(|i| usize::min(self.schemas.len() - 1, i + 1))
                    .or(Some(0)),
            ),
            KeyCode::Char('?') => self.show_list_keymaps = true,
            KeyCode::Char('/') => self.show_filter = true,
            KeyCode::Char('q') => return Ok(Some(Command::Quit)),
            _ => {}
        };
        Ok(None)
    }

    fn handle_form_key_event(&mut self, key_event: KeyEvent) -> anyhow::Result<Option<Command>> {
        match (key_event.code, key_event.modifiers) {
            (KeyCode::Tab, _) => match self.form_state.focused_field {
                FormFocus::Name => self.form_state.focused_field = FormFocus::Description,
                FormFocus::Description => self.form_state.focused_field = FormFocus::Confirm,
                FormFocus::Confirm => self.form_state.focused_field = FormFocus::Cancel,
                FormFocus::Cancel => self.form_state.focused_field = FormFocus::Name,
            },
            (KeyCode::Char(c), _) => match self.form_state.focused_field {
                FormFocus::Name => self.form_state.name.push(c),
                FormFocus::Description => self.form_state.description.push(c),
                _ => {}
            },
            (KeyCode::Enter, _) => match self.form_state.focused_field {
                FormFocus::Confirm => {
                    let name = self.form_state.name.clone();
                    let description = self.form_state.description.clone();

                    let sender_copy = self
                        .sender
                        .clone()
                        .expect("should always have a sender at this point");

                    tokio::spawn(async move {
                        match httpretty::fs::create_schema(name, description).await {
                            Ok(schema) => {
                                if sender_copy.send(Command::CreateSchema(schema)).is_err() {
                                    tracing::error!("failed to send command through channel");
                                    std::process::abort();
                                }
                            }
                            Err(e) => {
                                if sender_copy.send(Command::Error(e.to_string())).is_err() {
                                    tracing::error!("failed to send error command through channel");
                                    std::process::abort();
                                }
                            }
                        }
                    });
                }
                FormFocus::Cancel => {
                    self.pane_focus = PaneFocus::List;
                    self.form_state.reset();
                }
                _ => {}
            },
            (KeyCode::Backspace, _) => match self.form_state.focused_field {
                FormFocus::Name => {
                    self.form_state.name.pop();
                }
                FormFocus::Description => {
                    self.form_state.description.pop();
                }
                _ => {}
            },
            _ => {}
        }
        Ok(None)
    }

    #[tracing::instrument(skip_all)]
    fn handle_confirm_popup_key_event(&mut self, key_event: KeyEvent) -> anyhow::Result<()> {
        match key_event.code {
            KeyCode::Char('y') => {
                let selected = self
                    .list_state
                    .selected()
                    .expect("deleting when nothing is selected should never happen");
                let schema = self
                    .schemas
                    .get(selected)
                    .expect("should never attempt to delete a non existing item");
                let path = schema.path.clone();

                tokio::spawn(async move {
                    tracing::debug!("attempting to delete schema: {:?}", path);
                    httpretty::fs::delete_schema(&path)
                        .await
                        .expect("failed to delete schema from filesystem");
                });

                self.schemas.remove(selected);
                self.list_state.set_items(self.schemas.clone());
                self.list_state.select(None);
                self.prompt_delete_current = false;
            }
            KeyCode::Char('n') => {
                self.prompt_delete_current = false;
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_error_popup_key_event(&mut self, key_event: KeyEvent) -> anyhow::Result<()> {
        match key_event.code {
            KeyCode::Char('o') | KeyCode::Esc | KeyCode::Enter => {
                self.show_error_popup = false;
            }
            _ => {}
        };

        Ok(())
    }

    fn build_hint_text(&self) -> Line<'static> {
        "[j/k -> up/down] [n -> new] [enter -> select item] [? -> help] [q -> quit]"
            .fg(self.colors.bright.black)
            .to_centered_line()
    }

    fn build_help_popup(&self) -> Paragraph<'_> {
        let lines = vec![
            Line::from(vec![
                "h/<left>".fg(self.colors.normal.magenta),
                "    - select left item".into(),
            ]),
            Line::from(vec![
                "j/<down>".fg(self.colors.normal.magenta),
                "    - select item below".into(),
            ]),
            Line::from(vec![
                "k/<up>".fg(self.colors.normal.magenta),
                "      - select item above".into(),
            ]),
            Line::from(vec![
                "l/<right>".fg(self.colors.normal.magenta),
                "   - select right item".into(),
            ]),
            Line::from(vec![
                "n/c".fg(self.colors.normal.magenta),
                "         - creates a new collection".into(),
            ]),
            Line::from(vec![
                "d".fg(self.colors.normal.magenta),
                "           - deletes the selected collection".into(),
            ]),
            Line::from(vec![
                "?".fg(self.colors.normal.magenta),
                "           - toggle this help window".into(),
            ]),
            Line::from(vec![
                "enter".fg(self.colors.normal.magenta),
                "       - select item under cursor".into(),
            ]),
            Line::from(vec![
                "/".fg(self.colors.normal.magenta),
                "           - enter filter mode".into(),
            ]),
            Line::from(vec![
                "q".fg(self.colors.normal.magenta),
                "           - quits the application".into(),
            ]),
        ];
        Paragraph::new(lines).wrap(Wrap { trim: true }).block(
            Block::default()
                .title("Help")
                .title_style(Style::default().fg(self.colors.normal.white.into()))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(self.colors.bright.black.into()))
                .padding(Padding::new(2, 2, 1, 1))
                .bg(self.colors.normal.black.into()),
        )
    }

    fn build_filter_input(&self) -> Line<'_> {
        Line::from(format!("/{}", self.filter))
    }
}

impl Component for Dashboard<'_> {
    fn draw(&mut self, frame: &mut Frame, _: Rect) -> anyhow::Result<()> {
        let title = BigText::builder()
            .pixel_size(PixelSize::Quadrant)
            .style(Style::default().fg(self.colors.normal.magenta.into()))
            .lines(vec![" Select a collection".into()])
            .build()?;

        frame.render_widget(title, self.layout.title_pane);
        frame.render_stateful_widget(
            self.list.clone(),
            self.layout.schemas_pane,
            &mut self.list_state,
        );

        if self.show_error_popup {
            let popup = ErrorPopup::new(self.error_message.clone(), self.colors);
            popup.render(self.layout.error_popup, frame.buffer_mut());
        }

        if self.pane_focus.eq(&PaneFocus::Form) {
            let form = NewCollectionForm::new(self.colors);
            form.render(
                self.layout.form_popup,
                frame.buffer_mut(),
                &mut self.form_state,
            );
        }

        if self.show_filter {
            let filter_input = self.build_filter_input();
            frame.render_widget(filter_input, self.layout.help_pane);
        } else {
            let hint_text = self.build_hint_text();
            frame.render_widget(hint_text, self.layout.help_pane);
        }

        if self.show_list_keymaps {
            Clear.render(self.layout.help_popup, frame.buffer_mut());
            let list_keymaps_popup = self.build_help_popup();
            list_keymaps_popup.render(self.layout.help_popup, frame.buffer_mut());
        }

        if self.prompt_delete_current {
            let selected_index = self
                .list_state
                .selected()
                .expect("attempted to open confirm popup without an item selected");
            let selected_item_name = &self
                .schemas
                .get(selected_index)
                .expect("should never be able to have an out of bounds selection")
                .info
                .name;

            let confirm_popup = ConfirmPopup::new(
                format!(
                    "You really want to delete collection {}?",
                    selected_item_name
                ),
                self.colors,
            );
            confirm_popup.render(self.layout.confirm_popup, frame.buffer_mut());
        }

        Ok(())
    }

    fn resize(&mut self, new_size: Rect) {
        self.layout = build_layout(new_size);
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> anyhow::Result<Option<Command>> {
        if self.show_list_keymaps {
            self.show_list_keymaps = false;
            return Ok(None);
        }

        if self.show_filter {
            self.handle_filter_key_event(key_event);
            return Ok(None);
        }

        if self.prompt_delete_current {
            self.handle_confirm_popup_key_event(key_event)?;
            return Ok(None);
        }

        if self.show_error_popup {
            self.handle_error_popup_key_event(key_event)?;
            return Ok(None);
        }

        match self.pane_focus {
            PaneFocus::List => self.handle_list_key_event(key_event),
            PaneFocus::Form => self.handle_form_key_event(key_event),
        }
    }

    fn register_command_handler(&mut self, sender: UnboundedSender<Command>) -> anyhow::Result<()> {
        self.sender = Some(sender.clone());
        Ok(())
    }
}

fn build_layout(size: Rect) -> DashboardLayout {
    let [top, help_pane] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Fill(1), Constraint::Length(1)])
        .areas(size);

    let [_, title_pane, schemas_pane] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(5),
            Constraint::Fill(1),
        ])
        .areas(top);

    let help_popup = Rect::new(size.width / 4, size.height / 2 - 7, size.width / 2, 14);
    let confirm_popup = Rect::new(size.width / 4, size.height / 2 - 4, size.width / 2, 8);
    let form_popup = Rect::new(size.width / 4, size.height / 2 - 7, size.width / 2, 14);
    let error_popup = Rect::new(size.width / 4, size.height / 2 - 10, size.width / 2, 20);

    DashboardLayout {
        schemas_pane,
        help_pane,
        title_pane,
        help_popup,
        confirm_popup,
        form_popup,
        error_popup,
    }
}