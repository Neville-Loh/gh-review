use ratatui::{
    style::Style,
    text::{Line, Span},
};
use unicode_width::UnicodeWidthStr;

/// Word-wrap a [`Line`] of styled [`Span`]s into multiple lines, preserving
/// each span's [`Style`].
///
/// Breaks at word boundaries (spaces) when the accumulated display width
/// exceeds `max_width`. A single word wider than `max_width` is never split
/// mid-word; it simply extends the line.
pub fn wrap_spans(spans: &[Span<'static>], max_width: usize) -> Vec<Line<'static>> {
    if max_width == 0 {
        return vec![Line::from(spans.to_vec())];
    }

    let mut result_lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut current_width: usize = 0;
    let mut span_buf = String::new();
    let mut span_style = Style::default();
    let mut first = true;

    for span in spans {
        if first {
            span_style = span.style;
            first = false;
        }

        for word in span.content.split_inclusive(' ') {
            let word_w = word.width();

            if current_width + word_w > max_width && current_width > 0 {
                flush(&mut span_buf, span_style, &mut current_spans);
                result_lines.push(Line::from(std::mem::take(&mut current_spans)));
                current_width = 0;
            }

            if span.style != span_style {
                flush(&mut span_buf, span_style, &mut current_spans);
                span_style = span.style;
            }

            span_buf.push_str(word);
            current_width += word_w;
        }
    }

    flush(&mut span_buf, span_style, &mut current_spans);
    if !current_spans.is_empty() {
        result_lines.push(Line::from(current_spans));
    }
    if result_lines.is_empty() {
        result_lines.push(Line::default());
    }
    result_lines
}

fn flush(buf: &mut String, style: Style, out: &mut Vec<Span<'static>>) {
    if !buf.is_empty() {
        out.push(Span::styled(std::mem::take(buf), style));
    }
}
