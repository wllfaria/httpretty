use ratatui::{
    style::Stylize,
    text::{Line, Span},
};
use reqtui::syntax::highlighter::HIGHLIGHTER;
use tree_sitter::Tree;

fn is_endline(c: char) -> bool {
    matches!(c, '\n')
}

pub fn build_styled_content(
    content: &str,
    tree: Option<&Tree>,
    colors: &colors::Colors,
) -> Vec<Line<'static>> {
    let mut highlights = HIGHLIGHTER
        .read()
        .unwrap()
        .apply(content, tree, &colors.tokens);

    let mut styled_lines: Vec<Line> = vec![];
    let mut current_line: Vec<Span> = vec![];
    let mut current_token = String::default();
    let mut current_capture = highlights.pop_front();

    for (i, c) in content.chars().enumerate() {
        if let Some(ref capture) = current_capture {
            if i == capture.start && current_token.is_empty() {
                current_token.push(c);
                continue;
            }
            if i == capture.start && !current_token.is_empty() {
                current_line.push(Span::from(current_token.clone()).fg(colors.normal.white));
                current_token.clear();
                current_token.push(c);
                continue;
            }
            if i == capture.end && is_endline(c) {
                current_line.push(Span::styled(current_token.clone(), capture.style));
                styled_lines.push(current_line.clone().into());

                current_token.clear();
                current_line.clear();
                current_capture = highlights.pop_front();
                continue;
            }

            if i == capture.end {
                current_line.push(Span::styled(current_token.clone(), capture.style));
                current_token.clear();
                current_token.push(c);
                current_capture = highlights.pop_front();
                continue;
            }

            if is_endline(c) {
                current_line.push(Span::styled(current_token.clone(), capture.style));
                styled_lines.push(current_line.clone().into());

                current_token.clear();
                current_line.clear();
                continue;
            }

            current_token.push(c);
            continue;
        }

        if !current_token.is_empty() && !is_endline(c) {
            current_line.push(Span::from(current_token.clone()).fg(colors.normal.white));
            current_token.clear();
            current_token.push(c);
            continue;
        }

        if is_endline(c) {
            current_line.push(Span::from(current_token.clone()).fg(colors.normal.white));
            styled_lines.push(current_line.clone().into());

            current_token.clear();
            current_line.clear();
            continue;
        }

        current_token.push(c);
    }

    current_line.push(current_token.clone().into());
    styled_lines.push(current_line.clone().into());

    styled_lines
}
