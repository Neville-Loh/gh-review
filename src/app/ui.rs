use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::components::help::HelpOverlay;
use crate::components::review_bar::ReviewBar;
use crate::types::DiffMode;

use super::{App, Focus};

impl App {
    pub fn draw(&mut self, frame: &mut Frame) {
        let size = frame.area();

        let completion_h = if self.command_bar.active {
            self.command_bar.completion_height(&self.keymap) + 1
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

        let desc_open = self.description_panel.visible;
        let content_constraints = if desc_open {
            vec![
                Constraint::Length(30),
                Constraint::Min(40),
                Constraint::Percentage(35),
            ]
        } else {
            vec![Constraint::Length(30), Constraint::Min(0)]
        };
        let content_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(content_constraints)
            .split(main_layout[1]);

        // Clear the entire content row to prevent stale artifacts
        // when panels resize (e.g. description drawer closing).
        let content_area = main_layout[1];
        for y in content_area.y..content_area.y + content_area.height {
            for x in content_area.x..content_area.x + content_area.width {
                if let Some(cell) = frame.buffer_mut().cell_mut((x, y)) {
                    cell.reset();
                }
            }
        }

        let diff_area = content_layout[1];
        let diff_height = diff_area.height.saturating_sub(2) as usize;
        self.visible_height = diff_height;
        let panel_inner_width = diff_area.width.saturating_sub(2) as usize;
        let new_wrap_width = match self.diff_view.mode {
            DiffMode::SideBySide => panel_inner_width / 2,
            DiffMode::Unified => panel_inner_width,
        };
        if new_wrap_width != self.diff_view.wrap_width {
            self.diff_view.wrap_width = new_wrap_width;
            self.rebuild_display();
        }
        self.diff_view.ensure_visible(diff_height);

        self.file_picker.draw(
            content_layout[0],
            frame.buffer_mut(),
            self.focus == Focus::FilePicker,
        );

        self.diff_view
            .draw(diff_area, frame.buffer_mut(), self.focus == Focus::DiffView);

        if desc_open {
            self.description_panel.draw(
                content_layout[2],
                frame.buffer_mut(),
                self.focus == Focus::Description,
                &self.stack,
            );
        }

        if self.command_bar.active {
            self.command_bar
                .draw_completions(main_layout[2], frame.buffer_mut(), &self.keymap);
            self.command_bar
                .draw_input(main_layout[3], frame.buffer_mut(), &self.keymap);
        } else if self.search_bar.active {
            let (curr, total) = self.diff_view.search.match_info();
            self.search_bar
                .draw(main_layout[3], frame.buffer_mut(), curr, total);
        } else {
            self.status.tick();
            ReviewBar::draw(
                main_layout[3],
                frame.buffer_mut(),
                self.diff_view.current_context(),
                self.pending_comments.len(),
                &self.status,
                &self.keymap,
                self.focus,
                !self.stack.is_empty(),
                self.ai_available,
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
            let custom_help = self.keymap.custom_action_help();
            HelpOverlay::draw(
                size,
                frame.buffer_mut(),
                &self.keymap,
                &custom_help,
                !self.stack.is_empty(),
            );
        }
    }

    fn draw_title(&self, frame: &mut Frame, area: Rect) {
        use ratatui::style::Color;

        let spans = if let Some(ref meta) = self.pr_meta {
            let status =
                self.stack
                    .status(self.pr_number)
                    .unwrap_or(crate::stack::PrStatus::from_metadata(
                        &meta.state,
                        meta.draft,
                        meta.review_decision.as_deref(),
                    ));
            let mut s = vec![
                Span::styled(" ", crate::theme::Theme::title()),
                Span::styled(
                    format!("{} ", status.icon()),
                    ratatui::style::Style::default().fg(status.color()),
                ),
                Span::styled(
                    format!("{} #{} — {} ", self.repo, meta.number, meta.title),
                    crate::theme::Theme::title(),
                ),
            ];
            if let Some(additions) = meta.additions {
                s.push(Span::styled(
                    format!("+{additions}"),
                    ratatui::style::Style::default().fg(Color::Green),
                ));
            }
            if let Some(deletions) = meta.deletions {
                s.push(Span::styled(
                    format!("/-{deletions}"),
                    ratatui::style::Style::default().fg(Color::Red),
                ));
            }
            s
        } else if self.loading {
            vec![Span::styled(
                format!(" {} #{} — Loading...", self.repo, self.pr_number),
                crate::theme::Theme::title(),
            )]
        } else {
            vec![Span::styled(
                format!(" {} #{}", self.repo, self.pr_number),
                crate::theme::Theme::title(),
            )]
        };

        let line = Line::from(spans);
        let bar = Paragraph::new(line).style(crate::theme::Theme::review_bar());
        Widget::render(bar, area, frame.buffer_mut());
    }
}
