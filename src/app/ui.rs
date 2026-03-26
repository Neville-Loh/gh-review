use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::components::help::HelpOverlay;
use crate::components::review_bar::ReviewBar;

use super::{App, Focus};

impl App {
    pub fn draw(&mut self, frame: &mut Frame) {
        let size = frame.area();

        let completion_h = if self.command_bar.active {
            self.command_bar.completion_height() + 1
        } else {
            0
        };

        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(completion_h),
                Constraint::Length(1),
            ])
            .split(size);

        self.draw_title(frame, main_layout[0]);

        let content_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(30),
                Constraint::Min(0),
            ])
            .split(main_layout[1]);

        let diff_height = content_layout[1].height.saturating_sub(2) as usize;
        self.visible_height = diff_height;
        self.diff_view.ensure_visible(diff_height);

        self.file_picker.draw(
            content_layout[0],
            frame.buffer_mut(),
            self.focus == Focus::FilePicker,
        );

        self.diff_view.draw(
            content_layout[1],
            frame.buffer_mut(),
            self.focus == Focus::DiffView,
        );

        if self.command_bar.active {
            self.command_bar
                .draw_completions(main_layout[2], frame.buffer_mut());
            self.command_bar
                .draw_input(main_layout[3], frame.buffer_mut());
        } else if self.search_bar.active {
            let (curr, total) = self.diff_view.search.match_info();
            self.search_bar
                .draw(main_layout[3], frame.buffer_mut(), curr, total);
        } else {
            ReviewBar::draw(
                main_layout[3],
                frame.buffer_mut(),
                self.pending_comments.len(),
                &self.status_msg,
                self.status_is_error,
            );
        }

        if self.comment_input.visible {
            self.comment_input
                .draw(content_layout[1], frame.buffer_mut());
        }

        if self.review_confirm.visible {
            self.review_confirm.draw(size, frame.buffer_mut());
        }

        if self.show_help {
            HelpOverlay::draw(size, frame.buffer_mut());
        }
    }

    fn draw_title(&self, frame: &mut Frame, area: Rect) {
        let title = if let Some(ref meta) = self.pr_meta {
            format!(
                " {} #{} — {} ({}→{})",
                self.repo, meta.number, meta.title, meta.base.ref_name, meta.head.ref_name
            )
        } else if self.loading {
            format!(" {} #{} — Loading...", self.repo, self.pr_number)
        } else {
            format!(" {} #{}", self.repo, self.pr_number)
        };

        let line = Line::from(vec![Span::styled(title, crate::theme::Theme::title())]);
        let bar = Paragraph::new(line).style(crate::theme::Theme::review_bar());
        Widget::render(bar, area, frame.buffer_mut());
    }
}
